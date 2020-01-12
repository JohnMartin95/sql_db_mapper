//! Turn the AST of the database from sql_tree into a Rust syntax tree fron syn

use super::{
	sql_tree::*,
	Opt,
};
use syn::*;
use quote::{
	ToTokens,
	quote,
};
use proc_macro2::{
	Span,
	TokenStream,
};

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

/// Takes a reference to a struct and the program options and returns a new type
///
/// Used to turn the types in sql_tree into a Rust syntax tree
pub trait ConvertToAst {
	type Output;
	fn to_rust_ast(&self, opt : &Opt) -> Self::Output;

	fn as_string(&self, opt : &Opt) -> String
	where Self::Output : ToTokens {
		self.to_rust_ast(opt).to_token_stream().to_string()
	}
}

impl ConvertToAst for FullDB {
	type Output = File;

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
	fn to_rust_ast(&self, opt : &Opt) -> Self::Output {
		File {
			shebang: None,
			attrs: vec![
				parse_quote!{ #![allow(non_snake_case)] },
				parse_quote!{ #![allow(unused_imports)] },
				parse_quote!{ #![allow(non_camel_case_types)] },
			],
			items: vec![
				parse_quote!{ pub use sql_db_mapper_core as orm; },
				parse_quote!{ use orm::*; },
			].extend2(
				self.schemas.iter().map(|v| v.to_rust_ast(opt)).map(Item::Mod)
			),
		}
	}
}

impl ConvertToAst for Schema {
	type Output = ItemMod;

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
	fn to_rust_ast(&self, opt : &Opt) -> Self::Output {
		let name = Ident::new(&self.name, Span::call_site());
		let content : TokenStream =
			self.types.iter().map(|v| v.to_rust_ast(opt))
			.flatten()
			.chain(self.procs.iter().map(|v| v.to_rust_ast(opt)).flatten())
			.map(|v| v.to_token_stream()).collect();
		parse_quote!{
			pub mod #name {
				use super::*;
				#content
			}
		}
	}
}

impl ConvertToAst for PsqlType {
	type Output = Option<Item>;
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
	fn to_rust_ast(&self, opt : &Opt) -> Self::Output {
		use PsqlTypType::*;
		Some(
			match &self.typ {
				Enum(e)      => enum_to_ast_helper(e, &self.name, opt),
				Composite(c) => composite_to_ast_helper(c, &self.name, opt),
				Base(b)      => base_to_ast_helper(b, opt)?,
				Domain(d)    => domain_to_ast_helper(d, &self.name, opt),
				Other        => {
					let name_type : Type  = syn::parse_str(&self.name).unwrap();
					if self.oid == 2278 {
						parse_quote!{ pub type #name_type = (); }
					} else {
						if opt.debug { println!("Couldn't convert type: {}, {}", self.name, self.oid) };
						return None;
					}
				}
			}
		)
	}
}

/// creates the syn node for an enum
fn enum_to_ast_helper(e : &PsqlEnumType, name : &str, _opt : &Opt) ->  Item {
	let name_type : Type  = syn::parse_str(name).unwrap();

	//the enum definition itself
	let enum_body : TokenStream = e.labels
		.iter()
		.map(|v| {
			let v_ident : Type  = syn::parse_str(&v).unwrap();
			parse_quote!{ #v_ident, }
		}).collect::<Vec<punctuated::Punctuated<Variant, token::Comma>>>()
		.into_iter()
		.map(|v| v.to_token_stream()).collect();
	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] };
	let mut full_enum : ItemEnum = parse_quote!{ pub enum #name_type { #enum_body } };
	full_enum.attrs.push(derive_thing);

	full_enum.into()
}

/// creates the syn node for a struct
fn composite_to_ast_helper(c : &PsqlCompositeType, name : &str, _opt : &Opt) ->  Item {
	let name_type : Type  = syn::parse_str(name).unwrap();

	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] };
	let struct_body : TokenStream = c.cols
		.iter()
		.map(|v| -> TokenStream {
			let field_name  : Type = syn::parse_str(&v.name).unwrap();
			let schema_name : Type = syn::parse_str(&v.type_ns_name).unwrap();
			let type_name   : Type = syn::parse_str(&v.type_name).unwrap();
			let typ = if v.not_null {
				quote!{ super::#schema_name::#type_name }
			} else {
				quote!{ Option<super::#schema_name::#type_name> }
			};
			parse_quote!{
				pub #field_name : #typ,
			}
		}).collect();
	let mut full_struct : ItemStruct = parse_quote!{ pub struct #name_type { #struct_body } };
	full_struct.attrs.push(derive_thing);

	full_struct.into()
}

/// creates the syn node for a base type (typedef)
fn base_to_ast_helper(b : &PsqlBaseType, opt : &Opt) -> Option<Item> {
	let oid_type = match b.oid {
		16 => return Some(parse_quote!{ pub use bool; }),
		17 => "Vec<u8>",
		20 => "i64",
		23 => "i32",
		26 => "u32",
		25 | 1042 | 1043 => "String",
		1082 => "NaiveDate",
		1114 => "NaiveDateTime",
		1184 => "DateTime<Utc>",
		1186 => "Interval",
		1700 => "Decimal",
		2278 => "()",
		oid => {
			if opt.debug { println!("No Rust type for postgres type with oid : {}", oid) };
			//format!("\ntype NoRustForSqlType_{} = ();", self.oid)
			return None;
		}
	};
	let name_type : Type  = syn::parse_str(&b.name).unwrap();
	let oid_type : Type = syn::parse_str(oid_type).unwrap();

	Some(parse_quote!{ pub type #name_type = #oid_type; })
}

/// creates the syn node for a domain (newtype)
fn domain_to_ast_helper(b : &PsqlDomain, name : &str, _opt : &Opt) ->  Item {
	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] };
	let name_type : Type  = syn::parse_str(name).unwrap();
	let schema_name : Type = syn::parse_str(&b.base_ns_name).unwrap();
	let type_name   : Type = syn::parse_str(&b.base_name).unwrap();
	let mut full_struct : ItemStruct = parse_quote!{
		pub struct #name_type(pub super::#schema_name::#type_name);
	};
	full_struct.attrs.push(derive_thing);

	full_struct.into()
}


impl ConvertToAst for Vec<SqlProc> {
	type Output = Vec<Item>;


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
	/// pub fn overloaded_function<T: 'static + overloaded_function::OverloadTrait>(input: T) -> T::Output {
	/// 	<T as overloaded_function::OverloadTrait>::tmp(input)
	/// }
	/// // A private module with a public trait inside is used to hide implementation details
	/// mod overloaded_function {
	/// 	use super::*;
	/// 	would use #[async_trait] in an async mapping
	/// 	pub trait OverloadTrait {
	/// 		type Output;
	/// 		fn tmp(self) -> Self::Output;
	/// 	}
	/// 	const OVERLOAD_FUNCTION0_SQL: &str = "SELECT * FROM \"schema\".\"overloaded_function\"($1)";
	/// 	impl<'a> OverloadTrait for (&'a mut Client, &'a super::pg_catalog::int4) {
	/// 		type Output = Result<Option<super::super::pg_catalog::void>, SqlError>;
	/// 		fn tmp(self) -> Self::Output {
	/// 			/* implementation */
	/// 		}
	/// 	}
	/// 	//impls for other input params
	/// }
	/// ```
	fn to_rust_ast(&self, opt : &Opt) -> Self::Output {
		match self.len() {
			0 => Vec::new(),
			1 => self[0].to_rust_ast(opt),
			_ => {
				let name_type : Type  = syn::parse_str(&self[0].name).unwrap();
				let trait_impls : TokenStream = self.iter().enumerate().map(|(i,p)| to_trait_impl(i,p, opt)).collect();
				let doc_comments = to_overload_doc(&self);
				let mut fn_docs = vec![
					parse_quote!{#[doc = "This is an overloaded SQL function, it takes one tuple parameter."]},
					parse_quote!{#[doc = ""]},
					parse_quote!{#[doc = "Valid input types for this function are:"]},
				];
				fn_docs.extend(doc_comments);

				// output type depending on wether the code is async
				let output_type = if opt.sync {
					quote!{ T::Output }
				} else {
					quote!{ impl Future<Output = T::Output> }
				};

				let mut fn_code : ItemFn = parse_quote!{
					pub fn #name_type<T:'static + #name_type::OverloadTrait>(input : T) -> #output_type {
						<T as #name_type::OverloadTrait>::tmp(input)
					}
				};
				fn_code.attrs.extend(fn_docs);
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

				let mod_with_impls : ItemMod = parse_quote!{
					mod #name_type {
						use super::*;
						#is_async_trait
						pub trait OverloadTrait {
							type Output;
							#async_fn fn tmp(self) -> Self::Output;
						}
						#trait_impls
					}
				};

				vec![
					fn_code.into(),
					mod_with_impls.into()
				]
			}
		}
	}
}

fn to_trait_impl(index : usize, proc : &SqlProc, opt : &Opt) -> TokenStream {
	//build SQL string to call proc
	let new_name = format!("{}{}", proc.name, index);
	as_rust_helper(proc, &new_name, true, opt)
		.iter().map(|v| v.to_token_stream()).collect()
}
fn to_tuple_type(types : &[TypeAndName], opt : &Opt) -> Type {
	let mut ret = if opt.sync {
		String::from("(&'a mut Client, ")
	} else {
		String::from("(&'a Client, ")
	};
	for tan in types {
		ret += "&'a ";
		ret += &tan.typ;
		ret += ", ";
	}
	ret += ")";
	syn::parse_str(&ret).unwrap()
}
fn to_tuple_pattern(types : &[TypeAndName]) -> TokenStream {
	let mut ret = String::from("(client, ");
	for tan in types {
		ret += &tan.name;
		ret += ", ";
	}
	ret += ")";
	syn::parse_str(&ret).unwrap()
}

fn to_overload_doc(procs : &[SqlProc]) -> Vec<Attribute> {
	procs.iter().enumerate().map(|(i,v)| {
		let name = &v.name;
		let func_parms = v.inputs.as_function_params();
		let ret_type_name = match &v.outputs {
			ProcOutput::Existing(t) => t.clone(),
			ProcOutput::NewType(_) => format!("{}{}Return", name, i)
		};
		let new_ret_type_name =
			if v.returns_set {
				format!("Vec<{}>", ret_type_name)
			} else {
				format!("Option<{}>", ret_type_name)
			};
		let doc_comment = format!("{}(( client : &Client, {} )) -> {}", name, func_parms, new_ret_type_name);
		let ret = parse_quote!{
			#[doc = #doc_comment]
		};
		ret
	}).collect()
}


fn as_rust_helper(proc : &SqlProc, name : &str, is_overide : bool, opt : &Opt) -> Vec<Item> {
	let name_type : Type  = syn::parse_str(name).unwrap();

	//build SQL string to call proc
	let call_string_name : Type = syn::parse_str(&format!("{}_SQL", name.to_uppercase())).unwrap();

	let call_string = make_call_string(&proc.ns_name, &proc.name, proc.num_args as usize);
	let call_string : Item = parse_quote!{ const #call_string_name : &str = #call_string; };

	let mut ret = vec![
		call_string,
	];

	//if proc returns table create type for that proc
	if let ProcOutput::NewType(tans) = &proc.outputs {
		let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] };
		let struct_body : TokenStream = tans
			.iter()
			.map(|tan| -> TokenStream {
				let field_name : Ident = syn::parse_str(&tan.name).unwrap();
				let type_name : TokenStream = syn::parse_str(&tan.typ).unwrap();
				parse_quote!{
					pub #field_name : #type_name,
				}
			}).collect();
		let struct_name : Ident = syn::parse_str(&format!("{}Return", name)).unwrap();
		let mut full_struct : ItemStruct = parse_quote!{ pub struct #struct_name { #struct_body } };
		full_struct.attrs.push(derive_thing);

		ret.push(full_struct.into());
	}

	//get the output type name
	let ret_type_name = match &proc.outputs {
		ProcOutput::Existing(t) => {
			if t == "super::pg_catalog::record" {
				if opt.debug { println!("Cannot make wrapper for procedure {} which returns pg_catalog::record", t) };
				return Vec::new();
			} else {
				if is_overide {
					format!("super::{}", t)
				} else {
					t.clone()
				}
			}
		},
		ProcOutput::NewType(_) => format!("{}Return", name)
	};
	let ret_type_name : Type = syn::parse_str(&ret_type_name).unwrap();
	//get the return type properly wrapped in a Vec or Option
	let new_ret_type_name : Type=
		if proc.returns_set {
			parse_quote!{ Vec<#ret_type_name> }
		} else {
			parse_quote!{ Option<#ret_type_name> }
		};

	let func_params : TokenStream = proc.inputs.as_function_params();
	let query_params : TokenStream = as_query_params(&proc.inputs);

	let (opt_async, opt_await, is_async_trait, client_type) = if opt.sync {
		(quote!{  }, quote!{  }, quote!{  }, quote!{ &mut Client })
	} else {
		(quote!{ async }, quote!{ .await }, quote!{ #[async_trait] }, quote!{ &Client })
	};

	//the body of the function
	let body : TokenStream = if proc.returns_set {
		parse_quote!{
			let stmt = client.prepare(#call_string_name)#opt_await?;
			client
				.query(&stmt, &[#query_params])#opt_await?
				.into_iter()
				.map(#ret_type_name::from_row)
				.collect()
		}
	} else {
		parse_quote!{
			let stmt = client.prepare(#call_string_name)#opt_await?;
			Ok(client
				.query_opt(&stmt, &[#query_params])#opt_await?
				.map(#ret_type_name::from_row)
				.transpose()?
			)
		}
	};
	//the wrappings on the body
	let func_text : Item =
	if is_overide {
		let tuple_type : Type = to_tuple_type(&proc.inputs, opt);
		let tuple_pattern : TokenStream = to_tuple_pattern(&proc.inputs);
		parse_quote!{
			#is_async_trait
			impl<'a> OverloadTrait for #tuple_type {
				type Output = Result<#new_ret_type_name, SqlError>;
				#opt_async fn tmp(self) -> Self::Output {
					let #tuple_pattern = self;
					#body
				}
			}
		}
	} else {
		parse_quote!{
			pub #opt_async fn #name_type(
				client : #client_type,
				#func_params
			) -> Result<#new_ret_type_name, SqlError> {
				#body
			}
		}
	};

	ret.push(func_text);
	ret
}


impl ConvertToAst for SqlProc {
	type Output = Vec<Item>;
	/// Generates a rust functions for a non overloaded SQL procedures
	///
	/// See the documentation on the impl of ConvertToAst for Vec<SqlProc> foir more information
	fn to_rust_ast(&self, opt : &Opt) -> Self::Output {
		as_rust_helper(&self, &self.name, false, opt)
	}
}


trait ToFuncParams {
	fn as_function_params(&self) -> TokenStream;
}
impl ToFuncParams for Vec<TypeAndName> {
	fn as_function_params(&self) -> TokenStream {
		let mut ret = String::new();
		for tan in self {
			ret += &format!("\n\t{} : &{},", tan.name, tan.typ);
		}
		syn::parse_str(&ret).unwrap()
	}
}

fn make_call_string(namespace : &str, function : &str, len : usize) -> LitStr {
	let mut ret = format!(r#"SELECT * FROM "{}"."{}"("#, namespace, function);
	for i in 1..len {
		ret += &format!("${},", i);
	}
	ret += &format!("${})", len);
	let call_string = LitStr::new(&ret, Span::call_site());
	call_string
}

fn as_query_params(inputs : &[TypeAndName]) -> TokenStream {
	let mut ret = String::new();
	for tan in inputs {
		ret += &format!("{}, ", tan.name);
	}
	syn::parse_str(&ret).unwrap()
}
