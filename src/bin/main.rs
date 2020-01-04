use postgres::{Connection, TlsMode};
use std::{
	fs::File,
	io::Write
};
use sql_db_mapper::{
	connection::*,
	db_model::*,
};

use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
	let mut args : Vec<String> = env::args().collect();
	if !(args.len() == 2 || args.len() == 3) {
		panic!("Must be called with exactly 1 or 2 arguments.\n\tsql_db_mapper connection_string [output_file]\nIf output_file is left out Rust code is printed to stdout")
	}

	let connection_string = args.remove(1);
	let output_file = if args.len() == 2 {
		Some(args.remove(1))
	} else {
		 None
	};

	let conn = Connection::connect(connection_string, TlsMode::None).unwrap();
	std::mem::drop(args);

	println!("[dependencies]\nsql_db_mapper = \"{}\"\n\n", VERSION);

	let conn = MyConnection::new(&conn);
	let mut full_db = FullDB {schemas : Vec::new()};

	// gets all the schemas in the current db
	let schemas = conn.get_schemas();

	for mut schema in schemas {
		//get all types and tables
		let types = conn.get_types(schema.id);
		for typ in types {
			schema.add_type(typ);
		}
		//get all stored procedures/functions
		let procs = conn.get_procedures(schema.id);
		schema.append(procs);

		//add everything to the schema object
		full_db.add_schema(schema);
	}

	let s = full_db.as_rust_string();
	if let Some(output_file) = output_file {
		let f = File::create(output_file);
		match f {
			Ok(mut f) => f.write_all(s.as_bytes()).expect("failed to write to file"),
			Err(e) => {
				eprintln!("Error ({}) while opening output file. Writing output to stdout just in case", e);
				println!("{}", s);
			}
		}
	} else {
		println!("{}", s);
	}
}
//


/*	//get all columns (in order) with their types for a given table
	//note: need to keep track of all user defined types and give them a default implementation
*/
/*
//past me wrote this like a saint knowing I would someday need it
//gets the types needed for all the functions
SELECT
	A.oid as "id",
	B.nspname as "namespace",
	A.proname as "functionName",
	--Coalesce's prevent nulls, instead return empty arrays
	--just take length(proargtypes) from proargnames as proargtypes conatains just oid of inputs
	COALESCE((SELECT A.proargnames[0: array_length(A.proargtypes, 1)]), '{}') as "inputNames",
	COALESCE(
		--the names for user defined types, null if row doesn't return a user defined type
		H.colNames::text[],
		--take just the names from the end(representing output names, if it's null or empty null it to try the next
		NULLIF((select A.proargnames[array_length(A.proargtypes, 1)+ 1 : array_length(A.proargnames, 1)]), '{}'),
		CASE WHEN C.typname='void' THEN
			--there are actually no output values
			'{}'::text[]
		ELSE
			--there was 1 output value
			'{RETVALUE}'
		END
	) as "outputNames",
	--next two are similar to above two but for type rather than name
	COALESCE((SELECT A.typeNames[0: array_length(A.proargtypes, 1)]), '{}')::text[] as "inputTypes", --Array of IN/INOUT type names
	COALESCE(
		H.typeNames::text[],
		NULLIF((select A.typeNames[array_length(A.proargtypes, 1)+ 1 : array_length(A.typeNames, 1)]), '{}'),
		CASE WHEN C.typname='void' THEN
			'{}'
		ELSE
			ARRAY[C.typname] --the typename for a function that returns a base value
		END
	) as "outputTypes"--Array of OUT argument types
FROM
--hell below
(
	SELECT --values we want/need from, not * because we eed to group_by for the array_agg(regation)
		s.oid,  s.proname, s.prorettype, s.proargtypes, s.proargmodes, s.pronamespace, s.proargnames,
		--the reason for all this crazy, an array of argument typenames
		--NULL if removes void type argument we imply below
		NULLIF(array_agg(s.typname),'{void}') as typeNames
	FROM(
		SELECT p.oid, p.*, typname --grab everything from pg_proc along with 1 typename for one of its arguments
		--cross join pg_proc and the elements from p.proallargtypes, or p.proargtypes, or 'void'
		--proallargtypes would contain all the types per its name but it is null when all args are inputs
		--proargtypes contains all input types solving this problem but
			--we need at least 1 element in the array for the join (joining a function and it's list of arguments, joins ignore rows that don't exist, outer joins also didn't help)
			--so we need to return null if proargtypes is empty
			--the remaining case is a function that takes no arguments and returns single value or void
			--for this case we return the type void(to act as the one element in the array)
		FROM
			pg_proc p,
			--break up the array of type oids from their array
			unnest(
				--do what was mentioned above
				COALESCE(
					p.proallargtypes,
					NULLIF( --make it null when empty
						(--the value we want to compare to is p.proargtypes but it is a oidvector so '{}' can't be coerced to that
							--make an array from... --the cast is needed because the below converion to string would cause this to return text[]
							SELECT array_agg(z.unnest::oid)
							from
							(
								--.. elements of (the vector cast to a string turned to an array)
								SELECT unnest(
									string_to_array(p.proargtypes::text,' ')
								)
							) z
						), '{}' --if the above equals this return null
					),
					--the function has no arguments so lets say it has one argument of type void
					--type void had oid of 2278 at time of writing but lets not rely on that
					--array_agg because these are all arrays of all argument types
					(SELECT array_agg(oid) FROM pg_type WHERE typname='void')
				)
			) WITH ordinality u --give ordinality equal to the order of the arguments
			--this join lets us join each type oid to it's typename
			JOIN pg_type t ON t.oid = u
		ORDER BY ordinality
	) s
	--the hard part is done
	GROUP BY s.oid, s.proname,s.prorettype, s.proargtypes, s.proargmodes, s.pronamespace, s.proargnames
) A
--gives access to the namespace name
JOIN pg_namespace B on A.pronamespace=B.oid
--gives access to the return type, (int, void, ect.. as well as record or some user-defined type)
JOIN pg_type C on A.prorettype=C.oid
--gives the return type's namespace
JOIN pg_namespace D on C.typnamespace=D.oid
--this one gives us the column names and types for user defined types (types and tables)
left JOIN
(
	--get the type id, the array of type names, and the array of column names
	select E.oid, array_agg(G.typname ORDER BY F.attnum) as typeNames, array_agg(F.attname ORDER BY F.attnum) as colNames
	--type E of user made type
	from pg_type E
		--attributes F contains the columns of type E
		join pg_attribute F on E.typrelid=F.attrelid AND F.attnum>0
		--type G conatins the name of the types of the columns in G
		join pg_type G on F.atttypid=G.oid
	group by E.oid
) H on H.oid=A.prorettype --join on any functions that return user defined types
--I don't give any craps about the default functions
WHERE B.nspname!='pg_catalog' AND B.nspname!='information_schema'AND B.nspname!='public'
--put them in some order
ORDER BY id;
*/
