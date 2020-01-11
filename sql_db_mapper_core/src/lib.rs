//! Helper types and functions for auto-generateed psql database wrappers


/// Contains and reexports types that callers of the wrapped db would need to use it

pub use tokio_postgres::{
	Client,
	NoTls,
	Error as SqlError,
	row::Row,
};
pub use postgres_types::{
	to_sql_checked,
	FromSql,
	ToSql,
	IsNull,
	Type,
};
pub use std::error::Error;
pub use rust_decimal::{
	Decimal,
	prelude::ToPrimitive
};
pub use chrono::{NaiveDateTime, NaiveDate, NaiveTime, DateTime, Utc, Duration};

pub trait TryFromRow: Sized {
	fn from_row(row : Row) -> Result<Self, SqlError>;
}


#[derive(Debug, Clone)]
pub struct Interval {
	pub dur : Duration
}
impl<'a> FromSql<'a> for Interval {
	fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let x = i64::from_sql(&Type::INT4, &raw[0..8])?;
		Ok(Interval{ dur : Duration::microseconds(x) })
	}
	fn accepts(ty: &Type) -> bool {
		ty.oid() == 1186
	}
}

pub mod exports {
	pub use super::TryFromRow;
}

#[derive(Debug, Clone)]
pub struct EnumParseError {
	typ : &'static str,
	variant : String
}
impl EnumParseError {
	pub fn new(typ : &'static str, variant : String) -> EnumParseError {
		EnumParseError { typ, variant }
	}
}
impl Error for EnumParseError {}
impl std::fmt::Display for EnumParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f,"Invalid Enum Variant '{}' for type '{}'", self.variant, self.typ)
	}
}

//TODO forgot that these 'need' impls to make implementaion of function wrappers easier
// consider switching back to FromRow-like trait or looking at other solutions

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
impl TryFromRow for Decimal {
	fn from_row(row: Row) -> Result<Self, SqlError> {
		row.try_get(0)
	}
}
