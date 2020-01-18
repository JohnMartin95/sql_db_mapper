//! Helper types and functions for auto-generateed psql database wrappers
//!
//! Reexports the Client and Row type as well of redefining the error type to SqlError
//!
//! Also rexports the underlying database connector as db_frontend for convience

/// Contains and reexports types that callers of the wrapped db would need to use it
#[cfg(not(feature = "sync"))]
pub use tokio_postgres::{
	self as db_frontend,
	Client,
	Error as SqlError,
	row::Row,
};

#[cfg(feature = "sync")]
pub use postgres::{
	self as db_frontend,
	Client,
	Error as SqlError,
	row::Row,
};

pub use postgres_types::{
	FromSql,
	ToSql,
	IsNull,
	Type,
};
use postgres_types::to_sql_checked;

use std::error::Error;

#[cfg(not(feature = "sync"))]
pub use std::future::Future;

pub use rust_decimal::{
	self,
	prelude::*
};
pub use chrono;

use chrono::{NaiveDateTime, NaiveDate, DateTime, Utc, Duration};

pub use sql_db_mapper_derive::*;

pub use postgres_derive::*;


pub trait TryFromRow: Sized {
	fn from_row(row : Row) -> Result<Self, SqlError>;
}


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
		let i = self.dur.num_milliseconds();
		i.to_sql(&Type::INT4, &mut out)
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
