//! Helper types and functions for auto-generateed psql database wrappers


/// Contains and reexports types that callers of the wrapped db would need to use it
pub mod orm {
	pub use postgres::{Connection, TlsMode};
	pub use rust_decimal::{
		Decimal,
		prelude::ToPrimitive
	};
	pub use chrono::{NaiveDateTime, NaiveDate, NaiveTime, DateTime, Utc, Duration};
	pub use postgres::Result as SqlResult;

	use std::fmt::{self, Display, Formatter};
	use super::exports::*;

	#[derive(Debug, Clone)]
	pub struct Interval {
		pub dur : Duration
	}
	impl FromSql for Interval {
		fn from_sql<'a>(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
			let x = i64::from_sql(&INT4, &raw[0..8])?;
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
	impl Display for EnumParseError {
		fn fmt(&self, f: &mut Formatter) -> fmt::Result {
			write!(f,"Invalid Enum Variant '{}' for type '{}'", self.variant, self.typ)
		}
	}
}

/// reexports elements that the auto-generated code uses
pub mod exports {
	pub use postgres::{
		rows::Row,
		to_sql_checked,
		types::{
			FromSql,
			ToSql,
			IsNull,
			Type,
			INT4,
			TEXT
		}
	};
	pub use std::error::Error;
}
