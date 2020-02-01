# Changelog

All notable changes to this project will be documented in this file.


## Unreleased

## 0.0.4 - 2020-01-18

### Added
- Option use_tuples which allows for mapping only overloaded procs to take one tuple, all procs to take tuples, skip mapping overloaded procs at all, and only mapping the first defined of the overloaded procs
- Option for generated types to derive serde Serialize and Deserialize
- Option for generating mapping as a whole crate directory
- Option for changing output types, fields, and functions to use rust standard name casing (CamelCase for types and snake_case for fields and functions)

### Changed
- Backend of code generation no long uses syn Node instead keeping everything in quote generated TokenStreams
- `Interval` type now a newtype around time 0.2 Duration (previous was chrono::Druation which was a reexport of time 0.1 Duration)
- Core no longer reexports its dependencies, instread leaving that up to the generated code to do for itself
- Output now contains (in a doc comment) what version sql_db_mapper it was generated with and the arguments used
- connection string is no longer a positional argument and instead uses the option `--conn`. If not provided the env variable SQL_MAP_CONN is also checked

## 0.0.3 - 2020-01-13

### Fixed
- Overloaded function mapping no longer have 'static lifetime requirement

## 0.0.2 - 2020-01-13

### Added
- This changelog file
- New crate sql_db_mapper_core to contain exports and traits need by generated code
- New crate sql_db_mapper_derive to simplify generated code by deriving required traits
- Ability to generate an async wrapper (which is now the default)

### Changed
- Generated code dependency upgraded from postgres 0.15 to 0.17
- Reexports needed in generated code are moved from sql_db_mapper to the new sql_db_mapper_core
- Trait to convert `Row`s to a type now in sql_db_mapper_core rather than generated
- Backend of code generation moved from `format!`ing strings to syn, proc_macro2, and quote

### Fixed
- Nullable columns now correctly mapped to Option<T>

## 0.0.1 - 2020-01-04

### Added
- Initial Release
- Executable for generating database mappings
