use postgres::{Client, Statement};
use super::db_model::*;
use std::collections::{
	HashMap,
	hash_map::Entry,
};

const GET_SCHEMAS : &str =
"SELECT ns.oid, nspname, nspowner, rolname
FROM pg_namespace ns
LEFT JOIN pg_roles r
ON nspowner = r.oid
ORDER BY ns.oid ASC";


const GET_TYPES : &str =
"SELECT oid,
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

const GET_ENUM : &str =
"SELECT oid, *
FROM pg_enum
WHERE enumtypid = $1
ORDER BY enumsortorder ASC";


const GET_COLUMNS : &str =
"SELECT attnum,
	attname,
	atttypid,
	typname,
	nspname,
	attlen,
	atttypmod,
	attnotnull
FROM pg_attribute a
LEFT JOIN pg_type b ON atttypid = b.oid
LEFT JOIN pg_namespace c ON typnamespace = c.oid
WHERE attnum > 0 AND NOT attisdropped
	AND attrelid = $1
ORDER BY attnum ASC";

const GET_DOMAIN_BASE : &str =
"SELECT t2.oid,
	ns.nspname,
	t2.typname
FROM pg_type AS t
JOIN pg_type AS t2
	ON t2.oid = t.typbasetype
JOIN pg_namespace AS ns
	ON t2.typnamespace = ns.oid
WHERE t.oid = $1";

const GET_PROC_NAMES : &str =
"SELECT MIN(p.oid) as p_oid,
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

const GET_PROCS : &str =
"SELECT ns.oid as ns_oid,
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

const GET_TYPE_NAME : &str =
"SELECT ns.nspname, t.typname
FROM pg_type t
JOIN pg_namespace AS ns
	ON ns.oid = t.typnamespace
WHERE t.oid = $1";

const RUST_KEYWORDS : [&str; 58]= [
	"as",
	"use",
	"extern crate",
	"break",
	"const",
	"continue",
	"crate",
	"else",
	"if",
	"if let",
	"enum",
	"extern",
	"false",
	"fn",
	"for",
	"if",
	"impl",
	"in",
	"for",
	"let",
	"loop",
	"match",
	"mod",
	"move",
	"mut",
	"pub",
	"impl",
	"ref",
	"return",
	"Self",
	"self",
	"static",
	"struct",
	"super",
	"trait",
	"true",
	"type",
	"unsafe",
	"use",
	"where",
	"while",
	"abstract",
	"alignof",
	"become",
	"box",
	"do",
	"final",
	"macro",
	"offsetof",
	"override",
	"priv",
	"proc",
	"pure",
	"sizeof",
	"typeof",
	"unsized",
	"virtual",
	"yield"
];

pub struct MyClient {
	client : Client,
	statements : HashMap<&'static str, Statement>
}
//
impl MyClient {
	pub fn new(client: Client) -> MyClient {
		MyClient {
			client,
			statements : HashMap::new(),
		}
	}


	pub fn prepare_cached<'a>(&'a mut self, stmt_str : &'static str) -> Statement {
		match self.statements.entry(stmt_str) {
			Entry::Occupied(v) => v.into_mut().clone(),
			Entry::Vacant(v) => v.insert(self.client.prepare(stmt_str).unwrap()).clone(),
		}
	}

	pub fn get_all(&mut self) -> FullDB {
		let mut full_db = FullDB {schemas : Vec::new()};

		// gets all the schemas in the current db
		let schemas = self.get_schemas();

		for mut schema in schemas {
			//get all types and tables
			let types = self.get_types(schema.id);
			for typ in types {
				schema.add_type(typ);
			}
			//get all stored procedures/functions
			let procs = self.get_procedures(schema.id);
			schema.append(procs);

			//add everything to the schema object
			full_db.add_schema(schema);
		}
		full_db
	}

	pub fn get_schemas(&mut self) -> Vec<Schema> {
		self.client.query(GET_SCHEMAS, &[])
			.unwrap()
			.into_iter()
			.map(|row| {
				Schema {
					id :row.get(0),
					name : row.get(1),
					owner_name : row.get(3),
					types : Vec::new(),
					procs : Vec::new()
				}
			}).collect()
	}

	pub fn get_procedures(&mut self, schema_id : SchemaId) -> Vec<Vec<SqlProc>> {
		let stmt = self.prepare_cached(GET_PROC_NAMES);

		self.client
		.query(&stmt, &[&schema_id])
		.unwrap()
		.into_iter()
		.map(|v| -> String {
			v.get(1)
		})
		.map(|proc_name| -> Vec<SqlProc> {
			let stmt = self.prepare_cached(GET_PROCS);

			self.client
			.query(&stmt, &[&schema_id, &proc_name])
			.unwrap()
			.into_iter()
			.map(|v| {
				let all_arg_types : Option<Vec<u32>> = v.get(9);
				let arg_modes : Option<Vec<i8>> = v.get(10);

				let(all_arg_types, arg_modes): (Vec<u32>, Vec<i8>) =
					if let Some(all_arg_types) = all_arg_types {
						if let Some(arg_modes) = arg_modes {
							(all_arg_types, arg_modes)
						} else {
							let inputs : Vec<u32> = v.get(8);
							let len = inputs.len();
							(inputs, vec![b'i' as i8; len])
						}
					} else {
						let inputs : Vec<u32> = v.get(8);
						let len = inputs.len();
						(inputs, vec![b'i' as i8; len])
					};
				let arg_names : Option<Vec<String>> = v.get(11);
				let arg_names = match arg_names {
					Some(a_n) => a_n,
					None => Vec::new()
				};
				let (inputs, outputs) = self.get_proc_output_type(&all_arg_types, &arg_modes, arg_names);

				let outputs = if outputs.is_empty() {
					let ret_type_id : u32 = v.get(6);
					let stmt = self.prepare_cached(GET_TYPE_NAME);
					let mut type_name : Vec<_> = self.client
						.query(&stmt, &[&ret_type_id])
						.unwrap()
						.into_iter()
						.map(|v2| {
							let ns  : String = v2.get(0);
							let typ : String = v2.get(1);
							(ns, typ)
						}).collect();
					assert_eq!(type_name.len(), 1);
					let (nspname,typename) = type_name.remove(0);
					let full_typ = format!("crate::{}::{}", nspname, typename);
					ProcOutput::Existing(full_typ)
				} else {
					ProcOutput::NewType(outputs)
				};

				SqlProc {
					ns : v.get(0),
					ns_name : v.get(1),
					oid : v.get(2),
					name : v.get(3),
					returns_set : v.get(4),
					num_args : v.get(5),
					inputs,
					outputs,
				}
			}).collect()
		}).collect()
	}

	fn get_proc_output_type(&mut self, all_arg_types : &[u32], arg_modes: &[i8], arg_names : Vec<String>) -> (Vec<TypeAndName>, Vec<TypeAndName>) {
		assert_eq!(all_arg_types.len(), arg_modes.len());
		let arg_names =
			if all_arg_types.len() != arg_names.len() {
				let mut tmp : Vec<String> = Vec::new();
				for i in 0..all_arg_types.len() {
					tmp.push(format!("input_{}", i));
				}
				tmp
			} else {
				arg_names
			};
		let arg_names : Vec<_> = arg_names.into_iter().enumerate().map(|(i,v)| {
			if v.is_empty() || RUST_KEYWORDS.iter().any(|&keyword| keyword==v) {
				format!("input_{}", i)
			} else {
				v
			}
		}).collect();
		let mut inputs  : Vec<TypeAndName> = Vec::new();
		let mut outputs : Vec<TypeAndName> = Vec::new();

		for i in 0..arg_modes.len() {
			let typ_oid = all_arg_types[i];
			let typ_mode = arg_modes[i];
			let arg_name = arg_names[i].clone();

			let stmt = self.prepare_cached(GET_TYPE_NAME);
			let mut type_name : Vec<_> = self.client
				.query(&stmt, &[&typ_oid])
				.unwrap()
				.into_iter()
				.map(|v2| {
					let ns  : String = v2.get(0);
					let typ : String = v2.get(1);
					(ns, typ)
				}).collect();
			assert_eq!(type_name.len(), 1);
			let (nspname,typename) = type_name.remove(0);
			let full_typ = format!("crate::{}::{}", nspname, typename);

			match typ_mode as u8 {
				b'i' => inputs.push(TypeAndName{typ:full_typ, name:arg_name}),
				b't' => outputs.push(TypeAndName{typ:full_typ, name:arg_name}),
				_ => ()//panic!("Only input params and table outputs supported")
			}
		}
		(inputs, outputs)
	}

	pub fn get_types(&mut self, schema_id : SchemaId) -> Vec<PsqlType>{
		let ns_oid = schema_id;
		let stmt = self.prepare_cached(GET_TYPES);

		self.client
		.query(&stmt, &[&ns_oid])
		.unwrap()
		.into_iter()
		.map(|v| {
			PsqlType {
				oid : v.get(0),
				name : v.get(1),
				ns : schema_id,
				len : v.get(2),
				by_val : v.get(3),
				typ : {
					let tmp : i8 = v.get(4);
					use PsqlTypType::*;
					match tmp as u8 as char {
						'e' => Enum(PsqlEnumType {
							labels :self.get_enum_labels(v.get(0))
						}),
						'c' => Composite(PsqlCompositeType {
							cols : self.get_columns(v.get(5))
						}),
						'b' => Base(PsqlBaseType {
							oid : v.get(0),
							name : v.get(1)
						}),
						'd' => Domain(self.get_domain_base(v.get(0))),
						_ => {
							// println!("typ:{}, name:{}, oid:{}", tmp as u8 as char, v.get::<_, String>(1), v.get::<_, u32>(0));
							Other
						}
					}
				},
				relid : v.get(5),
				align : v.get(6)
			}
		}).collect()
	}

	fn get_domain_base(&mut self, oid : u32) -> PsqlDomain{
		let stmt = self.prepare_cached(GET_DOMAIN_BASE);

		self.client
		.query(&stmt, &[&oid])
		.unwrap()
		.into_iter()
		.map(|v| {
			PsqlDomain {
				base_oid : v.get(0),
				base_ns_name : v.get(1),
				base_name : v.get(2)
			}
		}).next()
		.unwrap()

	}

	pub fn get_columns(&mut self, rel_id : u32) -> Vec<Column>{
		let stmt = self.prepare_cached(GET_COLUMNS);

		self.client
		.query(&stmt, &[&rel_id])
		.unwrap()
		.into_iter()
		.map(|v| {
			Column {
				pos : v.get(0),
				name : v.get(1),
				type_id : v.get(2),
				type_name : v.get(3),
				type_ns_name : v.get(4),
				not_null : v.get(7)
			}
		}).collect()
	}

	fn get_enum_labels(&mut self, type_id:u32) -> Vec<String> {
		let stmt = self.prepare_cached(GET_ENUM);

		self.client
		.query(&stmt, &[&type_id])
		.unwrap()
		.into_iter()
		.map(|v| {
			v.get(3)
		}).collect()
	}
}
