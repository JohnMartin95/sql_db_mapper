# sql_db_mapper
Rust code generator for sql databases

Connects to a PostgreSQL database and creates a rust module representing the schema complete with mappings for stored functions/procedures

#### Future Work
* upgrade Postgres connection dependecy from posgres-0.15 to tokio-postgres-0.5
	* This will also mean changing generated code to async
* clean code generation to make use of an existing abstract syntax tree for rust
	* this opens the possibilty of allowing use of this crate as a proc macro as well
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
