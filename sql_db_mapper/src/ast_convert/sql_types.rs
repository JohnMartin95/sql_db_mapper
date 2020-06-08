//! Functions for generating rust types
use super::{
	super::{
		sql_tree::*,
		Opt,
	},
	format_heck,
	Case::*,
	// get_derives,
};
use quote::quote;
use proc_macro2::TokenStream;


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
pub fn type_to_rust(typ: &PsqlType, opt : &Opt)-> TokenStream {
	use PsqlTypType::*;
	match &typ.typ {
		Enum(e)      => enum_to_ast(e, &typ.name, opt),
		Composite(c) => composite_to_ast(c, &typ.name, opt),
		Base(b)      => base_to_ast(b, opt),
		Domain(d)    => domain_to_ast(d, &typ.name, opt),
		Other(oid)        => {
			if *oid == 2278 {
				let name_type = format_heck(&typ.name, opt, CamelCase);
				quote!{ pub type #name_type = (); }
			} else {
				if opt.debug { println!("Couldn't convert type: {}, {}", typ.name, oid) };
				quote!{  }
			}
		},
		SimpleComposite(c) => simple_composite_to_ast(c, &typ.name, opt),
	}
}

/// creates the syn node for an enum
pub fn enum_to_ast(e : &PsqlEnumType, name : &str, opt : &Opt) -> TokenStream {
	let name_type = format_heck(name, opt, CamelCase);

	//the enum definition itself
	let enum_body = e.labels
		.iter()
		.map(|v| {
			format_heck(v, opt, CamelCase)
		});
	let derives = get_derives();

	quote!{
		#derives
		pub enum #name_type {
			#(#enum_body),*
		}
	}
}

/// creates the syn node for a struct
pub fn composite_to_ast(c : &PsqlCompositeType, name : &str, opt : &Opt) -> TokenStream {
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
	let derives = get_derives();

	quote!{
		#derives
		pub struct #name_type {
			#(#struct_body),*
		}
	}
}

/// creates the syn node for a base type (typedef)
pub fn base_to_ast(b : &PsqlBaseType, opt : &Opt) -> TokenStream {
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
pub fn domain_to_ast(b : &PsqlDomain, name : &str, opt : &Opt) ->  TokenStream {
	let name_type   = format_heck(name, opt, CamelCase);
	let schema_name = format_heck(&b.base_ns_name, opt, SnakeCase);
	let type_name   = format_heck(&b.base_name, opt, CamelCase);
	let derives = get_derives();

	quote!{
		#derives
		pub struct #name_type(pub super::#schema_name::#type_name);
	}
}

/// creates the syn node for a struct for the anon return type of a function
pub fn simple_composite_to_ast(c : &NamesAndTypes, name : &str, opt : &Opt) -> TokenStream {
	let struct_name = format_heck(name, opt, CamelCase);
	let struct_body = c.0.iter()
		.map(|tan| -> TokenStream {
			let field_name = format_heck(&tan.name, opt, SnakeCase);
			let type_name  = tan.typ.to_tokens(opt);
			quote!{
				pub #field_name : #type_name
			}
		});
	let derives = get_derives();

	quote!{
		#derives
		pub struct #struct_name {
			#(#struct_body),*
		}
	}
}


fn get_derives() -> TokenStream {
	quote!{
		#[cfg_attr(feature = "with_serde", derive(Serialize, Deserialize))]
		#[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
	}
}
