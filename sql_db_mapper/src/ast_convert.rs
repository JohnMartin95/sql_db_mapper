//! Turn the AST of the database from sql_tree into a Rust syntax tree fron syn

use super::{
	sql_tree::*,
	Opt,
	format_rust,
};
use quote::{
	quote,
	format_ident,
};
use proc_macro2::{
	TokenStream,
};
use heck::*;

mod sql_types;
mod sql_procs;

enum Case {
	SnakeCase,
	CamelCase,
	ShoutySnake,
}
use Case::*;
/// Do capitalization
fn format_heck(name : &str, opt:&Opt, case : Case) -> proc_macro2::Ident {
	if opt.formatted {
		match case {
			SnakeCase   => format_ident!("{}", name.to_snake_case()),
			CamelCase   => format_ident!("{}", name.to_camel_case()),
			ShoutySnake => format_ident!("{}", name.to_shouty_snake_case()),
		}
	} else {
		format_ident!("{}", name)
	}
}

/// Takes a reference to a struct and the program options and returns a new type
///
/// Used to turn the types in sql_tree into a Rust syntax tree
pub trait ConvertToAst {
	fn to_rust_tokens(&self, opt : &Opt) -> TokenStream;

	fn as_string(&self, opt : &Opt) -> String  {
		self.to_rust_tokens(opt).to_string()
	}

	fn maybe_formatted(&self, opt : &Opt) ->  String {
		let output = self.as_string(opt);
		if opt.ugly {
			output
		} else {
			format_rust(&output)
		}
	}
}

impl ConvertToAst for FullDB {

	/// Turn a full database structure into a single rust file mapping all it's types and functions
	///
	/// ```ignore
	/// #![allow(non_snake_case)]
	/// #![allow(unused_imports)]
	/// #![allow(non_camel_case_types)]
	/// pub use sql_db_mapper::helper_types::{
	/// 	orm,
	/// 	exports::*
	/// };
	/// use orm::*;
	///
	/// //code for each schema here
	/// ```
	fn to_rust_tokens(&self, opt : &Opt)-> TokenStream {
		let schemas = self.schemas.iter().map(|v| {
			let name = format_heck(&v.name, opt, SnakeCase);
			if opt.dir {
				quote!{
					pub mod #name;
				}
			} else {
				let schema_def = v.to_rust_tokens(opt);
				quote!{
					pub mod #name {
						#schema_def
					}
				}
			}
		});
		//allows if case isn't fixed
		let fixed_case = if opt.formatted {
			quote!{}
		} else {
			quote!{
				#![allow(non_snake_case)]
				#![allow(non_camel_case_types)]
			}
		};
		//uses that depend on if the code is sync
		let client_use = if opt.sync {
			quote!{
				pub use postgres::{
					Client,
					Error as SqlError,
				};
				use postgres::row::Row;
			}
		} else {
			quote!{
				pub use tokio_postgres::{
					Client,
					Error as SqlError,
				};
				use tokio_postgres::row::Row;
				pub use std::future::Future;
			}
		};
		//serde use
		let serde_use = if opt.serde {
			quote!{
				use serde::{
					Serialize,
					Deserialize,
				};
			}
		} else {
			quote!{}
		};
		let doc_str = format!("Generated by sql_db_mapper version={}", super::VERSION );
		let call_params = format!("Called with arguments `{}`", opt.get_call_string());

		quote!{
			#![doc = #doc_str]
			#![doc = ""]
			#![doc = #call_params]
			#![allow(unused_imports)]
			#fixed_case
			pub use sql_db_mapper_core as orm;
			use chrono;
			use postgres_derive::*;
			#client_use
			#serde_use
			use orm::*;
			#(#schemas)*
		}
	}
}
impl FullDB {
	//writes the output text to either
	pub fn make_output(&self, opt : &Opt) {
		use std::{
			fs::File,
			io::Write
		};
		let toml_content = opt.get_cargo_toml();
		if let Some(output_file) = &opt.output {
			let mut output_file = output_file.clone();
			if opt.dir {
				//create crate directory
				std::fs::create_dir_all(&output_file).unwrap();

				//generate Cargo.toml
				let mut toml_path = output_file.clone();
				toml_path.push("Cargo.toml");
				let mut cargo_toml = File::create(toml_path).unwrap();
				cargo_toml.write_all(toml_content.as_bytes()).expect("failed to write to file");

				//generate src directory
				output_file.push("src/");
				std::fs::create_dir_all(&output_file).unwrap();

				//generate lib.rs file
				let mut lib_rs = output_file.clone();
				lib_rs.push("lib.rs");
				let mut lib_rs = File::create(lib_rs).unwrap();
				lib_rs.write_all(self.maybe_formatted(&opt).as_bytes()).expect("failed to write to file");

				// make file for each schema's module
				for schema in &self.schemas {
					let mut schema_rs = output_file.clone();
					schema_rs.push(format!("{}.rs", schema.name));
					let mut schema_rs = File::create(schema_rs).unwrap();
					schema_rs.write_all(schema.maybe_formatted(&opt).as_bytes()).expect("failed to write to file");
				}
			} else {
				println!("{}\n", toml_content);
				let mut f = File::create(output_file).unwrap();
				f.write_all(self.maybe_formatted(&opt).as_bytes()).expect("failed to write to file");
			}
		} else {
			println!("{}\n", toml_content);
			println!("{}", self.maybe_formatted(&opt));
		}
	}
}

impl ConvertToAst for Schema {

	/// Renders a schema as a rust module
	///
	/// ```ignore
	/// mod #schema_name {
	/// 	use super::*;
	///
	/// 	//code for all tables and other types
	///
	/// 	//code for each procedures/function
	/// }
	/// ```
	fn to_rust_tokens(&self, opt : &Opt)-> TokenStream {
		let type_defs = self.types.iter().map(|v| sql_types::type_to_rust(&v, opt));
		let proc_defs = self.procs.iter().map(|v| sql_procs::proc_to_rust(&v, opt));
		quote!{
			use super::*;
			#(#type_defs)*
			#(#proc_defs)*
		}
	}
}

impl FullType {
	fn to_tokens(&self, opt:&Opt) -> TokenStream {
		let typ    = format_heck(&self.name, opt, CamelCase);
		let schema = format_heck(&self.schema, opt, SnakeCase);
		quote!{ super::#schema::#typ }
	}
}


// trait ToFuncParams {
// 	fn as_function_params(&self, opt:&Opt) -> TokenStream;
// }
impl NamesAndTypes {
	fn as_function_params(&self, opt:&Opt) -> TokenStream {
		self.0.iter().map(|tan| {
			let name = format_heck(&tan.name, opt, SnakeCase);
			let typ  = tan.typ.to_tokens(opt);
			quote!{ #name : &#typ, }
		}).collect()
	}
}
