use super::{Row, SqlError};


/// Converts from a [`tokio_postgres::Row`]. Implementations are provided for rows that contain only a single value of several types that implement [`FromSql`] (currently no check is done that the row only contained one value)
///
/// [`TryFromRow`]: ./trait.TryFromRow.html
/// [`tokio_postgres::Row`]: https://docs.rs/tokio-postgres/0.7/tokio_postgres/row/struct.Row.html
/// [`FromSql`]: https://docs.rs/postgres-types/0.2/postgres_types/trait.FromSql.html
pub trait TryFromRow: Sized {
	fn from_row(row: &Row) -> Result<Self, SqlError>;
}
// std types that have FromSql implementations
impl TryFromRow for () {
	fn from_row(_row: &Row) -> Result<Self, SqlError> {
		Ok(())
	}
}

/// Provides an implementation of [`TryFromRow`] for a given type that implements [`FromSql`]
///
/// Does not check if the row contains more than one value (i.e. a row containing 3 columns with the first being a bool will be directly convertable to a bool)
///
/// [`TryFromRow`]: ./trait.TryFromRow.html
/// [`FromSql`]: https://docs.rs/postgres-types/0.2/postgres_types/trait.FromSql.html
macro_rules! try_from_row {
	($impl_type:ty) => {
		impl TryFromRow for $impl_type {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				row.try_get(0)
			}
		}
		impl TryFromRow for Option<$impl_type> {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				row.try_get(0)
			}
		}
		impl TryFromRow for Vec<$impl_type> {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				row.try_get(0)
			}
		}
		impl TryFromRow for Vec<Option<$impl_type>> {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				row.try_get(0)
			}
		}
		impl TryFromRow for Option<Vec<$impl_type>> {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				row.try_get(0)
			}
		}
		impl TryFromRow for Option<Vec<Option<$impl_type>>> {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				row.try_get(0)
			}
		}
	};
}

try_from_row!(bool);
try_from_row!(i8);
try_from_row!(i16);
try_from_row!(i32);
try_from_row!(u32);
try_from_row!(i64);
try_from_row!(f32);
try_from_row!(f64);
try_from_row!(String);
try_from_row!(Vec<u8>);
try_from_row!(std::collections::HashMap<String, Option<String>>);
try_from_row!(std::time::SystemTime);
try_from_row!(std::net::IpAddr);


// Provide auto implementations for tuples (usefule for when doing quick+dirty sql)
use postgres_types::FromSqlOwned;
macro_rules! try_from_tuple {
	($($typ_name:ident),*, $($number:literal),*) => {
		impl< $($typ_name:FromSqlOwned),* > TryFromRow for ($($typ_name),*,) {
			fn from_row(row: &Row) -> Result<Self, SqlError> {
				Ok((
					$(row.try_get($number)?),*,
				))
			}
		}
	};
}
try_from_tuple!(A, 1);
try_from_tuple!(A, B, 1, 2);
try_from_tuple!(A, B, C, 1, 2, 3);
try_from_tuple!(A, B, C, D, 1, 2, 3, 4);
try_from_tuple!(A, B, C, D, E, 1, 2, 3, 4, 5);
try_from_tuple!(A, B, C, D, E, F, 1, 2, 3, 4, 5, 6);
try_from_tuple!(A, B, C, D, E, F, G, 1, 2, 3, 4, 5, 6, 7);
try_from_tuple!(A, B, C, D, E, F, G, H, 1, 2, 3, 4, 5, 6, 7, 8);
try_from_tuple!(A, B, C, D, E, F, G, H, I, 1, 2, 3, 4, 5, 6, 7, 8, 9);
try_from_tuple!(A, B, C, D, E, F, G, H, I, J, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
try_from_tuple!(A, B, C, D, E, F, G, H, I, J, K, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
try_from_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);


// bit_vec
#[cfg(feature = "with-bit-vec-0_6")]
try_from_row!(bit_vec::BitVec);

// chrono
#[cfg(feature = "with-chrono-0_4")]
mod chrono_impls {
    use super::*;
	try_from_row!(chrono::NaiveDateTime);
	try_from_row!(chrono::DateTime<chrono::Utc>);
	try_from_row!(chrono::DateTime<chrono::Local>);
	try_from_row!(chrono::DateTime<chrono::FixedOffset>);
	try_from_row!(chrono::NaiveDate);
	try_from_row!(chrono::NaiveTime);
}

// eui48
#[cfg(feature = "with-eui48-0_4")]
try_from_row!(eui48::MacAddress);

// geo_types
#[cfg(feature = "geo-types")]
mod geo_types_impls {
	use super::*;
	try_from_row!(geo_types::Point<f64>);
	try_from_row!(geo_types::Rect<f64>);
	try_from_row!(geo_types::LineString<f64>);
}

// rust_decimal
#[cfg(feature = "rust_decimal")]
try_from_row!(rust_decimal::Decimal);

// serde_json
#[cfg(feature = "serde_json")]
try_from_row!(serde_json::Value);

// time
#[cfg(feature = "with-time-0_2")]
mod time_impls {
	use super::*;
	try_from_row!(time::PrimitiveDateTime);
	try_from_row!(time::OffsetDateTime);
	try_from_row!(time::Date);
	try_from_row!(time::Time);

}

// uuid
#[cfg(feature = "uuid")]
try_from_row!(uuid::Uuid);
