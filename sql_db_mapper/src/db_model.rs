pub use super::sql_tree::*;

pub trait ConvertToRust {
	fn as_rust_string(&self) -> String {
		"".to_owned()
	}
}

impl ConvertToRust for FullDB {
	fn as_rust_string(&self) -> String{
		let mut ret = String::new();
		ret +=
r#"#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
pub use sql_db_mapper_core as orm;
use orm::*;
"#;
		for schema in &self.schemas {
			// println!("{}", schema.name);
			ret += &schema.as_rust_string();
			ret += "\n";
		}
		ret
	}
}

impl ConvertToRust for Schema {
	fn as_rust_string(&self) -> String {
		let content : String =
			self.types.iter().map(|typ| {
				typ.as_rust_string().replace("\n", "\n\t").replace("\n\t\n", "\n\n")
			}).chain(
				self.procs.iter().map(|procs_with_samne_name| {
					procs_with_samne_name.as_rust_string().replace("\n", "\n\t").replace("\n\t\n", "\n\n")
				})
			).collect();

		format!("\npub mod {} {{\n\tuse super::*;\n{}\n}}\n", self.name, content)
	}
}

impl ConvertToRust for PsqlType {
	fn as_rust_string(&self) -> String {
		use PsqlTypType::*;
		match &self.typ {
			Enum(e) => {
				format!("\n#[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]\npub enum {} {{{}\n}}\n",
					self.name,
					e.as_rust_string()
				)
			},
			Composite(c) => {
				format!("\n#[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]\npub struct {} {{{}\n}}\n",
					self.name,
					c.as_rust_string(),
				)
			},
			Base(b) => b.as_rust_string(),
			Domain(d) => {
				format!("\n#[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]\npub struct {}({}::{});",
					self.name,
					d.base_ns_name,
					d.base_name,
				)
			},
			Other => {
				if self.oid == 2278 {
					format!("\npub type {} = ();",
						self.name
					)
				} else {
					// println!("	Couldn't convert type: {}, {}", self.name, self.oid);
					"".to_owned()
				}
			}
		}
	}
}

impl ConvertToRust for PsqlEnumType {
	fn as_rust_string(&self) -> String {
		self.labels
		.iter()
		.fold(String::new(), |acc, s| {
			format!("{}\n\t{},", acc, s)
		})
	}
}

impl ConvertToRust for PsqlCompositeType {
	fn as_rust_string(&self) -> String {
		self.cols
		.iter()
		.map(|v| {
			v.as_rust_string()
		}).fold(String::new(), |acc, s| {
			format!("{}\n\t{},", acc, s)
		})
	}
}

impl ConvertToRust for PsqlBaseType {
	fn as_rust_string(&self) -> String {
		format!("\npub type {} = {};", self.name, {
			match self.oid {
				16 => return "\npub use bool;".to_owned(),
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
				_ => return "".to_owned() //format!("\ntype NoRustForSqlType_{} = ();", self.oid)
			}
		})
	}
}

impl ConvertToRust for Vec<SqlProc> {
	fn as_rust_string(&self) -> String {
		let trait_impls : String = self.iter().enumerate().map(|(i,p)| to_trait_impl(i,p)).collect();
		match self.len() {
			//no overloading
			0 => String::new(),
			1 => self[0].as_rust_string(),
			_ => {
				let doc_comments : String = self.iter().map(|s| {
					format!(
						"\n/// * {}((\n/// \tconn : &Connection,{}\n/// )) -> {}",
						s.name,
						s.inputs.as_function_params().replace("\n", "\n/// "),
						s.get_ret_type().1.replace("<", "&lt;").replace(">", "&gt;")
					)
				}).collect();
				format!(
			r#"/// This is an overloaded SQL function, it takes one tuple parameter.
///
///Valid input types for this function are:
///{2}
pub fn {0}<T:{0}::OverloadTrait>(input : T) -> T::Output {{
	<T as {0}::OverloadTrait>::tmp(input)
}}
mod {0} {{
	use super::*;
	pub trait OverloadTrait {{
		type Output;
		fn tmp(self) -> Self::Output;
	}}
	{1}
}}"#,
					self[0].name,
					trait_impls.replace("\n\t\n", "\n\n"),
					doc_comments,
				)
			},
		}
	}
}

fn to_trait_impl(index : usize, proc : &SqlProc) -> String {
	as_rust_helper(proc, &format!("{}{}", proc.name, index), true)
}
fn to_tuple_type(types : &[TypeAndName]) -> String {
	let mut ret = String::from("(&Connection, ");
	for tan in types {
		ret += "&";
		ret += &tan.typ;
		ret += ", ";
	}
	ret += ")";
	ret
}
fn to_tuple_pattern(types : &[TypeAndName]) -> String {
	let mut ret = String::from("(conn, ");
	for tan in types {
		ret += &tan.name;
		ret += ", ";
	}
	ret += ")";
	ret
}

impl ConvertToRust for Column {
	fn as_rust_string(&self) -> String {
		format!("pub {} : crate::{}::{}", self.name, self.type_ns_name, self.type_name)
	}
}

fn as_rust_helper(proc : &SqlProc, name : &str, is_overide : bool) -> String {
	let mut ret = "\n".to_owned();
	//build SQL string to call proc
	let call_string_name = format!("{}_SQL", name.to_uppercase());

	let call_string = make_call_string(&proc.ns_name, &proc.name, proc.num_args as usize);
	let call_string = format!("const {} : &str = {};\n", call_string_name, call_string);
	ret += &call_string;

	//if proc returns table create type for that proc
	if let ProcOutput::NewType(tans) = &proc.outputs {
		let struct_body : String = tans.iter().map(|tan| {
			format!("\n\t\tpub {} : {},", tan.name, tan.typ)
		}).collect();
		ret += &format!("#[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]\npub struct {}Return {{{}\n}}\n", proc.name, struct_body);
	}
	//get the output type name
	let ret_type_name = match &proc.outputs {
		ProcOutput::Existing(t) => {
			if t == "pg_catalog::record" {
				return String::new();
			} else {
				t.clone()
			}
		},
		ProcOutput::NewType(_) => format!("{}Return", name)
	};
	let new_ret_type_name : String =
		if proc.returns_set {
			format!{ "Vec<{}>", ret_type_name }
		} else {
			format!{ "Option<{}>", ret_type_name }
		};

	//make function string
	let func_params = proc.inputs.as_function_params();
	let query_params = as_query_params(&proc.inputs);
	let final_call = if proc.returns_set { "collect" } else { "next" };
	let func_text =
	if is_overide {
		let tuple_type = to_tuple_type(&proc.inputs);
		let tuple_pattern = to_tuple_pattern(&proc.inputs);
		format!(
r"
impl OverloadTrait for {} {{
	type Output = SqlResult<{}>;
	fn tmp(self) -> Self::Output {{
		let {} = self;
		Ok(
			conn
			.prepare_cached({})?
			.query(&[{}])?
			.into_iter()
			.map({}::from_row)
			.{}()
		)
	}}
}}
",
			tuple_type,
			new_ret_type_name,
			tuple_pattern,
			call_string_name,
			query_params,
			ret_type_name,
			final_call,
		)
	} else {
		format!(
r"pub fn {}(
conn : &Connection,{}
) -> SqlResult<{}> {{
Ok(
	conn
	.prepare_cached({})?
	.query(&[{}])?
	.into_iter()
	.map({}::from_row)
	.{}()
)
}}
",
			proc.name,
			func_params,
			new_ret_type_name,
			call_string_name,
			query_params,
			ret_type_name,
			final_call,
		)
	};
	ret += &func_text;

	ret

}

impl SqlProc {
	fn get_ret_type(&self) -> (String, String) {
		//get the output type name
		let ret_type_name = match &self.outputs {
			ProcOutput::Existing(t) => t.clone(),
			ProcOutput::NewType(_) => format!("{}Return", self.name)
		};
		let new_ret_type_name =
			if self.returns_set {
				format!("Vec<{}>", ret_type_name)
			} else {
				format!("Option<{}>", ret_type_name)
			};
		(ret_type_name, new_ret_type_name)
	}
}

impl ConvertToRust for SqlProc {
	fn as_rust_string(&self) -> String {
		as_rust_helper(&self, &self.name, false)
	}
}

impl ConvertToRust for Vec<TypeAndName> {
	fn as_rust_string(&self) -> String {
		let mut ret = String::new();
		for tan in self {
			ret += &format!("\n\tpub {} : {},", tan.name, tan.typ);
		}
		ret
	}
}
impl ToFuncParams for Vec<TypeAndName> {
	fn as_function_params(&self) -> String {
		let mut ret = String::new();
		for tan in self {
			ret += &format!("\n\t{} : &{},", tan.name, tan.typ);
		}
		ret
	}
}

trait ToFuncParams {
	fn as_function_params(&self) -> String;
}
trait ToImpl {
	fn to_impl(&self) -> String;
}

fn make_call_string(namespace : &str, function : &str, len : usize) -> String {
	let mut ret = format!(r#""SELECT * FROM \"{}\".\"{}\"("#, namespace, function);
	for i in 1..len {
		ret += &format!("${},", i);
	}
	ret += &format!("${})\"", len);
	ret
}

fn as_query_params(inputs : &[TypeAndName]) -> String {
	let mut ret = String::new();
	for tan in inputs {
		ret += &format!("{}, ", tan.name);
	}
	ret
}
impl ToImpl for Vec<TypeAndName> {
	fn to_impl(&self) -> String {
		let mut ret = String::new();
		for (i, tan) in self.iter().enumerate() {
			ret += &format!("\n\t\t\t{} : row.get({}),", tan.name, i);
		}
		ret
	}
}
