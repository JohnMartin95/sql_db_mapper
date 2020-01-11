//! Connects to a PostgreSQL database and creates a rust module representing the schema complete with mappings for stored functions/procedures

pub mod connection;

pub mod db_model;

mod sql_tree;

#[cfg(feature = "use_ast")]
pub mod ast_convert;


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
