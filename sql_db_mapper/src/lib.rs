//! Connects to a PostgreSQL database and creates a rust module representing all the schemas complete with mappings for stored functions/procedures

mod sql_tree;
pub mod connection;
pub mod ast_convert;

pub const VERSION: &str = std::env!("CARGO_PKG_VERSION");

use structopt::StructOpt;
use std::path::PathBuf;

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

	/// Generate synchronous mapping
	#[structopt(short, long)]
	pub sync: bool,

	/// Skip running output through rustfmt
	#[structopt(short, long)]
	pub ugly: bool,

	/// Include derives for serde on all generated types
	#[structopt(long)]
	pub serde: bool,

	/// Program will treat output as a directory name rather than a file and generate a whole crate. If output is not provided code is printed as usual
	#[structopt(long)]
	pub dir: bool,

	/// How to use tuples (used by default for just overloads). Options:
	/// overloads (the default, use tuples to represent function overloading).
	/// all (Have all functions take a tuple for consitency).
	/// none (skip mapping overloaded procs at all).
	/// one_overload (avoid tuples by only mapping the oldest sql proc in the database).
	#[structopt(long, default_value = "overloads")]
	pub use_tuples : Tuples,

	/// String to connect to database, see tokio_postgres::Config for details
	#[structopt()]
	pub conn_string: String,

	/// Output file, stdout if not present
	#[structopt(parse(from_os_str))]
	pub output: Option<PathBuf>,
}
#[derive(Debug, StructOpt, Clone, Copy)]
pub enum Tuples {
	ForOverloads,
	ForAll,
	NoOverloads,
	OldestOverload,
}
impl std::str::FromStr for Tuples {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Tuples,  &'static str> {
		match s {
			"overloads"    => Ok(Tuples::ForOverloads),
			"all"          => Ok(Tuples::ForAll),
			"none"         => Ok(Tuples::NoOverloads),
			"one_overload" => Ok(Tuples::OldestOverload),
			_ => Err("Invalid tuple handling option"),
		}
	}
}

impl Opt {
	pub fn get_cargo_toml(&self) -> String {
		let package_name =
		if let Some(output_file) = &self.output {
			// let mut output_file = output_file.clone();
			if let Some(name) = output_file.file_name() {
				if let Some(name) = name.to_str() {
					String::from(name)
				} else {
					String::from("my_db_mapping")
				}
			} else {
				String::from("my_db_mapping")
			}
		} else {
			String::from("my_db_mapping")
		};

		let mut dependencies = format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2018\"\n\n[dependencies]\npostgres-types = \"0.1\"\n", package_name);

		if !self.sync {
			dependencies += "async-trait = \"0.1.22\"\n";
		}
		if self.serde {
			dependencies += "serde = { version = \"1.0\", features = [\"derive\"] }\n";
		}

		dependencies +=
		match (self.sync, self.serde) {
			(true,  true ) => "sql_db_mapper_core = { version = \"0.0.2\", features=[\"sync\", \"with_serde\"] }\n",
			(true,  false) => "sql_db_mapper_core = { version = \"0.0.2\", features=[\"sync\"] }\n",
			(false, true ) => "sql_db_mapper_core = { version = \"0.0.2\", features=[\"with_serde\"] }\n",
			(false, false) => "sql_db_mapper_core = \"0.0.2\"\n",
		};
		dependencies
	}
}

/// Calls rustfmt (the program) on the input
///
/// On any rustfmt error stderr is written to and a copy of the input is returned
///
/// Can panics if acquiring/writing to stdin fails or the the text written to stdout or stderr by rustfmt is not valid utf8
pub fn format_rust(value: &str) -> String {
	use std::{
		process::{
			Command,
			Stdio,
		},
		io::Write,
	};
	if let Ok(mut proc) = Command::new("rustfmt").arg("--emit=stdout")
		.arg("--edition=2018")
		.args(&["--config", "fn_single_line=true,hard_tabs=true,imports_layout=Vertical"])
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
					// slice between after the prefix and before the suffix
					// (currently 14 from the start and 2 before the end, respectively)
					return std::str::from_utf8(&output.stdout)
						.unwrap()
						.to_owned()
						.into();
				} else {
					eprintln!("{:?}", output.status.code());
					eprintln!("{}", std::str::from_utf8(&output.stdout).unwrap());
				}
			},
			Err(e) => {
				eprintln!("Error or something: {}", e);
			}
		}
	} else {
		eprintln!("failed to spawn rustfmt")
	}
	value.to_string()
}
