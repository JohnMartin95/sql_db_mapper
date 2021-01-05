#![forbid(unsafe_code)]
//! Helper types and functions for auto-generateed psql database wrappers
//!
//! Provides the [`TryFromRow`] trait which converts from a [`tokio_postgres::Row`]. Implementations are provided for common types
//!
//! Reexports [`tokio_postgres::Error`] as SqlError (the Result::Err of the return from [`TryFromRow::from_row`]) and [`tokio_postgres::Row`]
//!
//! [`tokio_postgres::Error`]: https://docs.rs/tokio-postgres/0.6/tokio_postgres/error/struct.Error.html
//! [`tokio_postgres::Row`]: https://docs.rs/tokio-postgres/0.6/tokio_postgres/row/struct.Row.html
//! [`TryFromRow::from_row`]: ./trait.TryFromRow.html#tymethod.from_row
//! [`TryFromRow`]: ./trait.TryFromRow.html
//! [`Interval`]: ./struct.Interval.html

//reexports
pub use postgres::Client as SyncClient;
pub use postgres_types::{FromSql, ToSql};
pub use sql_db_mapper_derive::*;
pub use tokio_postgres::{row::Row, Client as AsyncClient, Error as SqlError};

#[cfg(feature = "bit-vec")]
pub use bit_vec;
#[cfg(feature = "chrono")]
pub use chrono;
#[cfg(feature = "geo-types")]
pub use geo_types;
#[cfg(feature = "rust_decimal")]
pub use rust_decimal;
#[cfg(feature = "serde_json")]
pub use serde_json;
#[cfg(feature = "uuid")]
pub use uuid;

/// Implementation of `TryFromRow` for various types
mod try_from_row;
pub use try_from_row::TryFromRow;
