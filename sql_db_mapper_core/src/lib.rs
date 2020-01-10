//! Helper types and functions for auto-generateed psql database wrappers


/// Contains and reexports types that callers of the wrapped db would need to use it

pub use tokio_postgres::{
	Client,
	NoTls,
	Error as SqlError,
	row::Row,
	types::{
		to_sql_checked,
		FromSql,
		ToSql,
		IsNull,
		Type,
	},
};
pub use std::error::Error;
pub use rust_decimal::{
	Decimal,
	prelude::ToPrimitive
};
pub use chrono::{NaiveDateTime, NaiveDate, NaiveTime, DateTime, Utc, Duration};


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
