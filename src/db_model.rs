pub type SchemaId = u32;

pub trait ConvertToRust {
	fn as_rust_string(&self) -> String {
		"".to_owned()
	}
}

pub struct FullDB {
	pub schemas : Vec<Schema>,
}
impl FullDB {
	pub fn add_schema(&mut self, schema : Schema) {
		self.schemas.push(schema);
	}
}

const FROM_ROW_TYPES : [&'static str; 11]= [
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

impl ConvertToRust for FullDB {
	fn as_rust_string(&self) -> String{
		let mut ret = String::new();
		ret += "#![allow(non_snake_case)]\n";
		ret += "#![allow(unused_imports)]\n";
		ret += "#![allow(non_camel_case_types)]\n";
		ret += "\npub use sql_db_mapper::orm::orm;\n";
		ret += "use sql_db_mapper::orm::exports::*;\n";
		ret += "use orm::*;\n";
		// ret += "use postgres::types::{FromSql, Type, TEXT};\n";
		ret += &format!("\ntrait FromRow {{\n\tfn from_row(row:Row) -> Self;\n}}\n\n");
		for s in FROM_ROW_TYPES.iter() {
			ret += &format!("impl FromRow for {} {{\n\tfn from_row(row:Row) -> Self {{ row.get(0) }}\n}}\n", s);
		}
		ret += &format!("impl FromRow for () {{\n\tfn from_row(_row:Row) -> Self {{}}\n}}\n");
		for schema in &self.schemas {
			// println!("{}", schema.name);
			ret += &schema.as_rust_string();
			ret += "\n";
		}
		ret
	}
}

type TypeId = u32;

pub struct Schema {
	pub id : SchemaId,
	pub name : String,
	pub owner_name : String,
	pub types : Vec<PsqlType>,
	pub procs : Vec<Vec<SqlProc>>,
}
impl Schema {
	pub fn add_type(&mut self, typ : PsqlType) {
		self.types.push(typ);
	}
	// pub fn add_proc(&mut self, proc : SqlProc) {
	// 	self.procs.push(proc);
	// }
	pub fn append(&mut self, mut all_procs : Vec<Vec<SqlProc>>) {
		self.procs.append(&mut all_procs);
	}
}
impl ConvertToRust for Schema {
	fn as_rust_string(&self) -> String {
		let mut ret = String::new();
		ret += &format!("\npub mod {} {{\n\tuse super::*;\n", self.name);
		for typ in &self.types {
			ret += &typ.as_rust_string().replace("\n", "\n\t").replace("\n\t\n", "\n\n");
		}
		for procs_with_samne_name in &self.procs {
			if procs_with_samne_name.len() == 1 {
				//no overloading
				ret += &procs_with_samne_name[0].as_rust_string().replace("\n", "\n\t").replace("\n\t\n", "\n\n");
			} else if procs_with_samne_name.len() > 1 {
				ret += &overload_fn_to_rust_string(procs_with_samne_name).replace("\n", "\n\t").replace("\n\t\n", "\n\n");
			}
			//ignore case where Vec is empty
		}
		ret += &format!("\n}}\n");
		ret
	}
}
fn overload_fn_to_rust_string(procs : &Vec<SqlProc>) -> String {
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
		procs[0].name,
		to_trait_impls(procs).replace("\n", "\n\t").replace("\n\t\n", "\n\n"),
		to_overload_doc(procs),
	)
}

fn to_overload_doc(procs : &Vec<SqlProc>) -> String {
	let mut ret = String::new();
	for proc in procs {
		ret += &format!(
			"\n/// * {}((\n/// \tconn : &Connection,{}\n/// )) -> {}",
			proc.name,
			proc.inputs.as_function_params().replace("\n", "\n/// "),
			proc.get_ret_type().1.replace("<", "&lt;").replace(">", "&gt;")
		);
	}
	ret
}

fn to_trait_impls(procs : &Vec<SqlProc>) -> String {
	procs.iter().enumerate().map(|(i,p)| to_trait_impl(i,p)).collect()
}

fn to_trait_impl(index : usize, proc : &SqlProc) -> String {
	let mut ret = "\n".to_owned();
	//build SQL string to call proc
	let call_string_name = format!("{}{}_SQL", proc.name.to_uppercase(), index);

	let mut call_string = format!(r#"const {} : &str = "SELECT * FROM \"{}\".\"{}\"("#, call_string_name, proc.ns_name, proc.name);
	for i in 1..proc.num_args {
		call_string += &format!("${},", i);
	}
	call_string += &format!("${})\";\n", proc.num_args);
	ret += &call_string;

	//if proc returns table create type for that proc
	if let ProcOutput::NewType(tans) = &proc.outputs {
		ret += &format!("#[derive(Debug, Clone)]\npub struct {}{}Return {{{}\n}}\n", proc.name, index, tans.as_rust_string());
		ret += &format!("impl FromRow for {}Return {{\n\tfn from_row(row:Row) -> Self {{\n\t\t{}Return {{{}\n\t\t}}\n\t}}\n}}\n", proc.name, proc.name, tans.to_impl());
	}
	//get the output type name
	let ret_type_name = match &proc.outputs {
		ProcOutput::Existing(t) => t.clone(),
		ProcOutput::NewType(_) => format!("{}{}Return", proc.name, index)
	};
	if ret_type_name == "pg_catalog::record" {
		return "".to_owned();
	}
	let new_ret_type_name =
		if proc.returns_set {
			format!("Vec<{}>", ret_type_name)
		} else {
			format!("Option<{}>", ret_type_name)
		};
	//make function string
	let func_text = format!(
r#"
impl OverloadTrait for {} {{
	type Output = SqlResult<{}>;
	fn tmp(self) -> Self::Output {{
		let {} = self;
		Ok(
			conn
			.prepare_cached({})?
			.query(&[{}])?
			.into_iter()
			.map(|v| {}::from_row(v))
			.{}()
		)
	}}
}}
"#,
		to_tuple_type(&proc.inputs),
		new_ret_type_name,
		to_tuple_pattern(&proc.inputs).replace("\n", "\n\t"),
		call_string_name,
		proc.inputs.as_query_params(),
		ret_type_name,
		if proc.returns_set { "collect" } else { "next" },
	);
	ret += &func_text;

	ret
}
fn to_tuple_type(types : &Vec<TypeAndName>) -> String {
	let mut ret = String::from("(&Connection, ");
	for tan in types {
		ret += "&";
		ret += &tan.typ;
		ret += ", ";
	}
	ret += ")";
	ret
}
fn to_tuple_pattern(types : &Vec<TypeAndName>) -> String {
	let mut ret = String::from("(conn, ");
	for tan in types {
		ret += &tan.name;
		ret += ", ";
	}
	ret += ")";
	ret
}

pub struct PsqlType {
	pub oid : u32,
	pub name : String,
	pub ns : SchemaId,
	pub len : i16,
	pub by_val : bool,
	pub typ : PsqlTypType,
	pub relid : u32,
	pub align : i8
}
impl ConvertToRust for PsqlType {
	fn as_rust_string(&self) -> String {
		use PsqlTypType::*;
		match &self.typ {
			Enum(e) => {
				format!("\n#[derive(Debug, Clone)]\npub enum {} {{{}\n}}\n",
					self.name,
					e.as_rust_string()
				)
				+
				&e.to_impl(&self.name, self.oid)
			},
			Composite(c) => {
				format!("\n#[derive(Debug, Clone)]\npub struct {} {{{}\n}}\nimpl FromRow for {} {{\n\tfn from_row(row:Row) -> Self {{\n\t\t{} {{{}\n\t\t}}\n\t}}\n}}\n",
					self.name,
					c.as_rust_string(),
					self.name,
					self.name,
					c.to_impl()
				)
			},
			Base(b) => b.as_rust_string(),
			Domain(d) => {
				format!("\npub type {} = {};",
					self.name,
					d.as_rust_string()
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
pub enum PsqlTypType {
	Enum(PsqlEnumType),
	Composite(PsqlCompositeType),
	Base(PsqlBaseType),
	Domain(PsqlDomain),
	Other
}
pub struct PsqlEnumType {
	pub labels : Vec<String>
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
impl PsqlEnumType {
	pub fn to_impl(&self, name : &str, oid : u32) -> String {
		let mut match_arms = String::new();
		let mut to_match_arms = String::new();
		for lab in self.labels.iter() {
			match_arms += &format!("\"{}\" => Ok(Self::{}),\n\t\t\t", lab, lab);
			to_match_arms += &format!("Self::{} => b\"{}\",\n\t\t\t", lab, lab);
		}
		//the return string
		format!(
r#"impl FromSql for {} {{
	fn from_sql<'a>(_: &Type, raw: &'a [u8]) -> std::result::Result<Self, Box<dyn Error + Sync + Send>> {{
		let x = String::from_sql(&TEXT, raw)?;
		match x.as_str() {{
			{}_       => Err(Box::new(EnumParseError::new("{}", x)))
		}}
	}}
	fn accepts(ty: &Type) -> bool {{
		ty.oid() == {}
	}}
}}
impl ToSql for {} {{
	fn to_sql(&self, _: &Type, w: &mut Vec<u8>) -> std::result::Result<IsNull, Box<dyn Error + Sync + Send>> {{
		w.extend_from_slice(match self {{
			{}
		}});
		Ok(IsNull::No)
	}}

	fn accepts(ty: &Type) -> bool {{
		ty.oid() == {}
	}}

	to_sql_checked!();
}}
"#, name, match_arms, name, oid, name, to_match_arms, oid)
	}
}

pub struct PsqlCompositeType {
	pub cols : Vec<Column>
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
impl ToImpl for PsqlCompositeType {
	fn to_impl(&self) -> String {
		let mut ret = String::new();
		for (i, col) in self.cols.iter().enumerate() {
			ret += &format!("\n\t\t\t{} : row.get({}),", col.name, i);
		}
		ret
	}
}

pub struct PsqlBaseType {
	pub oid : u32,
	pub name : String
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

pub struct PsqlDomain {
	pub base_oid : u32,
	pub base_name : String,
	pub base_ns_name : String
}
impl ConvertToRust for PsqlDomain {
	fn as_rust_string(&self) -> String {
		format!("{}::{}", self.base_ns_name, self.base_name)
	}
}

pub struct Column {
	pub pos : i16,
	pub name : String,
	pub type_id : TypeId,
	pub type_name : String,
	pub type_ns_name : String,
	pub not_null : bool
}
impl ConvertToRust for Column {
	fn as_rust_string(&self) -> String {
		format!("pub {} : crate::{}::{}", self.name, self.type_ns_name, self.type_name)
	}
}

pub struct SqlProc {
	pub ns : u32,
	pub ns_name : String,
	pub oid : u32,
	pub name : String,
	pub returns_set : bool,
	pub num_args : i16,
	pub inputs : Vec<TypeAndName>,
	pub outputs: ProcOutput,
}
pub struct TypeAndName {
	pub typ : String,
	pub name : String
}
pub enum ProcOutput {
	Existing(String),
	NewType(Vec<TypeAndName>)
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
		let mut ret = "\n".to_owned();
		//build SQL string to call proc
		let call_string_name = format!("{}_SQL", self.name.to_uppercase());

		let mut call_string = format!(r#"const {} : &str = "SELECT * FROM \"{}\".\"{}\"("#, call_string_name, self.ns_name, self.name);
		for i in 1..self.num_args {
			call_string += &format!("${},", i);
		}
		call_string += &format!("${})\";\n", self.num_args);
		ret += &call_string;

		//if proc returns table create type for that proc
		if let ProcOutput::NewType(tans) = &self.outputs {
			ret += &format!("#[derive(Debug, Clone)]\npub struct {}Return {{{}\n}}\n", self.name, tans.as_rust_string());
			ret += &format!("impl FromRow for {}Return {{\n\tfn from_row(row:Row) -> Self {{\n\t\t{}Return {{{}\n\t\t}}\n\t}}\n}}\n", self.name, self.name, tans.to_impl());
		}
		//get the output type name
		let (ret_type_name, new_ret_type_name) = self.get_ret_type();
		if ret_type_name == "pg_catalog::record" {
			return "".to_owned();
		}
		//make function string
		let func_text = format!(
r"pub fn {}(
	conn : &Connection,{}
) -> SqlResult<{}> {{
	Ok(
		conn
		.prepare_cached({})?
		.query(&[{}])?
		.into_iter()
		.map(|v| {}::from_row(v))
		.{}()
	)
}}
",
			self.name,
			self.inputs.as_function_params(),
			new_ret_type_name,
			call_string_name,
			self.inputs.as_query_params(),
			ret_type_name,
			if self.returns_set { "collect" } else { "next" }
		);
		ret += &func_text;

		ret
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
trait ToQueryParams {
	fn as_query_params(&self) -> String;
}
trait ToFuncParams {
	fn as_function_params(&self) -> String;
}
trait ToImpl {
	fn to_impl(&self) -> String;
}
impl ToQueryParams for Vec<TypeAndName> {
	fn as_query_params(&self) -> String {
		let mut ret = String::new();
		for tan in self {
			ret += &format!("{}, ", tan.name);
		}
		ret
	}
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
