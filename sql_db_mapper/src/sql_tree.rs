//! A simple AST of a PostgreSQL database

/// The root Node of the database, contains all the schemas
#[derive(Debug, Clone)]
pub struct FullDB {
	pub schemas : Vec<Schema>,
}

impl FullDB {
	pub fn add_schema(&mut self, schema : Schema) {
		self.schemas.push(schema);
	}
}

/// Database schema. COntains all Types and procedures defined inside
///
/// All sql procures with overloading (the same name) are stored in a Vec the the length of the `procs` Vec is the number of unique procedure names in the schema
#[derive(Debug, Clone)]
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
	pub fn append(&mut self, mut all_procs : Vec<Vec<SqlProc>>) {
		self.procs.append(&mut all_procs);
	}
}

pub type SchemaId = u32;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum PsqlTypType {
	/// pg_type.typtype e
	Enum(PsqlEnumType),
	/// pg_type.typtype c
	Composite(PsqlCompositeType),
	/// pg_type.typtype b
	Base(PsqlBaseType),
	/// pg_type.typtype d
	Domain(PsqlDomain),
	/// Types not included above (p, r) Currently ignored but may be used in the future
	Other
}

#[derive(Debug, Clone)]
pub struct PsqlEnumType {
	pub labels : Vec<String>
}

#[derive(Debug, Clone)]
pub struct PsqlCompositeType {
	pub cols : Vec<Column>
}

#[derive(Debug, Clone)]
pub struct Column {
	pub pos : i16,
	pub name : String,
	pub type_id : u32,
	pub type_name : String,
	pub type_ns_name : String,
	pub not_null : bool
}

#[derive(Debug, Clone)]
pub struct PsqlBaseType {
	pub oid : u32,
	pub name : String
}

#[derive(Debug, Clone)]
pub struct PsqlDomain {
	pub base_oid : u32,
	pub base_name : String,
	pub base_ns_name : String
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct TypeAndName {
	pub typ : String,
	pub name : String
}

/// The return type of a procedure
#[derive(Debug, Clone)]
pub enum ProcOutput {
	/// The procedure returns an existing type (columns of a table, user defined enum)
	Existing(String),
	/// The procedure returns a new anonymous type
	NewType(Vec<TypeAndName>)
}
