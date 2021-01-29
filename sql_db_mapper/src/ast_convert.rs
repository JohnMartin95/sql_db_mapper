//! Turn the AST of the database from sql_tree into a Rust syntax tree fron syn

use super::{format_rust, sql_tree::*, Opt};
use heck::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{fs::File, io::Write};

mod sql_procs;
mod sql_types;

/// Type of capitalization to do with heck
enum Case {
	SnakeCase,
	CamelCase,
	ShoutySnake,
}
use Case::*;
/// Optionally do capitalization
fn format_heck(name: &str, opt: &Opt, case: Case) -> proc_macro2::Ident {
	if opt.rust_case {
		match case {
			SnakeCase => format_ident_h(&name.to_snake_case()),
			CamelCase => format_ident_h(&name.to_camel_case()),
			ShoutySnake => format_ident_h(&name.to_shouty_snake_case()),
		}
	} else {
		format_ident_h(name)
	}
}
fn format_ident_h(s: &str) -> proc_macro2::Ident {
	if let Some(c) = s.chars().next() {
		if c.is_ascii_digit() {
			format_ident!("_{}", s)
		} else {
			format_ident!("{}", s)
		}
	} else {
		panic!("formatted identifier should not be empty")
	}
}

/// Optionally Format the tokens with rustfmt
fn maybe_format(input: &TokenStream, opt: &Opt) -> String {
	let output = input.to_string();
	if opt.ugly {
		output
	} else {
		format_rust(
			&output,
			opt.rustfmt_config.as_deref(),
			opt.rustfmt_config_path.as_deref(),
		)
	}
}

impl FullDB {
	//writes the output text to either a file, directory, or stdout
	pub fn make_output(&self, opt: &Opt) {
		let toml_content = opt.get_cargo_toml();
		if let Some(output_file) = &opt.output {
			let output_file = output_file.clone();
			if opt.dir {
				self.make_full_crate(opt, toml_content, output_file);
			} else {
				println!("{}\n", toml_content);
				let mut f = File::create(output_file).unwrap();
				let lib_rs_content = maybe_format(&self.to_rust_tokens(opt), &opt);
				f.write_all(lib_rs_content.as_bytes()).expect("failed to write to file");
			}
		} else {
			println!("{}\n", toml_content);
			let lib_rs_content = maybe_format(&self.to_rust_tokens(opt), &opt);
			println!("{}", lib_rs_content);
		}
	}

	/// Makes a full crate for the mapping into a directory
	fn make_full_crate(&self, opt: &Opt, toml_content: String, mut output_file: std::path::PathBuf) {
		//create crate directory
		std::fs::create_dir_all(&output_file).unwrap();

		//generate Cargo.toml
		let toml_path = path_push_helper(&output_file, "Cargo.toml");
		File::create(toml_path)
			.expect("failed to create Cargo.toml")
			.write_all(toml_content.as_bytes())
			.expect("failed to write to file");

		//generate src directory
		output_file.push("src/");
		std::fs::create_dir_all(&output_file).unwrap();

		//generate lib.rs file
		let lib_rs = path_push_helper(&output_file, "lib.rs");
		File::create(lib_rs)
			.unwrap()
			.write_all(
				maybe_format(&self.to_rust_tokens(opt), &opt).as_bytes(), // maybe_format(&self.to_rust_tokens(opt), &opt).as_bytes()
			)
			.expect("failed to write to file");

		//generate types.rs file and types folder
		let types_rs = path_push_helper(&output_file, "types.rs");
		File::create(types_rs)
			.unwrap()
			.write_all(maybe_format(&self.types_content(opt), &opt).as_bytes())
			.expect("failed to write to file");
		let types_folder = path_push_helper(&output_file, "types/");
		std::fs::create_dir_all(&types_folder).unwrap();

		let sync_folder = path_push_helper(&output_file, "sync_fns/");
		let async_folder = path_push_helper(&output_file, "async_fns/");

		//generate sync/async files and folders
		if !self.no_procs() {
			//generate sync_fns.rs file/folder
			let sync_rs = path_push_helper(&output_file, "sync_fns.rs");
			File::create(sync_rs)
				.unwrap()
				.write_all(maybe_format(&self.sync_content(opt), &opt).as_bytes())
				.expect("failed to write to file");
			std::fs::create_dir_all(&sync_folder).unwrap();

			//generate async_fns.rs file/folder
			let async_rs = path_push_helper(&output_file, "async_fns.rs");
			File::create(async_rs)
				.unwrap()
				.write_all(maybe_format(&self.async_content(opt), &opt).as_bytes())
				.expect("failed to write to file");
			std::fs::create_dir_all(&async_folder).unwrap();
		}


		// make file for each schema's module
		for schema in &self.schemas {
			let file_name = format!("{}.rs", schema.name);

			if !schema.no_types() {
				let schema_t = path_push_helper(&types_folder, &file_name);
				File::create(schema_t)
					.unwrap()
					.write_all(maybe_format(&schema.types_content(opt), &opt).as_bytes())
					.expect("failed to write to file");
			}

			if !schema.no_procs() {
				let schema_s = path_push_helper(&sync_folder, &file_name);
				let schema_a = path_push_helper(&async_folder, &file_name);

				File::create(schema_s)
					.unwrap()
					.write_all(maybe_format(&schema.funcs_content(opt, true), &opt).as_bytes())
					.expect("failed to write to file");

				File::create(schema_a)
					.unwrap()
					.write_all(maybe_format(&schema.funcs_content(opt, false), &opt).as_bytes())
					.expect("failed to write to file");
			}
		}
	}

	/// Get the rust tokens for the top level of the mapping (it changes depending on whether the dir option is used)
	fn to_rust_tokens(&self, opt: &Opt) -> TokenStream {
		if opt.dir {
			self.to_dir_tokens(opt)
		} else {
			self.to_flat_tokens(opt)
		}
	}

	/// builds the contents of the types module
	pub fn types_content(&self, opt: &Opt) -> TokenStream {
		let schemas = self.schemas.iter().map(|v| v.get_types_module(opt));

		quote! {
			use serde::{
				Serialize,
				Deserialize,
			};
			use postgres_types::{
				FromSql,
				ToSql,
			};
			use super::orm::*;

			#(#schemas)*
		}
	}

	/// builds the contents of the sync_fns module
	pub fn sync_content(&self, opt: &Opt) -> TokenStream {
		let schemas = self.schemas.iter().map(|v| v.get_funcs_module(opt, true));

		quote! {
			pub use super::orm::{
				SyncClient as Client,
				SqlError,
			};
			use sql_db_mapper_core::TryFromRow;

			#(#schemas)*
		}
	}

	/// builds the contents of the async_fns module
	pub fn async_content(&self, opt: &Opt) -> TokenStream {
		let schemas = self.schemas.iter().map(|v| v.get_funcs_module(opt, false));

		quote! {
			pub use super::orm::{
				AsyncClient as Client,
				SqlError,
			};
			use sql_db_mapper_core::TryFromRow;
			pub use std::future::Future;

			#(#schemas)*
		}
	}

	/// The tokens for FullDb when the whole mapping is being made into one file
	fn to_flat_tokens(&self, opt: &Opt) -> TokenStream {
		let opt_tokens = crate_root_start(opt);

		let types_tokens = self.types_content(opt);
		if self.no_procs() {
			quote! {
				#opt_tokens

				pub mod types{ use super::*; #types_tokens }
			}
		} else {
			let sync_tokens = self.sync_content(opt);
			let async_tokens = self.async_content(opt);

			quote! {
				#opt_tokens

				pub mod types{ use super::*; #types_tokens }
				#[cfg(feature = "sync")]
				pub mod sync_fns{ use super::*; #sync_tokens }
				#[cfg(feature = "async")]
				pub mod async_fns{ use super::*; #async_tokens }
			}
		}
	}

	/// The tokens for FullDb when a directory structure is being created
	fn to_dir_tokens(&self, opt: &Opt) -> TokenStream {
		let opt_tokens = crate_root_start(opt);
		if self.no_procs() {
			quote! {
				#opt_tokens

				pub mod types;
			}
		} else {
			quote! {
				#opt_tokens

				pub mod types;
				#[cfg(feature = "sync")]
				pub mod sync_fns;
				#[cfg(feature = "async")]
				pub mod async_fns;
			}
		}
	}
}
fn path_push_helper(path: &std::path::PathBuf, extention: &str) -> std::path::PathBuf {
	let mut p = path.clone();
	p.push(extention);
	p
}

impl Schema {
	///gets the content for this schema as it would appears in the `types` module
	fn get_types_module(&self, opt: &Opt) -> TokenStream {
		let name = format_heck(&self.name, opt, SnakeCase);
		if self.no_types() {
			quote! {}
		} else if opt.dir {
			quote! { pub mod #name; }
		} else {
			let content = self.types_content(opt);
			quote! {
				pub mod #name {
					#content
				}
			}
		}
	}

	fn types_content(&self, opt: &Opt) -> TokenStream {
		let type_defs = self.types.iter().map(|v| sql_types::type_to_rust(&v, opt));
		quote! {
			use super::*;
			#(#type_defs)*
		}
	}

	///gets the content for this schema as it would appears in the `sync_fns` and `async_fns` module
	fn get_funcs_module(&self, opt: &Opt, is_sync: bool) -> TokenStream {
		let name = format_heck(&self.name, opt, SnakeCase);
		if self.no_procs() {
			quote! {}
		} else if opt.dir {
			quote! { pub mod #name; }
		} else {
			let content = self.funcs_content(opt, is_sync);
			quote! {
				pub mod #name {
					#content
				}
			}
		}
	}

	fn funcs_content(&self, opt: &Opt, is_sync: bool) -> TokenStream {
		let proc_defs = self.procs.iter().map(|v| sql_procs::proc_to_rust(&v, opt, is_sync));
		quote! {
			use super::*;
			#(#proc_defs)*
		}
	}
}

impl FullType {
	fn to_tokens(&self, opt: &Opt) -> TokenStream {
		let schema = format_heck(&self.schema, opt, SnakeCase);
		let typ = format_heck(&self.name, opt, CamelCase);
		quote! { crate::types::#schema::#typ }
	}
}


impl NamesAndTypes {
	fn as_function_params(&self, opt: &Opt) -> TokenStream {
		self.0
			.iter()
			.map(|tan| {
				let name = format_heck(&tan.name, opt, SnakeCase);
				let typ = tan.typ.to_tokens(opt);
				quote! { #name : &#typ, }
			})
			.collect()
	}
}

/// Get the tokens that go at the top of the mapping, some uses, docs, and attributes
fn crate_root_start(opt: &Opt) -> TokenStream {
	//allows if case isn't fixed
	let fixed_case = if opt.rust_case {
		quote! {}
	} else {
		quote! {
			#![allow(non_snake_case)]
			#![allow(non_camel_case_types)]
		}
	};

	let doc_str = format!("Generated by sql_db_mapper version={}", super::VERSION);
	let call_params = format!("Called with arguments `{}`", opt.get_call_string());

	quote! {
		#![doc = #doc_str]
		#![doc = ""]
		#![doc = #call_params]
		#![allow(unused_imports)]
		#fixed_case
		pub use sql_db_mapper_core as orm;
		use orm::*;
	}
}
