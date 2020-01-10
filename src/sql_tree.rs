
#[derive(Debug, Clone)]
pub struct FullDB {
	pub schemas : Vec<Schema>,
}

#[derive(Debug, Clone)]
pub struct Schema {
	pub id : SchemaId,
	pub name : String,
	pub owner_name : String,
	pub types : Vec<PsqlType>,
	pub procs : Vec<Vec<SqlProc>>,
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
	Enum(PsqlEnumType),
	Composite(PsqlCompositeType),
	Base(PsqlBaseType),
	Domain(PsqlDomain),
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
	pub type_id : TypeId,
	pub type_name : String,
	pub type_ns_name : String,
	pub not_null : bool
}

type TypeId = u32;

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

#[derive(Debug, Clone)]
pub enum ProcOutput {
	Existing(String),
	NewType(Vec<TypeAndName>)
}
