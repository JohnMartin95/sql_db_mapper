# Changelog

All notable changes to this project will be documented in this file.


## Unreleased

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
