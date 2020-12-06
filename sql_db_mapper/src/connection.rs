use super::{
	sql_tree::*,
	pg_select_types::*,
};
use postgres::{Client, Statement};
use sql_db_mapper_core::*;


const RUST_KEYWORDS: [&str; 58] = [
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
	"yield",
];

pub struct MyClient {
	client: Client,
	schemas_stmt : Statement,
	types_stmt : Statement,
	enum_stmt : Statement,
	columns_stmt : Statement,
	domain_base_stmt : Statement,
	proc_names_stmt : Statement,
	procs_stmt : Statement,
	type_name_stmt : Statement,

}
//
impl MyClient {
	pub fn new(mut client: Client) -> MyClient {
		MyClient {
			schemas_stmt : client.prepare(GET_SCHEMAS).unwrap(),
			types_stmt : client.prepare(GET_TYPES).unwrap(),
			enum_stmt : client.prepare(GET_ENUM).unwrap(),
			columns_stmt : client.prepare(GET_COLUMNS).unwrap(),
			domain_base_stmt : client.prepare(GET_DOMAIN_BASE).unwrap(),
			proc_names_stmt : client.prepare(GET_PROC_NAMES).unwrap(),
			procs_stmt : client.prepare(GET_PROCS).unwrap(),
			type_name_stmt : client.prepare(GET_TYPE_NAME).unwrap(),
			client,
		}
	}

	pub fn get_all(&mut self, no_functions: bool) -> FullDB {
		let mut full_db = FullDB { schemas: Vec::new() };

		// gets all the schemas in the current db
		let schemas = self.get_schemas().unwrap();
		let schemas : Vec<_> = schemas.into_iter().map(|v|{
			Schema {
				id: v.oid,
				name: v.name,
				owner_name: v.owner,
				types: Vec::new(),
				procs: Vec::new(),
			}
		}).collect();

		for mut schema in schemas {
			//get all types and tables
			let types = self.get_psql_types(schema.id);
			schema.append_types(types);
			//get all stored procedures/functions (if required)
			if !no_functions {
				let (procs, types2) = self.get_procedures(schema.id);
				schema.append_procs(procs);
				schema.append_types(types2);
			}

			//add everything to the schema object
			full_db.add_schema(schema);
		}
		full_db
	}

	pub fn get_procedures(&mut self, schema_id: SchemaId) -> (Vec<Vec<SqlProc>>, Vec<PsqlType>) {
		let names = self.get_proc_names(schema_id).unwrap();

		let (procs, types): (Vec<_>, Vec<_>) = names
			.into_iter()
			.map(|v| self.get_procs_by_name(v.name, schema_id))
			.unzip();

		(procs, types.concat())
	}

	fn get_procs_by_name(&mut self, proc_name: String, schema_id: SchemaId) -> (Vec<SqlProc>, Vec<PsqlType>) {
		let full_procs = self.get_procs(schema_id, proc_name).unwrap();

		let mut procs = Vec::new();
		let mut types = Vec::new();

		for (p, t) in full_procs.into_iter().map(|v| self.get_proc_by_id(v)) {
			procs.push(p);
			types.extend(t);
		}

		(procs, types)
	}

	fn get_proc_by_id(&mut self, v: GetProcs) -> (SqlProc, Option<PsqlType>) {
		let (all_arg_types, arg_modes): (Vec<u32>, Vec<i8>) = if let Some(all_arg_types) = v.all_arg_types {
			if let Some(arg_modes) = v.arg_modes {
				(all_arg_types, arg_modes)
			} else {
				let inputs = v.arg_types;
				let len = inputs.len();
				(inputs, vec![b'i' as i8; len])
			}
		} else {
			let inputs = v.arg_types;
			let len = inputs.len();
			(inputs, vec![b'i' as i8; len])
		};
		let arg_names = v.arg_names;
		let arg_names = match arg_names {
			Some(a_n) => a_n,
			None => Vec::new(),
		};
		let (inputs, outputs) = self.get_proc_output_type(&all_arg_types, &arg_modes, arg_names);

		let new_outputs = if outputs.0.is_empty() {
			let ret_type_id = v.ret_type_id;
			let type_name = self.get_type_name(ret_type_id).unwrap().unwrap();

			FullType {
				schema: type_name.ns_name,
				name: type_name.name,
			}
		} else {
			FullType {
				schema: v.ns_name.clone(),
				name: format!("{}Return", v.name),
			}
		};

		let anon_ret_type = if outputs.0.is_empty() {
			None
		} else {
			Some(PsqlType {
				name: format!("{}Return", v.name),
				ns: v.ns_oid,
				typ: PsqlTypType::SimpleComposite(outputs),
			})
		};

		(
			SqlProc {
				ns: v.ns_oid,
				ns_name: v.ns_name,
				oid: v.p_oid,
				name: v.name,
				returns_set: v.returns_set,
				num_args: v.num_args,
				inputs,
				outputs: new_outputs,
			},
			anon_ret_type,
		)
	}

	fn get_proc_output_type(
		&mut self,
		all_arg_types: &[u32],
		arg_modes: &[i8],
		arg_names: Vec<String>,
	) -> (NamesAndTypes, NamesAndTypes) {
		assert_eq!(all_arg_types.len(), arg_modes.len());
		let arg_names = if all_arg_types.len() != arg_names.len() {
			let mut tmp: Vec<String> = Vec::new();
			for i in 0..all_arg_types.len() {
				tmp.push(format!("input_{}", i));
			}
			tmp
		} else {
			arg_names
		};
		let arg_names: Vec<_> = arg_names
			.into_iter()
			.enumerate()
			.map(|(i, v)| {
				if v.is_empty() || RUST_KEYWORDS.iter().any(|&keyword| keyword == v) {
					format!("input_{}", i)
				} else {
					v
				}
			})
			.collect();
		let mut inputs: Vec<TypeAndName> = Vec::new();
		let mut outputs: Vec<TypeAndName> = Vec::new();

		for i in 0..arg_modes.len() {
			let typ_oid = all_arg_types[i];
			let typ_mode = arg_modes[i];
			let arg_name = arg_names[i].clone();

			let type_name = self.get_type_name(typ_oid).unwrap().unwrap();

			match typ_mode as u8 {
				b'i' => inputs.push(TypeAndName {
					typ: FullType {
						schema: type_name.ns_name,
						name: type_name.name,
					},
					name: arg_name,
				}),
				b't' => outputs.push(TypeAndName {
					typ: FullType {
						schema: type_name.ns_name,
						name: type_name.name,
					},
					name: arg_name,
				}),
				_ => (), //panic!("Only input params and table outputs supported")
			}
		}
		(NamesAndTypes(inputs), NamesAndTypes(outputs))
	}

	pub fn get_psql_types(&mut self, schema_id: SchemaId) -> Vec<PsqlType> {
		let ns_oid = schema_id;
		// let stmt = self.prepare_cached(GET_TYPES);
		let types = self.get_types(ns_oid).unwrap();

		types.into_iter().map(|v| {
			PsqlType {
				name: v.name.clone(),
				ns: schema_id,
				// len : v.len,
				// by_val : v.by_val,
				typ: {
					use PsqlTypType::*;
					match v.typ as u8 {
						b'e' => Enum(PsqlEnumType {
							oid: v.oid,
							labels: self.get_enum_labels(v.oid),
						}),
						b'c' => Composite(PsqlCompositeType {
							oid: v.oid,
							cols: self.get_psql_columns(v.rel_id),
						}),
						b'b' => Base(PsqlBaseType {
							oid: v.oid,
							name: v.name,
						}),
						b'd' => Domain(self.get_psql_domain(v.oid)),
						_ => {
							// println!("typ:{}, name:{}, oid:{}", tmp as u8 as char, v.get::<_, String>(1), v.get::<_, u32>(0));
							Other(v.oid)
						},
					}
				},
				// relid : v.rel_id,
				// align : v.align,
			}
		}).collect()
	}

	fn get_psql_domain(&mut self, oid: u32) -> PsqlDomain {
		let domain_base = self
			.get_domain_base(oid)
			.unwrap()
			.unwrap();
		
		PsqlDomain {
			oid,
			base_oid: domain_base.oid,
			base_ns_name: domain_base.ns_name,
			base_name: domain_base.typ_name,
		}
	}

	pub fn get_psql_columns(&mut self, rel_id: u32) -> Vec<Column> {
		self
			.get_columns(rel_id)
			.unwrap()
			.into_iter()
			.map(|v| Column {
				pos: v.attnum,
				name: v.name,
				type_id: v.typ_id,
				type_name: v.typ_name,
				type_ns_name: v.nspname,
				not_null: v.not_null,
			})
			.collect()
	}

	fn get_enum_labels(&mut self, type_id: u32) -> Vec<String> {
		self
			.get_enum(type_id)
			.unwrap()
			.into_iter()
			.map(|v| v.label)
			.collect()
	}
}
/// Wrappers on SQL select statements
impl MyClient {
	fn get_schemas(&mut self) -> Result<Vec<GetSchemas>, SqlError> {
		self.client
			.query(&self.schemas_stmt, &[])?
			.iter()
			.map(TryFromRow::from_row)
			.collect()
	}
	fn get_types(&mut self, ns_id: u32) -> Result<Vec<GetTypes>, SqlError> {
		self.client
			.query(&self.types_stmt, &[&ns_id])?
			.iter()
			.map(TryFromRow::from_row)
			.collect()
	}
	fn get_enum(&mut self, type_id: u32) -> Result<Vec<GetEnum>, SqlError> {
		self.client
			.query(&self.enum_stmt, &[&type_id])?
			.iter()
			.map(TryFromRow::from_row)
			.collect()
	}
	fn get_columns(&mut self, class_id: u32) -> Result<Vec<GetColumns>, SqlError> {
		self.client
			.query(&self.columns_stmt, &[&class_id])?
			.iter()
			.map(TryFromRow::from_row)
			.collect()
	}
	fn get_domain_base(&mut self, type_id: u32) -> Result<Option<GetDomainBase>, SqlError> {
		self.client
			.query_opt(&self.domain_base_stmt, &[&type_id])?
			.as_ref()
			.map(TryFromRow::from_row)
			.transpose()
	}
	fn get_proc_names(&mut self, ns_id: u32) -> Result<Vec<GetProcNames>, SqlError> {
		self.client
			.query(&self.proc_names_stmt, &[&ns_id])?
			.iter()
			.map(TryFromRow::from_row)
			.collect()
	}
	fn get_procs(&mut self, ns_id: u32, proc_name : String) -> Result<Vec<GetProcs>, SqlError> {
		self.client
			.query(&self.procs_stmt, &[&ns_id, &proc_name])?
			.iter()
			.map(TryFromRow::from_row)
			.collect()
	}
	fn get_type_name(&mut self, id: u32) -> Result<Option<GetTypeName>, SqlError> {
		self.client
			.query_opt(&self.type_name_stmt, &[&id])?
			.as_ref()
			.map(GetTypeName::from_row)
			.transpose()
	}
}