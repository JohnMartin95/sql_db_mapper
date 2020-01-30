//! Turn the AST of the database from sql_tree into a Rust syntax tree fron syn

use super::{
	sql_tree::*,
	Opt,
	Tuples,
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

/// helper trait that does extend but consumes and returns self
trait MyExtend<A> : Extend<A> {
	fn extend2<I: IntoIterator<Item=A>>(self, iter: I) -> Self;
}
impl<T, A: Extend<T>> MyExtend<T> for A {
	fn extend2<I: IntoIterator<Item=T>>(mut self, iter: I) -> Self {
		self.extend(iter);
		self
	}
}
enum Case {
	SnakeCase,
	CamelCase,
	ShoutySnake,
}
use Case::*;
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

		quote!{
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
		let type_defs = self.types.iter().map(|v| v.to_rust_tokens(opt));
		let proc_defs = self.procs.iter().map(|v| v.to_rust_tokens(opt));
		quote!{
			use super::*;
			#(#type_defs)*
			#(#proc_defs)*
		}
	}
}

impl ConvertToAst for PsqlType {
	/// Takes a SQL type and renders its Rust equivalent
	///
	/// All the generated typs include derives for Debug, Clone, FromSql, ToSql, and TryFromRow (which allows straight conversion from the postgres and tokio-postgres Row struct into the type)
	///
	/// ```ignore
	/// //an postgres enum type
	/// #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
	/// pub enum MySqlEnum {
	/// 	Variant1,
	/// 	Variant2,
	/// }
	///
	/// // a composite type; the type of a table, view, or anonymous record returned by a procedure
	/// #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
	/// pub struct MyTable {
	/// 	// super prevents lookup errors when the schecma name is the same as the type name
	/// 	pub field0: super::pg_catalog::varchar,
	/// 	pub field1: super::schema::typ,
	/// 	pub field2: super::pg_catalog::bool,
	/// }
	///
	/// //base types. the only allowed base types are those in pg_catalog of which all are defined by a simple  typedef like below
	/// pub type bytea = Vec<u8>;
	/// pub type int8 = i64;
	///
	/// // a domain type, a simple wrapper on another type
	/// #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
	/// pub struct MyNewType(pub pg_catalog::varchar);
	///
	/// // other types can't be converted at the current moment (if the program is called with the debug flag it will print when it comes across something it skips)
	///
	/// ```
	fn to_rust_tokens(&self, opt : &Opt)-> TokenStream {
		use PsqlTypType::*;
		match &self.typ {
			Enum(e)      => enum_to_ast_helper(e, &self.name, opt),
			Composite(c) => composite_to_ast_helper(c, &self.name, opt),
			Base(b)      => base_to_ast_helper(b, opt),
			Domain(d)    => domain_to_ast_helper(d, &self.name, opt),
			Other        => {
				if self.oid == 2278 {
					let name_type = format_heck(&self.name, opt, CamelCase);
					quote!{ pub type #name_type = (); }
				} else {
					if opt.debug { println!("Couldn't convert type: {}, {}", self.name, self.oid) };
					quote!{  }
				}
			}
		}
	}
}

/// creates the syn node for an enum
fn enum_to_ast_helper(e : &PsqlEnumType, name : &str, opt : &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, CamelCase);

	//the enum definition itself
	let enum_body = e.labels
		.iter()
		.map(|v| {
			format_heck(v, opt, CamelCase)
		});
	let derives = get_derives(opt.serde);

	quote!{
		#derives
		pub enum #name_type {
			#(#enum_body),*
		}
	}
}

/// creates the syn node for a struct
fn composite_to_ast_helper(c : &PsqlCompositeType, name : &str, opt : &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, CamelCase);

	let struct_body = c.cols
		.iter()
		.map(|v| {
			let field_name = format_heck(&v.name, opt, SnakeCase);
			let schema_name = format_heck(&v.type_ns_name, opt, SnakeCase);
			let type_name = format_heck(&v.type_name, opt, CamelCase);
			if v.not_null {
				quote!{ pub #field_name : super::#schema_name::#type_name }
			} else {
				quote!{ pub #field_name : Option<super::#schema_name::#type_name> }
			}
		});
	let derives = get_derives(opt.serde);

	quote!{
		#derives
		pub struct #name_type {
			#(#struct_body),*
		}
	}
}

/// creates the syn node for a base type (typedef)
fn base_to_ast_helper(b : &PsqlBaseType, opt : &Opt) -> TokenStream {
	let name_type = format_heck(&b.name, opt, CamelCase);

	let oid_type = match b.oid {
		16 => quote!{ bool },
		17 => quote!{ Vec<u8> },
		20 => quote!{ i64 },
		23 => quote!{ i32 },
		26 => quote!{ u32 },
		25 | 1042 | 1043 => quote!{ String },
		1082 => quote!{ chrono::NaiveDate },
		1114 => quote!{ chrono::NaiveDateTime },
		1184 => quote!{ chrono::DateTime<chrono::Utc> },
		1186 => quote!{ sql_db_mapper_core::Interval },
		1700 => quote!{ rust_decimal::Decimal },
		2278 => quote!{ () },
		oid => {
			if opt.debug { println!("No Rust type for postgres type with oid : {}", oid) };
			//format!("\ntype NoRustForSqlType_{} = ();", self.oid)
			return quote!{  };
		}
	};

	quote!{ pub type #name_type = #oid_type; }
}

/// creates the syn node for a domain (newtype)
fn domain_to_ast_helper(b : &PsqlDomain, name : &str, opt : &Opt) ->  TokenStream {
	let name_type   = format_heck(name, opt, CamelCase);
	let schema_name = format_heck(&b.base_ns_name, opt, SnakeCase);
	let type_name   = format_heck(&b.base_name, opt, CamelCase);
	let derives = get_derives(opt.serde);

	quote!{
		#derives
		pub struct #name_type(pub super::#schema_name::#type_name);
	}
}


impl ConvertToAst for Vec<SqlProc> {


	/// Takes a SQL procedure and turns it into a rust function
	///
	/// Implemented on a Vec of SqlProc which contains information on all Procs with the same name
	///
	/// ```ignore
	/// // a non overloaded sql proc
	/// // the sql string sent to the database
	/// const MY_FUNCTION_SQL: &str = "SELECT * FROM \"schema\".\"my_function\"($1,$2)";
	/// // Return struct only generated if the procedure returns an anonymous type
	/// #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
	/// pub struct my_functionReturn {=
	/// 	pub field0: super::pg_catalog::varchar,
	/// 	pub field1: super::schema::typ,
	/// }
	/// // fn can be sync as well
	/// pub async fn my_function(
	/// 	// client is & is async and &mut if sync (mirrors client's methods between tokio-postgres and postgres)
	/// 	client: &mut Client,
	/// 	param0: &super::pg_catalog::varchar,
	/// 	param1: &super::pg_catalog::varchar,
	/// // if the function did not return a set the return type would be Result<Option<T>, SqlError> instead
	/// ) -> Result<Vec<my_functionReturn>, SqlError> {
	/// 	/* implementation */
	/// }
	///
	/// // an overloaded sql proc
	/// // called like overloaded_function((client, other_params)) i.e. it takes a single tuple as input
	/// pub fn overloaded_function<T: overloaded_function::OverloadTrait>(input: T) -> T::Output {
	/// 	<T as overloaded_function::OverloadTrait>::tmp(input)
	/// }
	/// // A private module with a public trait inside is used to hide implementation details
	/// mod overloaded_function {
	/// 	use super::*;
	/// 	would use #[async_trait] in an async mapping
	/// 	pub trait OverloadTrait {
	/// 		type Output;
	/// 		fn tmp(self)-> TokenStream;
	/// 	}
	/// 	const OVERLOAD_FUNCTION0_SQL: &str = "SELECT * FROM \"schema\".\"overloaded_function\"($1)";
	/// 	impl<'a> OverloadTrait for (&'a mut Client, &'a super::pg_catalog::int4) {
	/// 		type Output = Result<Option<super::super::pg_catalog::void>, SqlError>;
	/// 		fn tmp(self)-> TokenStream {
	/// 			/* implementation */
	/// 		}
	/// 	}
	/// 	//impls for other input params
	/// }
	/// ```
	fn to_rust_tokens(&self, opt : &Opt) -> TokenStream {
		if self.len() == 0 {
			if opt.debug { println!("Error; retrieved an empty Vec of SqlProcs") };
			return quote!{  };
		}

		match opt.use_tuples {
			Tuples::ForOverloads => {
				if self.len() == 1 {
					self[0].to_rust_tokens(opt)
				} else {
					to_many_fns(&self, opt)
				}
			},
			Tuples::ForAll => {
				to_many_fns(&self, opt)
			},
			Tuples::NoOverloads => {
				if self.len() == 1 {
					self[0].to_rust_tokens(opt)
				} else {
					if opt.debug { println!("Overloaded Proc: '{}' not mapped", self[0].name) };
					quote!{  }
				}
			},
			Tuples::OldestOverload => {
				self[0].to_rust_tokens(opt)
			},
		}
	}
}
fn to_many_fns(procs : &[SqlProc], opt:&Opt) -> TokenStream {
	let name_type = format_heck(&procs[0].name, opt, SnakeCase);
	let doc_comments = to_overload_doc(&procs, opt);
	let fn_docs = quote!{
		/// This is an overloaded SQL function, it takes one tuple parameter.
		///
		/// Valid input types for this function are:
		#doc_comments
	};

	// output type depending on wether the code is async
	let fn_code = if opt.sync {
		quote!{
			#fn_docs
			pub fn #name_type<T:#name_type::OverloadTrait>(input : T) -> T::Output {
				<T as #name_type::OverloadTrait>::tmp(input)
			}
		}
	} else {
		quote!{
			#fn_docs
			pub fn #name_type<T:#name_type::OverloadTrait>(input : T) -> impl Future<Output = T::Output> {
				async {
					<T as #name_type::OverloadTrait>::tmp(input).await
				}
			}
		}
	};

	let (is_async_trait, async_fn) = if opt.sync {
		(
			quote!{ },
			quote!{ },
		)
	} else {
		(
			quote!{
				use async_trait::async_trait;
				#[async_trait]
			},
			quote!{ async },
		)
	};

	let trait_impls = procs.iter().enumerate().map(|(i,p)| to_trait_impl(i,p, opt));

	quote!{
		#fn_code
		mod #name_type {
			use super::*;
			#is_async_trait
			pub trait OverloadTrait {
				type Output;
				#async_fn fn tmp(self)-> Self::Output;
			}
			#(#trait_impls)*
		}
	}
}

fn to_trait_impl(index : usize, proc : &SqlProc, opt : &Opt) -> TokenStream {
	//build SQL string to call proc
	let new_name = format!("{}{}", proc.name, index);
	as_rust_helper(proc, &new_name, true, opt)
}
fn to_tuple_type(types : &[TypeAndName], opt : &Opt) -> TokenStream {
	let tuple_middle = types.iter().map(|tan| {
		let tmp = tan.typ.to_tokens(opt);
		quote!{ &'a super::#tmp }
	});

	if opt.sync {
		quote!{ (&'a mut Client, #(#tuple_middle),* ) }
	} else {
		quote!{ (&'a Client, #(#tuple_middle),* ) }
	}
}
fn to_tuple_pattern(types : &[TypeAndName], opt : &Opt) -> TokenStream {
	let tuple_middle = types.iter().map(|tan| {
		format_heck(&tan.name, opt, SnakeCase)
	});
	quote!{
		(client, #(#tuple_middle),* )
	}
}

fn to_overload_doc(procs : &[SqlProc], opt:&Opt) -> TokenStream {
	procs.iter().enumerate().map(|(i,v)| {
		let name = &v.name;
		let func_parms = v.inputs.as_function_params(opt);
		let ret_type_name = match &v.outputs {
			ProcOutput::Existing(t) => t.to_tokens(opt).to_string(),
			ProcOutput::NewType(_) => format!("{}{}Return", name, i)
		};
		let new_ret_type_name =
			if v.returns_set {
				format!("Vec<{}>", ret_type_name)
			} else {
				format!("Option<{}>", ret_type_name)
			};
		let doc_comment = format!("{}(( client : &Client, {} )) -> {}", name, func_parms, new_ret_type_name);
		quote!{
			#[doc = #doc_comment]
		}
	}).collect()
}


fn as_rust_helper(proc : &SqlProc, name : &str, is_overide : bool, opt : &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, SnakeCase);

	//build SQL string to call proc
	let call_string_name = format_heck(&format!("{}_SQL", name), opt, ShoutySnake);

	let call_string = make_call_string(&proc.ns_name, &proc.name, proc.num_args as usize);
	let call_string = quote!{ const #call_string_name : &str = #call_string; };

	//if proc returns table create type for that proc
	let new_return_type =
	if let ProcOutput::NewType(tans) = &proc.outputs {
		let struct_name = format_heck(&format!("{}Return", name), opt, CamelCase);
		let struct_body = tans.iter()
			.map(|tan| -> TokenStream {
				let field_name = format_heck(&tan.name, opt, SnakeCase);
				let type_name  = tan.typ.to_tokens(opt);
				quote!{
					pub #field_name : #type_name
				}
			});
		let derives = get_derives(opt.serde);

		quote!{
			#derives
			pub struct #struct_name {
				#(#struct_body),*
			}
		}
	} else {
		quote!{}
	};

	//get the output type name
	let ret_type_name = match &proc.outputs {
		ProcOutput::Existing(t) => {
			if t.schema == "pg_catalog" && t.name == "record" {
				if opt.debug { println!("Cannot make wrapper for procedure {} which returns pg_catalog::record", name) };
				return quote!{};
			} else {
				let typ = t.to_tokens(opt);
				if is_overide {
					quote!{ super::#typ }
				} else {
					quote!{ #typ }
				}
			}
		},
		ProcOutput::NewType(_) => {
			let ret_name = format_heck(&format!("{}Return", name), opt, CamelCase);
			quote!{ #ret_name }
		}
	};
	//get the return type properly wrapped in a Vec or Option
	let new_ret_type_name =
		if proc.returns_set {
			quote!{ Vec<#ret_type_name> }
		} else {
			quote!{ Option<#ret_type_name> }
		};

	let func_params = proc.inputs.as_function_params(opt);
	let query_params = as_query_params(&proc.inputs, opt);

	let (opt_async, opt_await, is_async_trait, client_type) = if opt.sync {
		(quote!{  }, quote!{  }, quote!{  }, quote!{ &mut Client })
	} else {
		(quote!{ async }, quote!{ .await }, quote!{ #[async_trait] }, quote!{ &Client })
	};

	//the body of the function
	let body = if proc.returns_set {
		quote!{
			let stmt = client.prepare(#call_string_name)#opt_await?;
			client
				.query(&stmt, &[#query_params])#opt_await?
				.into_iter()
				.map(#ret_type_name::from_row)
				.collect()
		}
	} else {
		quote!{
			let stmt = client.prepare(#call_string_name)#opt_await?;
			Ok(client
				.query_opt(&stmt, &[#query_params])#opt_await?
				.map(#ret_type_name::from_row)
				.transpose()?
			)
		}
	};
	//the wrappings on the body
	let func_text =
	if is_overide {
		let tuple_type = to_tuple_type(&proc.inputs, opt);
		let tuple_pattern = to_tuple_pattern(&proc.inputs, opt);
		quote!{
			#is_async_trait
			impl<'a> OverloadTrait for #tuple_type {
				type Output = Result<#new_ret_type_name, SqlError>;
				#opt_async fn tmp(self)-> Self::Output {
					let #tuple_pattern = self;
					#body
				}
			}
		}
	} else {
		quote!{
			pub #opt_async fn #name_type(
				client : #client_type,
				#func_params
			) -> Result<#new_ret_type_name, SqlError> {
				#body
			}
		}
	};
	quote!{
		#call_string
		#new_return_type
		#func_text
	}
}


impl ConvertToAst for SqlProc {
	/// Generates a rust functions for a non overloaded SQL procedures
	///
	/// See the documentation on the impl of ConvertToAst for Vec<SqlProc> foir more information
	fn to_rust_tokens(&self, opt : &Opt) -> TokenStream {
		as_rust_helper(&self, &self.name, false, opt)
	}
}

impl FullType {
	fn to_tokens(&self, opt:&Opt) -> TokenStream {
		let typ    = format_heck(&self.name, opt, CamelCase);
		let schema = format_heck(&self.schema, opt, SnakeCase);
		quote!{ super::#schema::#typ }
	}
}


trait ToFuncParams {
	fn as_function_params(&self, opt:&Opt) -> TokenStream;
}
impl ToFuncParams for Vec<TypeAndName> {
	fn as_function_params(&self, opt:&Opt) -> TokenStream {
		self.iter().map(|tan| {
			let name = format_heck(&tan.name, opt, SnakeCase);
			let typ  = tan.typ.to_tokens(opt);
			quote!{ #name : &#typ, }
		}).collect()
	}
}

fn make_call_string(namespace : &str, function : &str, len : usize) -> String {
	let mut ret = format!(r#"SELECT * FROM "{}"."{}"("#, namespace, function);
	for i in 1..len {
		ret += &format!("${},", i);
	}
	ret += &format!("${})", len);
	ret
}

fn as_query_params(inputs : &[TypeAndName], opt:&Opt) -> TokenStream {
	let names = inputs.iter().map(|tan|
		format_heck(&tan.name, opt, SnakeCase)
	);

	quote!{
		#(#names),*
	}
}


fn get_derives(serde : bool) -> TokenStream {
	if serde {
		quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql, Serialize, Deserialize)] }
	} else {
		quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] }
	}
}
