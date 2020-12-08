#![forbid(unsafe_code)]
//! Connects to a PostgreSQL database and creates a rust module representing all the schemas complete with mappings for stored functions/procedures

pub mod ast_convert;
pub mod connection;
mod pg_select_types;
mod sql_tree;

pub const VERSION: &str = std::env!("CARGO_PKG_VERSION");

use postgres::{Client, NoTls};
use std::path::PathBuf;
use structopt::StructOpt;

/// The program options for the code generation
#[derive(Debug, StructOpt)]
#[structopt(
	name = "sql_db_mapper",
	about = "Generate a rust wrapper for a PostgreSQL database",
	version = VERSION
)]
pub struct Opt {
	/// Activate debug mode
	#[structopt(short, long)]
	pub debug: bool,

	/// Skip running output through rustfmt
	#[structopt(short, long)]
	pub ugly: bool,

	/// Program will treat output as a directory name rather than a file and generate a whole crate. If output is not provided code is printed as usual
	#[structopt(long)]
	pub dir: bool,

	/// Convert names from the database to rust standard (i.e. table names in CamelCase, fields and functions in snake_case)
	#[structopt(long)]
	pub rust_case: bool,

	/// string passed to rustfmt --config
	#[structopt(long)]
	pub rustfmt_config: Option<String>,

	/// string passed to rustfmt --config-path
	#[structopt(long)]
	pub rustfmt_config_path: Option<String>,

	/// Only make mappings for tables and views
	#[structopt(long)]
	pub no_functions: bool,

	/// How to use tuples (used by default for just overloads). Options:
	/// overloads (the default, use tuples to represent function overloading).
	/// all (Have all functions take a tuple for consitency).
	/// none (skip mapping overloaded procs at all).
	/// one_overload (avoid tuples by only mapping the oldest sql proc in the database).
	#[structopt(long, default_value = "overloads")]
	pub use_tuples: Tuples,

	/// String to connect to database, see tokio_postgres::Config for details.
	/// If not provided envirment variable SQL_MAP_CONN is checked instead
	#[structopt(long, env = "SQL_MAP_CONN")]
	pub conn: String,

	/// Output file, stdout if not present
	#[structopt(parse(from_os_str))]
	pub output: Option<PathBuf>,
}

#[derive(Debug, StructOpt, Clone, Copy, PartialEq, Eq)]
pub enum Tuples {
	/// use tuples to represent function overloading
	ForOverloads,
	/// Have all functions take a tuple for consitency
	ForAll,
	/// skip mapping overloaded procs at all
	NoOverloads,
	/// avoid tuples by only mapping the oldest sql proc in the database
	OldestOverload,
}
impl std::str::FromStr for Tuples {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Tuples, &'static str> {
		match s {
			"overloads" => Ok(Tuples::ForOverloads),
			"all" => Ok(Tuples::ForAll),
			"none" => Ok(Tuples::NoOverloads),
			"one_overload" => Ok(Tuples::OldestOverload),
			_ => Err("Invalid tuple handling option, use one of (overloads, all, none, one_overload)"),
		}
	}
}
impl Tuples {
	fn to_str(&self) -> &'static str {
		match self {
			Tuples::ForOverloads => "overloads",
			Tuples::ForAll => "all",
			Tuples::NoOverloads => "none",
			Tuples::OldestOverload => "one_overload",
		}
	}
}

impl Opt {
	/// Produce the Cargo.toml file contents (the dependecies of the generated code)
	pub fn get_cargo_toml(&self) -> String {
		let package_name = self
			.output
			.as_ref()
			.map(|v| v.file_name())
			.flatten()
			.map(|v| v.to_str())
			.flatten()
			.unwrap_or("my_db_mapping");

		let mut dependencies = format!("[package]\nname = \"{}\"", package_name);
		dependencies += r#"
version = "0.1.0"
edition = "2018"

[dependencies]
sql_db_mapper_core = "0.0.4"
postgres-types = { version = "0.1", features = ["derive", "with-chrono-0_4"] }
chrono = "0.4"
#version 1.6 of rust_decimal isn't compiling
rust_decimal = { version = ">= 1.2, < 1.5", features = ["postgres"] }
postgres-derive = "0.4"

postgres  = { version = "0.17", optional = true }
tokio-postgres = { version = "0.5.1", optional = true }
async-trait = { version = "0.1.22", optional = true }

serde = { version = "1.0", features = ["derive"], optional = true }

[features]
with_serde = ["serde", "sql_db_mapper_core/with_serde"]
sync = ["postgres"]
async = ["tokio-postgres", "async-trait"]
"#;

		dependencies
	}

	/// Build a call string that could be used to get the same options
	pub fn get_call_string(&self) -> String {
		// let sync  =  if self.sync  { " -s" } else { "" };
		let ugly = if self.ugly { " -u" } else { "" };
		// let serde =  if self.serde { " --serde" } else { "" };
		let dir = if self.dir { " --dir" } else { "" };
		let rust_case = if self.rust_case { " --rust_case" } else { "" };
		let use_tuples = if self.use_tuples == Tuples::ForOverloads {
			String::new()
		} else {
			format!(" --use-tuples {}", self.use_tuples.to_str())
		};
		format!(
			"sql_db_mapper{ugly}{dir}{rust_case}{use_tuples}",
			// sync = sync,
			ugly = ugly,
			// serde = serde,
			dir = dir,
			rust_case = rust_case,
			use_tuples = use_tuples,
		)
	}

	pub fn get_client(&self) -> connection::MyClient {
		let client = Client::connect(&self.conn, NoTls)
			.expect("Failed to connect to database, please check your connection string and try again");

		connection::MyClient::new(client)
	}
}

/// Calls rustfmt (the program) on the input
///
/// On any rustfmt error stderr is written to and a copy of the input is returned
///
/// Can panic if acquiring/writing to stdin fails or the the text written to stdout or stderr by rustfmt is not valid utf8
pub fn format_rust(value: &str, rustfmt_config: Option<&str>, rustfmt_config_path: Option<&str>) -> String {
	use std::{
		io::Write,
		process::{Command, Stdio},
	};
	let mut args = Vec::new();
	if let Some(s) = rustfmt_config {
		args.push("--config");
		args.push(s);
	}
	if let Some(s) = rustfmt_config_path {
		args.push("--config-path");
		args.push(s);
	}
	if let Ok(mut proc) = Command::new("rustfmt")
		.arg("--emit=stdout")
		.arg("--edition=2018")
		.args(&args)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
	{
		{
			let stdin = proc.stdin.as_mut().unwrap();
			stdin.write_all(value.as_bytes()).unwrap();
		}
		match proc.wait_with_output() {
			Ok(output) => {
				if !output.stderr.is_empty() {
					eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());
				}
				if output.status.success() {
					return std::str::from_utf8(&output.stdout).unwrap().to_owned().into();
				} else {
					eprintln!("{:?}", output.status.code());
					eprintln!("{}", std::str::from_utf8(&output.stdout).unwrap());
				}
			},
			Err(e) => {
				eprintln!("Error running rustfmt: {}", e);
			},
		}
	} else {
		eprintln!("failed to spawn rustfmt")
	}
	value.to_string()
}
