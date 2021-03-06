//! Functions for generating rust types
use super::{
	super::{sql_tree::*, Opt},
	format_heck,
	Case::*,
};
use crate::ThirdParty;
use ThirdParty::*;

use proc_macro2::TokenStream;
use quote::quote;


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
pub fn type_to_rust(typ: &PsqlType, opt: &Opt) -> TokenStream {
	use PsqlTypType::*;
	let stripped_name = typ.name.clone().replace(|c: char| !(c.is_ascii_alphanumeric() || c=='_'), "_");
	match &typ.typ {
		Enum(e) => enum_to_ast(e, &stripped_name, opt),
		Composite(c) => composite_to_ast(c, &stripped_name, opt),
		Base(b) => base_to_ast(b, opt),
		Domain(d) => domain_to_ast(d, &stripped_name, opt),
		Other(oid) => {
			if *oid == 2278 {
				let name_type = format_heck(&typ.name, opt, CamelCase);
				quote! { pub type #name_type = (); }
			} else {
				if opt.debug {
					println!("Couldn't convert type: {}, {}", typ.name, oid)
				};
				quote! {}
			}
		},
		SimpleComposite(c) => simple_composite_to_ast(c, &typ.name, opt),
	}
}

/// creates the syn node for an enum
pub fn enum_to_ast(e: &PsqlEnumType, name: &str, opt: &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, CamelCase);

	//the enum definition itself
	let enum_body = e.labels.iter().map(|v| format_heck(v, opt, CamelCase));
	let derives = get_derives();

	quote! {
		#derives
		pub enum #name_type {
			#(#enum_body),*
		}
	}
}

/// creates the syn node for a struct
pub fn composite_to_ast(c: &PsqlCompositeType, name: &str, opt: &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, CamelCase);

	let struct_body = c.cols.iter().map(|v| {
		let field_name = format_heck(&v.name, opt, SnakeCase);
		let schema_name = format_heck(&v.type_ns_name, opt, SnakeCase);
		let type_name = format_heck(&v.type_name, opt, CamelCase);
		let mut field_type = if v.not_null {
			quote! { super::#schema_name::#type_name }
		} else {
			quote! { Option<super::#schema_name::#type_name> }
		};
		for _ in 0..v.num_dimentions {
			field_type = quote! { Vec<#field_type> };
		}
		quote! { pub #field_name : #field_type }
	});
	let derives = get_derives();

	quote! {
		#derives
		pub struct #name_type {
			#(#struct_body),*
		}
	}
}

/// like `std::try` but returns an empty TokenStream on None
macro_rules! my_try {
	($expr:expr) => {
		match $expr {
			std::option::Option::Some(val) => val,
			std::option::Option::None => {
				return quote! {};
			},
		}
	};
}

/// creates the syn node for a base type (typedef)
pub fn base_to_ast(b: &PsqlBaseType, opt: &Opt) -> TokenStream {
	let third_party = |lib_name: ThirdParty, tokens: TokenStream| -> Option<TokenStream> {
		if opt.uses_lib(lib_name) {
			Some(tokens)
		} else {
			if opt.debug {
				println!(
					"Enable {} dependency to provide mapping for postgres type `{}` with oid : {}",
					lib_name.to_str(),
					b.name,
					b.oid
				);
			}
			None
		}
	};
	let name_type = format_heck(&b.name, opt, CamelCase);

	let oid_type = match b.oid {
		16 => quote! { std::primitive::bool },
		17 => quote! { Vec<u8> },
		18 => quote! { i8 },
		19 | 25 | 1042 | 1043 => quote! { String },
		20 => quote! { i64 },
		21 => quote! { i16 },
		23 => quote! { i32 },
		26 => quote! { u32 },
		114 | 3802 => my_try!(third_party(SerdeJson, quote! { serde_json::Value })),
		700 => quote! { f32 },
		701 => quote! { f64 },
		869 => quote! { std::net::IpAddr },
		1082 => my_try!(third_party(Chrono, quote! { chrono::NaiveDate })),
		1083 => my_try!(third_party(Chrono, quote! { chrono::NaiveTime })),
		1114 => {
			if opt.uses_lib(Chrono) {
				quote! { chrono::NaiveDateTime }
			} else {
				quote! { std::time::SystemTime }
			}
		},
		1184 => {
			if opt.uses_lib(Chrono) {
				quote! { chrono::DateTime<chrono::Utc> }
			} else {
				quote! { std::time::SystemTime }
			}
		},
		1700 => my_try!(third_party(RustDecimal, quote! { rust_decimal::Decimal })),
		2278 => quote! { () },
		2950 => my_try!(third_party(Uuid, quote! { uuid::Uuid })),
		oid => {
			if opt.debug {
				println!("No Rust type for postgres type `{}` with oid : {}", b.name, oid);
			}
			return quote! {};
		},
	};

	quote! { pub type #name_type = #oid_type; }
}

/// creates the syn node for a domain (newtype)
pub fn domain_to_ast(b: &PsqlDomain, name: &str, opt: &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, CamelCase);
	let schema_name = format_heck(&b.base_ns_name, opt, SnakeCase);
	let type_name = format_heck(&b.base_name, opt, CamelCase);
	let derives = get_derives();

	quote! {
		#derives
		pub struct #name_type(pub super::#schema_name::#type_name);
	}
}

/// creates the syn node for a struct for the anon return type of a function
pub fn simple_composite_to_ast(c: &NamesAndTypes, name: &str, opt: &Opt) -> TokenStream {
	let struct_name = format_heck(name, opt, CamelCase);
	let struct_body = c.0.iter().map(|tan| -> TokenStream {
		let field_name = format_heck(&tan.name, opt, SnakeCase);
		let type_name = tan.typ.to_tokens(opt);
		quote! {
			pub #field_name : #type_name
		}
	});
	let derives = get_derives();

	quote! {
		#derives
		pub struct #struct_name {
			#(#struct_body),*
		}
	}
}


fn get_derives() -> TokenStream {
	quote! {
		#[derive(Serialize, Deserialize)]
		#[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
	}
}
