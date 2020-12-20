//! Select statments into pg_* tables and corresponding return types
use sql_db_mapper_core::*;


pub const GET_SCHEMAS: &str = "SELECT ns.oid, nspname, nspowner, rolname
FROM pg_namespace ns
LEFT JOIN pg_roles r
ON nspowner = r.oid
ORDER BY ns.oid ASC";
#[derive(Debug, TryFromRow)]
pub struct GetSchemas {
	pub oid: u32,
	pub name: String,
	pub owner: u32,
	pub rolname: String,
}

pub const GET_TYPES: &str = "SELECT oid,
	typname,
	typlen,
	typbyval,
	typtype,
	typrelid,
	typalign
FROM pg_type
WHERE typnamespace = $1 AND
	(typarray != 0 OR
	typtype = 'd' OR
	oid = 2278)
ORDER BY oid ASC";
#[derive(Debug, TryFromRow)]
pub struct GetTypes {
	pub oid: u32,
	pub name: String,
	pub len: i16,
	pub by_val: bool,
	pub typ: i8,
	pub rel_id: u32,
	pub align: i8,
}

pub const GET_ENUM: &str = "SELECT oid, enumtypid, enumsortorder, enumlabel
FROM pg_enum
WHERE enumtypid = $1
ORDER BY enumsortorder ASC";
#[derive(Debug, TryFromRow)]
pub struct GetEnum {
	pub oid: u32,
	pub typ_id: u32,
	pub sort_order: f32,
	pub label: String,
}

pub const GET_COLUMNS: &str = "SELECT attnum,
	attname,
	atttypid,
	typname,
	nspname,
	attlen,
	atttypmod,
	attnotnull,
	attndims
FROM pg_attribute a
LEFT JOIN pg_type b ON atttypid = b.oid
LEFT JOIN pg_namespace c ON typnamespace = c.oid
WHERE attnum > 0 AND NOT attisdropped
	AND attrelid = $1
ORDER BY attnum ASC";
#[derive(Debug, TryFromRow)]
pub struct GetColumns {
	pub attnum: i16,
	pub name: String,
	pub typ_id: u32,
	pub typ_name: String,
	pub nspname: String,
	pub len: i16,
	pub typ_mod: i32,
	pub not_null: bool,
	pub num_dimentions: i32,
}

pub const GET_DOMAIN_BASE: &str = "SELECT t2.oid,
	ns.nspname,
	t2.typname
FROM pg_type AS t
JOIN pg_type AS t2
	ON t2.oid = t.typbasetype
JOIN pg_namespace AS ns
	ON t2.typnamespace = ns.oid
WHERE t.oid = $1";
#[derive(Debug, TryFromRow)]
pub struct GetDomainBase {
	pub oid: u32,
	pub ns_name: String,
	pub typ_name: String,
}

pub const GET_PROC_NAMES: &str = "SELECT MIN(p.oid) as p_oid,
	p.proname
FROM pg_proc AS p
JOIN pg_namespace AS ns
	ON ns.oid = p.pronamespace
WHERE pronamespace = $1 AND
	pronamespace != 11 AND
	ns.nspname != 'information_schema' AND
	ns.nspname != 'public'
GROUP BY p.proname
ORDER BY p_oid ASC";
#[derive(Debug, TryFromRow)]
pub struct GetProcNames {
	pub oid: u32,
	pub name: String,
}

pub const GET_PROCS: &str = "SELECT ns.oid as ns_oid,
	ns.nspname,
	p.oid as p_oid,
	p.proname,
	p.proretset,
	p.pronargs,
	p.prorettype,
	t.typname,
	p.proargtypes,
	p.proallargtypes,
	p.proargmodes,
	p.proargnames
FROM pg_proc AS p
JOIN pg_namespace AS ns
	ON ns.oid = p.pronamespace
JOIN pg_type AS t
	ON p.prorettype = t.oid
WHERE pronamespace = $1 AND
	proname = $2 AND
	pronamespace != 11 AND
	ns.nspname != 'information_schema' AND
	ns.nspname != 'public'
ORDER BY p_oid ASC";
#[derive(Debug, TryFromRow)]
pub struct GetProcs {
	pub ns_oid: u32,
	pub ns_name: String,
	pub p_oid: u32,
	pub name: String,
	pub returns_set: bool,
	pub num_args: i16,
	pub ret_type_id: u32,
	pub ret_type_name: String,
	pub arg_types: Vec<u32>,
	pub all_arg_types: Option<Vec<u32>>,
	pub arg_modes: Option<Vec<i8>>,
	pub arg_names: Option<Vec<String>>,
}

pub const GET_TYPE_NAME: &str = "SELECT ns.nspname, t.typname
FROM pg_type t
JOIN pg_namespace AS ns
	ON ns.oid = t.typnamespace
WHERE t.oid = $1";
#[derive(Debug, TryFromRow)]
pub struct GetTypeName {
	pub ns_name: String,
	pub name: String,
}
