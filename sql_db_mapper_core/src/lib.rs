#![forbid(unsafe_code)]
//! Helper types and functions for auto-generateed psql database wrappers
//!
//! Provides the [`TryFromRow`] trait which converts from a [`tokio_postgres::Row`]. Implementations are provided for common types
//!
//! Also contains [`Interval`] which represents a SQL Interval
//!
//! Reexports [`tokio_postgres::Error`] as SqlError (the Result::Err of the return from [`TryFromRow::from_row`]) and [`tokio_postgres::Row`]
//!
//! [`tokio_postgres::Error`]: https://docs.rs/tokio-postgres/0.6/tokio_postgres/error/struct.Error.html
//! [`tokio_postgres::Row`]: https://docs.rs/tokio-postgres/0.6/tokio_postgres/row/struct.Row.html
//! [`TryFromRow::from_row`]: ./trait.TryFromRow.html#tymethod.from_row
//! [`TryFromRow`]: ./trait.TryFromRow.html
//! [`Interval`]: ./struct.Interval.html

//reexports
pub use sql_db_mapper_derive::*;
pub use postgres_types::{ FromSql, ToSql };
pub use tokio_postgres::{row::Row, Error as SqlError};

#[cfg(feature = "chrono")]
pub use chrono;
#[cfg(feature = "rust_decimal")]
pub use rust_decimal;
#[cfg(feature = "eui48")]
pub use eui48;
#[cfg(feature = "geo_types")]
pub use geo_types;
#[cfg(feature = "serde_json")]
pub use serde_json;
#[cfg(feature = "uuid")]
pub use uuid;
#[cfg(feature = "bit_vec")]
pub use bit_vec;

/// Implementation of `TryFromRow` for various types
mod try_from_row;
pub use try_from_row::TryFromRow;

#[cfg(feature = "time")]
/// Wrapper type around a [`time::Duration`] that implements [`ToSql`], [`FromSql`], and [`TryFromRow`]
///
/// [`time::Duration`]: https://docs.rs/time/0.2/time/struct.Duration.html
/// [`ToSql`]: https://docs.rs/postgres-types/0.1/postgres_types/trait.ToSql.html
/// [`FromSql`]: https://docs.rs/postgres-types/0.1/postgres_types/trait.FromSql.html
/// [`TryFromRow`]: ./trait.TryFromRow.html
#[cfg_attr(feature = "with_serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interval {
	pub dur: time::Duration,
}

use postgres_types::{to_sql_checked, IsNull, Type};
#[cfg(feature = "time")]
impl FromSql<'_> for Interval {
	fn from_sql(_ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		let x = i64::from_sql(&Type::INT4, &raw[0..8])?;
		Ok(Interval {
			dur: time::Duration::microseconds(x),
		})
	}

	fn accepts(ty: &Type) -> bool {
		ty.oid() == 1186
	}
}
#[cfg(feature = "time")]
impl ToSql for Interval {
	to_sql_checked!();

	fn to_sql(&self, _ty: &Type, mut out: &mut bytes::BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
		let i = self.dur.whole_milliseconds();
		(i as i64).to_sql(&Type::INT4, &mut out)
	}

	fn accepts(ty: &Type) -> bool {
		ty.oid() == 1186
	}
}
