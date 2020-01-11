use super::sql_tree::*;
use syn::*;
use quote::{
	ToTokens,
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

pub trait ConvertToAst {
	type Output;
	fn to_rust_ast(&self) -> Self::Output;

	fn as_string(&self) -> String
	where Self::Output : ToTokens {
		self.to_rust_ast().to_token_stream().to_string()
	}
}

impl ConvertToAst for FullDB {
	type Output = File;
	/** Output structure

	```ignore
	#![allow(non_snake_case)]
	#![allow(unused_imports)]
	#![allow(non_camel_case_types)]
	pub use sql_db_mapper::helper_types::{
		orm,
		exports::*
	};
	use orm::*;


	//code for each schema here
	``` */
	fn to_rust_ast(&self) -> Self::Output {
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
				self.schemas.iter().map(ConvertToAst::to_rust_ast).map(Item::Mod)
			),
		}
	}
}

impl ConvertToAst for Schema {
	type Output = ItemMod;

	/** Output structure

	```ignore
	mod #schema_name {
		use super::*;

		//code for all types and other types

		//code for each procedures/function
	}
	``` */
	fn to_rust_ast(&self) -> Self::Output {
		let name = Ident::new(&self.name, Span::call_site());
		let content : TokenStream =
			self.types.iter().map(ConvertToAst::to_rust_ast)
			.flatten()
			.chain(self.procs.iter().map(ConvertToAst::to_rust_ast).flatten())
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

	fn to_rust_ast(&self) -> Self::Output {
		use PsqlTypType::*;
		Some(
			match &self.typ {
				Enum(e) => enum_to_ast_helper(e, &self.name),
				Composite(c) => composite_to_ast_helper(c, &self.name),
				Base(b) => return base_to_ast_helper(b),
				Domain(d) => domain_to_ast_helper(d, &self.name),
				Other => {
					let name_type : Type  = syn::parse_str(&self.name).unwrap();
					if self.oid == 2278 {
						parse_quote!{ pub type #name_type = (); }
					} else {
						// println!("	Couldn't convert type: {}, {}", self.name, self.oid);
						return None;
					}
				}
			}
		)
	}
}

fn enum_to_ast_helper(e : &PsqlEnumType, name : &str) ->  Item {
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

fn composite_to_ast_helper(c : &PsqlCompositeType, name : &str) ->  Item {
	let name_type : Type  = syn::parse_str(name).unwrap();

	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] };
	let struct_body : TokenStream = c.cols
		.iter()
		.map(|v| -> TokenStream {
			let field_name  : Type = syn::parse_str(&v.name).unwrap();
			let schema_name : Type = syn::parse_str(&v.type_ns_name).unwrap();
			let type_name   : Type = syn::parse_str(&v.type_name).unwrap();
			parse_quote!{
				pub #field_name : crate::#schema_name::#type_name,
			}
		}).collect();
	let mut full_struct : ItemStruct = parse_quote!{ pub struct #name_type { #struct_body } };
	full_struct.attrs.push(derive_thing);

	full_struct.into()
}

fn base_to_ast_helper(b : &PsqlBaseType) -> Option<Item> {
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
		_ => return None //format!("\ntype NoRustForSqlType_{} = ();", self.oid)
	};
	let name_type : Type  = syn::parse_str(&b.name).unwrap();
	let oid_type : Type = syn::parse_str(oid_type).unwrap();

	Some(parse_quote!{ pub type #name_type = #oid_type; })
}

fn domain_to_ast_helper(b : &PsqlDomain, name : &str) ->  Item {
	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)] };
	let name_type : Type  = syn::parse_str(name).unwrap();
	let schema_name : Type = syn::parse_str(&b.base_ns_name).unwrap();
	let type_name   : Type = syn::parse_str(&b.base_name).unwrap();
	let mut full_struct : ItemStruct = parse_quote!{
		pub struct #name_type(#schema_name::#type_name);
	};
	full_struct.attrs.push(derive_thing);

	full_struct.into()
}


impl ConvertToAst for Vec<SqlProc> {
	type Output = Vec<Item>;

	fn to_rust_ast(&self) -> Self::Output {
		match self.len() {
			0 => Vec::new(),
			1 => self[0].to_rust_ast(),
			_ => {
				let name_type : Type  = syn::parse_str(&self[0].name).unwrap();
				let trait_impls : TokenStream = self.iter().enumerate().map(|(i,p)| to_trait_impl(i,p)).collect();
				let doc_comments = to_overload_doc(&self);
				let mut fn_docs = vec![
					parse_quote!{#[doc = "This is an overloaded SQL function, it takes one tuple parameter."]},
					parse_quote!{#[doc = ""]},
					parse_quote!{#[doc = "Valid input types for this function are:"]},
				];
				fn_docs.extend(doc_comments);

				let mut fn_code : ItemFn = parse_quote!{
					pub fn #name_type<T:'static + #name_type::OverloadTrait>(input : T) -> impl Future<Output = T::Output> {
						<T as #name_type::OverloadTrait>::tmp(input)
					}
				};
				fn_code.attrs.extend(fn_docs);

				let mod_with_impls : ItemMod = parse_quote!{
					mod #name_type {
						use async_trait::async_trait;
						use super::*;
						#[async_trait]
						pub trait OverloadTrait {
							type Output;
							async fn tmp(self) -> Self::Output;
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

fn to_trait_impl(index : usize, proc : &SqlProc) -> TokenStream {
	//build SQL string to call proc
	let new_name = format!("{}{}", proc.name, index);
	as_rust_helper(proc, &new_name, true,)
		.iter().map(|v| v.to_token_stream()).collect()
}
fn to_tuple_type(types : &[TypeAndName]) -> Type {
	let mut ret = String::from("(&'a Client, ");
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
		// println!("{}", ret);
		ret
	}).collect()
}


fn as_rust_helper(proc : &SqlProc, name : &str, is_overide : bool) -> Vec<Item> {
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
		// println!();
		let struct_name : Ident = syn::parse_str(&format!("{}Return", name)).unwrap();
		let mut full_struct : ItemStruct = parse_quote!{ pub struct #struct_name { #struct_body } };
		full_struct.attrs.push(derive_thing);

		ret.push(full_struct.into());
	}

	//get the output type name
	let ret_type_name = match &proc.outputs {
		ProcOutput::Existing(t) => {
			if t == "pg_catalog::record" {
				return Vec::new();
			} else {
				t.clone()
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
	//the body of the function
	let async_body : TokenStream = if proc.returns_set {
		parse_quote!{
			let stmt = client.prepare(#call_string_name).await?;
			client
				.query(&stmt, &[#query_params]).await?
				.into_iter()
				.map(#ret_type_name::from_row)
				.collect()
		}
	} else {
		parse_quote!{
			let stmt = client.prepare(#call_string_name).await?;
			Ok(client
				.query_opt(&stmt, &[#query_params]).await?
				.map(#ret_type_name::from_row)
				.transpose()?
			)
		}
	};
	//the wrappings on the body
	let func_text : Item =
	if is_overide {
		let tuple_type : Type = to_tuple_type(&proc.inputs);
		let tuple_pattern : TokenStream = to_tuple_pattern(&proc.inputs);
		parse_quote!{
			#[async_trait]
			impl<'a> OverloadTrait for #tuple_type {
				type Output = Result<#new_ret_type_name, SqlError>;
				async fn tmp(self) -> Self::Output {
					let #tuple_pattern = self;
					#async_body
				}
			}
		}
	} else {
		parse_quote!{
			pub async fn #name_type(
				client : &Client,
				#func_params
			) -> Result<#new_ret_type_name, SqlError> {
				#async_body
			}
		}
	};

	ret.push(func_text);
	ret
}


impl ConvertToAst for SqlProc {
	type Output = Vec<Item>;
	fn to_rust_ast(&self) -> Self::Output {
		as_rust_helper(&self, &self.name, false)
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
