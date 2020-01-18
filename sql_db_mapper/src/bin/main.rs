use structopt::StructOpt;
use postgres::{Client, NoTls};
use std::{
	fs::File,
	io::Write
};
use sql_db_mapper::{
	format_rust,
	connection::*,
	Opt,
};

use sql_db_mapper::ast_convert::*;

fn main() {
	let opt = Opt::from_args();

	let conn = Client::connect(&opt.conn_string, NoTls).expect("Failed to connect to database, please check your connection string and try again");

	let mut dependencies = String::from("[dependencies]\npostgres-types = \"0.1\"\n");

	if opt.sync {
		if opt.serde {
			dependencies += "sql_db_mapper_core = { version = \"0.0.2\", features=[\"sync\", \"with_serde\"] }\n";
		} else {
			dependencies += "sql_db_mapper_core = { version = \"0.0.2\", features=[\"sync\"] }\n";
		}
	} else {
		dependencies += "async-trait = \"0.1.22\"\n";
		if opt.serde {
			dependencies += "sql_db_mapper_core = { version = \"0.0.2\", features=[\"with_serde\"] }\n";
		} else {
			dependencies += "sql_db_mapper_core = \"0.0.2\"\n";
		}
	}
	// dependencies +=
	// match (opt.sync, opt.serde) {
	// 	(true,  true ) => "sql_db_mapper_core = { version = \"0.0.2\", features=[\"sync\", \"with_serde\"] }\n",
	// 	(true,  false) => "sql_db_mapper_core = { version = \"0.0.2\", features=[\"sync\"] }\n",
	// 	(false, true ) => "async-trait = \"0.1.22\"\nsql_db_mapper_core = { version = \"0.0.2\", features=[\"with_serde\"] }\n",
	// 	(false, false) => "async-trait = \"0.1.22\"\nsql_db_mapper_core = \"0.0.2\"\n",
	// };
	println!("{}", dependencies);

	let mut conn = MyClient::new(conn);
	let full_db = conn.get_all();

	let code_string = full_db.as_string(&opt);

	let final_str =
		if opt.ugly {
			code_string
		} else {
			format_rust(&code_string)
		};

	if let Some(output_file) = opt.output {
		let f = File::create(output_file);
		match f {
			Ok(mut f) => f.write_all(final_str.as_bytes()).expect("failed to write to file"),
			Err(e) => {
				eprintln!("Error ({}) while opening output file. Writing output to stdout just in case", e);
				println!("{}", final_str);
			}
		}
	} else {
		println!("{}", final_str);
	}
}
