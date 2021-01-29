# Changelog

All notable changes to this project will be documented in this file.


## Unreleased
Nothing yet

## 0.1.0 - 2021-01-29
This is a major overhaul so only some changes are specifically mentioned
### Added
- Option `use_tuples` which allows for mapping only overloaded procs to take one tuple, all procs to take tuples, skip mapping overloaded procs at all, and only mapping the first defined of the overloaded procs
- Option for generating mapping as a whole crate directory
- Option for changing output types, fields, and functions to use rust standard name casing (CamelCase for types and snake_case for fields and functions)
- Option `no_functions` which allows skipping the mapping of SQL procedures/functions
- Ability to format output code with a specified rustfmt config

### Changed
- Structure of generated code greatly changed breaking up types, and sync/async functions into 3 separate modules
- Backend of code generation no longer uses syn Node instead keeping everything in quote generated TokenStreams
- Output now contains (in a doc comment) what version sql_db_mapper it was generated with and the arguments used
- connection string is no longer a positional argument and instead uses the option `--conn`. If not provided the env variable `DATABASE_URL` is also checked
- TryFromSql now takes row by reference
- Core now provides more impls for TryFromRow and has placed those impls behind feature gates
- Use tokio 1.0

### Removed
- Options serde and sync removed in favor of always generating the code for both serde derives and for async and sync function wrapper around procedures but letting all those be feature-gated in the generated code
- `Interval` type no longer provided

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
