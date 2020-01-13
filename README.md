# sql_db_mapper
A command line utility for generating rust mappings to databases.

Connects to a PostgreSQL database and creates a rust module representing all the schemas complete with mappings for stored functions/procedures

Defaults to creating an async wrapping using `tokio-postgres` but has a flag to make synchronous functions with `postgres` instead

### Notes
Once generated the generated code does not contain additional checks that the database schema hasn't changed. While some type conversions will fail on the call care should be taken to update the generated code at the same time as the database

All functions generated take the client used to connect to the database as the first argument

SQL procedures/functons which are overloaded (two with the same name and different arguments) are mapped in this crate to functions which take a single tuple i,e, `my_func((client, id, "hello")) and my_func((client, id))` this means overloading a previously not overloaded SQL procedure would be a breaking change with regards to the generated code

### Help
```
sql_db_mapper 0.0.2
Generate a rust wrapper for a PostgreSQL database

USAGE:
    sql_db_mapper [FLAGS] <conn-string> [output]

FLAGS:
    -d, --debug      Activate debug mode
    -h, --help       Prints help information
    -s, --sync       Generate synchronous mapping
    -u, --ugly       Skip running output through rustfmt
    -V, --version    Prints version information

ARGS:
    <conn-string>    String to connect to database, see tokio_postgres::Config for details
    <output>         Output file, stdout if not present
```

## sql_db_mapper_core
Contains reexports and helper types which the generated code pulls in

## sql_db_mapper_derive
Features a derive macro from TryFromRow (defined in sql_db_mapper_core) which provides conversions from postgres' Row struct

### Future Work
* more options relating to how the code is generated
	* derives that may be wanted (serde comes to mind)
	* add option for all generated functions to take a tuple to avoid client breaking changes noted above
	* a derive or other proc_macro version of the code. It may not be recommended for compile time reasons but perhaps somebody would appreciate it
	* generate a whole (properly-structured) crate wrapping the database rather than one file and printing the dependencies to the console
	* increasing performance
	* use heck to allow names to be mapped to appropriate rust equivalents
* consider adding support for other popular databases as well
	* either through connecting to the database as is being currently done or possibly by parsing SQL itself

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
