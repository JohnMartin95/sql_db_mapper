//! Helper types and functions for auto-generateed psql database wrappers
//!
//! Provides the [`TryFromRow`] trait which converts from a [`tokio_postgres::Row`]. Implementations are provided for common types
//!
//! Also contains [`Interval`] which represents a SQL Interval
//!
//! Reexports [`tokio_postgres::Error`] as SqlError (the Result::Err of the return from [`TryFromRow::from_row`]) and [`tokio_postgres::Row`]
//!
//! [`tokio_postgres::Error`]: https://docs.rs/tokio-postgres/0.5.2/tokio_postgres/error/struct.Error.html
//! [`tokio_postgres::Row`]: https://docs.rs/tokio-postgres/0.5.2/tokio_postgres/row/struct.Row.html
//! [`TryFromRow::from_row`]: ./trait.TryFromRow.html#tymethod.from_row
//! [`TryFromRow`]: ./trait.TryFromRow.html
//! [`Interval`]: ./struct.Interval.html

pub use tokio_postgres::{
	Error as SqlError,
	row::Row,
};

use postgres_types::{
	FromSql,
	ToSql,
	IsNull,
	Type,
	to_sql_checked,
};

use std::error::Error;

use rust_decimal;

use chrono::{NaiveDateTime, NaiveDate, DateTime, Utc};
use time::Duration;

pub use sql_db_mapper_derive::*;


/// Converts from a [`tokio_postgres::Row`]. Implementations are provided for common types
///
/// [`TryFromRow`]: ./trait.TryFromRow.html
/// [`tokio_postgres::Row`]: https://docs.rs/tokio-postgres/0.5.2/tokio_postgres/row/struct.Row.html
pub trait TryFromRow: Sized {
	fn from_row(row : Row) -> Result<Self, SqlError>;
}
#[cfg(feature = "with_serde")]
use serde::{
	Serialize,
	Deserialize,
};

/// Wrapper type around a [`time::Duration`] that implements [`ToSql`] and [`FromSql`]
///
/// [`time::Duration`]: https://docs.rs/time/0.2.6/time/struct.Duration.html
/// [`ToSql`]: https://docs.rs/postgres-types/0.1.0/postgres_types/trait.ToSql.html
/// [`FromSql`]: https://docs.rs/postgres-types/0.1.0/postgres_types/trait.FromSql.html
#[cfg_attr(feature = "with_serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Interval {
	pub dur : Duration
}
impl FromSql<'_> for Interval {
	fn from_sql(_ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let x = i64::from_sql(&Type::INT4, &raw[0..8])?;
		Ok(Interval{ dur : Duration::microseconds(x) })
	}
	fn accepts(ty: &Type) -> bool {
		ty.oid() == 1186
	}
}
impl ToSql for Interval {
	fn to_sql(&self, _ty: &Type, mut out: &mut bytes::BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
		let i = self.dur.whole_milliseconds();
		(i as i64).to_sql(&Type::INT4, &mut out)
	}
	fn accepts(ty: &Type) -> bool {
		ty.oid() == 1186
	}

	to_sql_checked!();
}


impl TryFromRow for () {
	fn from_row(_row: Row) -> Result<Self, SqlError> {
		Ok(())
	}
}
impl TryFromRow for bool {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for Vec<u8> {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for i64 {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for i32 {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for u32 {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for String {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for NaiveDate {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for NaiveDateTime {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for DateTime<Utc> {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for Interval {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
impl TryFromRow for rust_decimal::Decimal {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
