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

const FROM_ROW_TYPES : [&str; 11]= [
"bool",
"Vec<u8>",
"i64",
"i32",
"u32",
"String",
"NaiveDate",
"NaiveDateTime",
"DateTime<Utc>",
"Interval",
"Decimal"
];

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

	trait FromRow {
		fn from_row(row:Row) -> Self;
	}
	// FromRow implementations for primitive/standard types

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
				parse_quote!{ pub use sql_db_mapper::helper_types::{ orm, exports::*, }; },
				parse_quote!{ use orm::*; },
				parse_quote!{ trait FromRow { fn from_row(row:Row) -> Self; } },
				parse_quote!{ impl FromRow for () { fn from_row(_row:Row) -> Self {} } },
			].extend2(
				FROM_ROW_TYPES.iter().map(|&v : &&str| {
					let v : Type = syn::parse_str(v).unwrap();
					parse_quote!{ impl FromRow for #v { fn from_row(row:Row) -> Self { row.get(0) } } }
				})
			).extend2(
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
	type Output = Vec<Item>;

	fn to_rust_ast(&self) -> Self::Output {
		use PsqlTypType::*;
		match &self.typ {
			Enum(e) => enum_to_ast_helper(e, &self.name, self.oid),
			Composite(c) => composite_to_ast_helper(c, &self.name),
			Base(b) => base_to_ast_helper(b),
			Domain(d) => domain_to_ast_helper(d, &self.name),
			Other => {
				let name_type : Type  = syn::parse_str(&self.name).unwrap();
				if self.oid == 2278 {
					vec![ parse_quote!{ pub type #name_type = (); } ]
				} else {
					// println!("	Couldn't convert type: {}, {}", self.name, self.oid);
					Vec::new()
				}
			}
		}
	}
}
fn enum_to_ast_helper(e : &PsqlEnumType, name : &str, oid : u32) ->  Vec<Item> {
	let name_type : Type  = syn::parse_str(name).unwrap();
	let name_str = LitStr::new(name, Span::call_site());
	let oid : LitInt = LitInt::new(&oid.to_string(), Span::call_site());

	//the enum definition itself
	let enum_body : TokenStream = e.labels
		.iter()
		.map(|v| {
			let v_ident : Type  = syn::parse_str(&v).unwrap();
			parse_quote!{ #v_ident, }
		}).collect::<Vec<punctuated::Punctuated<Variant, token::Comma>>>()
		.into_iter()
		.map(|v| v.to_token_stream()).collect();
	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone)] };
	let mut full_enum : ItemEnum = parse_quote!{ pub enum #name_type { #enum_body } };
	full_enum.attrs.push(derive_thing);

	// match armns in the To and FromSql impls
	let (match_arms, to_match_arms): (Vec<Arm>, Vec<Arm>) = e
		.labels
		.iter()
		.map(|v| {
			let v_ident : Type  = syn::parse_str(&v).unwrap();
			let v_str = LitStr::new(&v, Span::call_site());
			let v_bytes = LitByteStr::new(v.as_bytes(), Span::call_site());
			(
				parse_quote!{ #v_str => Ok(Self::#v_ident), },
				parse_quote!{ Self::#v_ident => #v_bytes, },
			)
		}).unzip();
	let match_arms : TokenStream    = match_arms   .into_iter().map(|v| v.to_token_stream()).collect();
	let to_match_arms : TokenStream = to_match_arms.into_iter().map(|v| v.to_token_stream()).collect();

	// impl of FromSql for the enum
	let from_sql_impl : Item = parse_quote!{
		impl FromSql for #name_type {
			fn from_sql<'a>(_: &Type, raw: &'a [u8]) -> std::result::Result<Self, Box<dyn Error + Sync + Send>> {
				let x = String::from_sql(&TEXT, raw)?;
				match x.as_str() {
					#match_arms
					_       => Err(Box::new(EnumParseError::new(#name_str, x)))
				}
			}
			fn accepts(ty: &Type) -> bool {
				ty.oid() == #oid
			}
		}
	};
	// impl of ToSql for the enum
	let to_sql_impl : Item = parse_quote!{
		impl ToSql for #name_type {
			fn to_sql(&self, _: &Type, w: &mut Vec<u8>) -> std::result::Result<IsNull, Box<dyn Error + Sync + Send>> {
				w.extend_from_slice(match self {
					#to_match_arms
				});
				Ok(IsNull::No)
			}

			fn accepts(ty: &Type) -> bool {
				ty.oid() == #oid
			}

			to_sql_checked!();
		}
	};

	vec![
		full_enum.into(),
		from_sql_impl,
		to_sql_impl,
	]
}

fn composite_to_ast_helper(c : &PsqlCompositeType, name : &str) ->  Vec<Item> {
	let name_type : Type  = syn::parse_str(name).unwrap();

	let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone)] };
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

	let from_row_code : TokenStream = c.cols
		.iter()
		.enumerate()
		.map(|(i,v)| -> TokenStream {
			let field_name : Type = syn::parse_str(&v.name).unwrap();
			let index : LitInt = LitInt::new(&i.to_string(), Span::call_site());
			parse_quote!{ #field_name : row.get(#index),}
		}).collect();
	let from_row_impl : Item = parse_quote!{
		impl FromRow for #name_type {
			fn from_row(row:Row) -> Self {
				Self {
					#from_row_code
				}
			}
		}
	};

	vec![
		full_struct.into(),
		from_row_impl,
	]
}

fn base_to_ast_helper(b : &PsqlBaseType) ->  Vec<Item> {
	let oid_type = match b.oid {
		16 => return vec![parse_quote!{ pub use bool; }],
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
		_ => return Vec::new() //format!("\ntype NoRustForSqlType_{} = ();", self.oid)
	};
	let name_type : Type  = syn::parse_str(&b.name).unwrap();
	let oid_type : Type = syn::parse_str(oid_type).unwrap();
	vec![
		parse_quote!{ pub type #name_type = #oid_type; }
	]
}

fn domain_to_ast_helper(b : &PsqlDomain, name : &str) ->  Vec<Item> {
	let name_type : Type  = syn::parse_str(name).unwrap();
	let schema_name : Type = syn::parse_str(&b.base_ns_name).unwrap();
	let type_name   : Type = syn::parse_str(&b.base_name).unwrap();
	vec![
		parse_quote!{ pub type #name_type = #schema_name::#type_name; }
	]
}


impl ConvertToAst for Vec<SqlProc> {
	type Output = Vec<Item>;

	fn to_rust_ast(&self) -> Self::Output {
		match self.len() {
			1 => self[0].to_rust_ast(),
			0 => Vec::new(),
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
					pub fn #name_type<T:#name_type::OverloadTrait>(input : T) -> T::Output {
						<T as #name_type::OverloadTrait>::tmp(input)
					}
				};
				fn_code.attrs.extend(fn_docs);

				let mod_with_impls : ItemMod = parse_quote!{
					mod #name_type {
						use super::*;
						pub trait OverloadTrait {
							type Output;
							fn tmp(self) -> Self::Output;
						}
						#trait_impls
					}
				};
				// vec![parse_quote!{
				// 	/// This is an overloaded SQL function, it takes one tuple parameter.
				// 	///
				// 	/// Valid input types for this function are:
				// 	#doc_comments
				// 	pub fn #name_type<T:#name_type::OverloadTrait>(input : T) -> T::Output {
				// 		<T as #name_type::OverloadTrait>::tmp(input)
				// 	}
				// 	mod #name_type {
				// 		use super::*;
				// 		pub trait OverloadTrait {
				// 			type Output;
				// 			fn tmp(self) -> Self::Output;
				// 		}
				// 		#trait_impls
				// 	}
				// }]
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
	proc.helper(&new_name, true,)
		.iter().map(|v| v.to_token_stream()).collect()
}
fn to_tuple_type(types : &[TypeAndName]) -> Type {
	let mut ret = String::from("(&Connection, ");
	for tan in types {
		ret += "&";
		ret += &tan.typ;
		ret += ", ";
	}
	ret += ")";
	syn::parse_str(&ret).unwrap()
}
fn to_tuple_pattern(types : &[TypeAndName]) -> TokenStream {
	let mut ret = String::from("(conn, ");
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
		let doc_comment = format!("{}(( conn : &Connection, {} )) -> {}", name, func_parms, new_ret_type_name);
		let ret = parse_quote!{
			#[doc = #doc_comment]
		};
		// println!("{}", ret);
		ret
	}).collect()
}

impl SqlProc {
	fn helper(&self, name : &str, is_overide : bool) -> Vec<Item> {
		let name_type : Type  = syn::parse_str(name).unwrap();

		//build SQL string to call proc
		let call_string_name : Type = syn::parse_str(&format!("{}_SQL", name.to_uppercase())).unwrap();

		let call_string = make_call_string(&self.ns_name, &self.name, self.num_args as usize);
		let call_string : Item = parse_quote!{ const #call_string_name : &str = #call_string; };

		let mut ret = vec![
			call_string,
		];

		//if proc returns table create type for that proc
		if let ProcOutput::NewType(tans) = &self.outputs {
			let derive_thing : Attribute = parse_quote!{ #[derive(Debug, Clone)] };
			let struct_body : TokenStream = tans
				.iter()
				.map(|tan| -> TokenStream {
					let field_name : Ident = syn::parse_str(&tan.name).unwrap();
					let type_name : TokenStream = syn::parse_str(&tan.typ).unwrap();
					// let field_name : Ident = syn::parse_str(&tan.name).unwrap();
					// let type_name : TokenStream = syn::parse_str("()").unwrap();
					// println!("{}\n{}\n\n", field_name, type_name);
					parse_quote!{
						pub #field_name : #type_name,
					}
				}).collect();
			// println!();
			//FIXME idon't think #(#name_type)Return worked, gotta amke/parse a string here
			let struct_name : Ident = syn::parse_str(&format!("{}Return", name)).unwrap();
			let mut full_struct : ItemStruct = parse_quote!{ pub struct #struct_name { #struct_body } };
			full_struct.attrs.push(derive_thing);

			let from_row_code : TokenStream = tans
				.iter()
				.enumerate()
				.map(|(i,tan)| -> TokenStream {
					let field_name : Type = syn::parse_str(&tan.name).unwrap();
					let index : LitInt = LitInt::new(&i.to_string(), Span::call_site());
					parse_quote!{ #field_name : row.get(#index),}
				}).collect();
			let from_row_impl : Item = parse_quote! {
				impl FromRow for #struct_name {
					fn from_row(row:Row) -> Self {
						Self {
							#from_row_code
						}
					}
				}
			};

			ret.push(full_struct.into());
			ret.push(from_row_impl);
		}

		//get the output type name
		let ret_type_name = match &self.outputs {
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
		let new_ret_type_name : Type=
			if self.returns_set {
				parse_quote!{ Vec<#ret_type_name> }
			} else {
				parse_quote!{ Option<#ret_type_name> }
			};

		let func_params : TokenStream = self.inputs.as_function_params();
		let query_params : TokenStream = as_query_params(&self.inputs);
		let final_call = if self.returns_set { "collect" } else { "next" };
		let final_call : Ident = syn::parse_str(final_call).unwrap();
		let func_text : Item =
		if is_overide {
			let tuple_type : Type = to_tuple_type(&self.inputs);
			let tuple_pattern : TokenStream = to_tuple_pattern(&self.inputs);
			parse_quote!{
				impl OverloadTrait for #tuple_type {
					type Output = SqlResult<#new_ret_type_name>;
					fn tmp(self) -> Self::Output {
						let #tuple_pattern = self;
						Ok(
							conn
							.prepare_cached(#call_string_name)?
							.query(&[#query_params])?
							.into_iter()
							.map(#ret_type_name::from_row)
							.#final_call()
						)
					}
				}
			}
		} else {
			parse_quote!{
				pub fn #name_type(
					conn : &Connection,
					#func_params
				) -> SqlResult<#new_ret_type_name> {
					Ok(
						conn
						.prepare_cached(#call_string_name)?
						.query(&[#query_params])?
						.into_iter()
						.map(#ret_type_name::from_row)
						.#final_call()
					)
				}

			}
		};

		ret.push(func_text);
		ret
	}
}


impl ConvertToAst for SqlProc {
	type Output = Vec<Item>;
	fn to_rust_ast(&self) -> Self::Output {
		self.helper(&self.name, false)
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
