# sql_db_mapper
A command line utility for generating rust mappings to databases.

Connects to a PostgreSQL database and creates a rust module representing all the schemas complete with mappings for stored functions/procedures

Maps SQL table, views, and functions to rust structs and functions using `tokio-postgres` and `postgres`

### Notes
Once generated the generated code does not contain additional checks that the database schema hasn't changed. While some type conversions will fail on the call care should be taken to update the generated code at the same time as the database

All functions generated take the client used to connect to the database as the first argument

SQL procedures/functons which are overloaded (two with the same name and different arguments) are mapped to functions which take a single tuple i,e, `my_func((client, id, "hello")) and my_func((client, id))` this means overloading a previously not overloaded SQL procedure would be a breaking change with regards to the generated code (unless use-tuples with options all or one are used)

### Help
```
sql_db_mapper 0.1.0
Generate a rust wrapper for a PostgreSQL database

USAGE:
    sql_db_mapper [FLAGS] [OPTIONS] --conn <conn> [output]

FLAGS:
    -d, --debug           Activate debug mode
        --dir             Program will treat output as a directory name rather than a file and generate a whole crate.
                          If output is not provided code is printed as usual
    -h, --help            Prints help information
        --no-functions    Only make mappings for tables and views
        --rust-case       Convert names from the database to rust standard (i.e. table names in CamelCase, fields and
                          functions in snake_case)
    -u, --ugly            Skip running output through rustfmt
    -V, --version         Prints version information

OPTIONS:
        --conn <conn>
            String to connect to database, see tokio_postgres::Config for details. If not provided environment variable
            DATABASE_URL is checked instead
        --rustfmt-config <rustfmt-config>              string passed to rustfmt --config
        --rustfmt-config-path <rustfmt-config-path>    string passed to rustfmt --config-path
        --use-tuples <use-tuples>
            How to use tuples (used by default for just overloads). Options: overloads (the default, use tuples to
            represent function overloading). all (Have all functions take a tuple for consitency). none (skip mapping
            overloaded procs at all). one_overload (avoid tuples by only mapping the oldest sql proc in the database)
            [default: overloads]

ARGS:
    <output>    Output file, stdout if not present
```

## sql_db_mapper_core
Contains trait TryFromRow for converting from tokio-postgres Rows to Rust types and implements it for several common types  
Also contains Interval struct for corresponding sql type
Reexports types that are convertable to/from sql types

## sql_db_mapper_derive
Features a derive macro from TryFromRow (defined in sql_db_mapper_core)

### Future Work
* more options relating to how the code is generated
	* Grab text from `COMMENT ON` and stick it in doc comments
	* Allow functions that take (for example) an &varchar to take an &str (varchar is a typedef of String so functions would need to be generic like HashMap's get)
* consider adding support for other popular databases as well or rust database libraries
	* either through connecting to the database as is being currently done or possibly by parsing SQL itself
    * sqlx and diesel code generators would be useful
* Add option for use of TLS connection

### Present TODOs
* remove dependencies from generated code
* allow for proper user configured rustfmt usage
* Prevent generation of empty modules
* Fix how pg_catalouge is populated so optional dependencies are used or not used correctly

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
