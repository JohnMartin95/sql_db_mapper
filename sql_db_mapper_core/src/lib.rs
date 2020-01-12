//! Helper types and functions for auto-generateed psql database wrappers


/// Contains and reexports types that callers of the wrapped db would need to use it
#[cfg(not(feature = "sync"))]
pub use tokio_postgres::{
	Client,
	Error as SqlError,
	row::Row,
};

#[cfg(not(feature = "sync"))]
#[allow(unused_imports)]
#[macro_use]
extern crate async_trait;
#[cfg(not(feature = "sync"))]
#[doc(hidden)]
pub use async_trait::*;

#[cfg(feature = "sync")]
pub use postgres::{
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

pub use std::future::Future;

pub use rust_decimal::{
	Decimal,
	prelude::ToPrimitive
};

pub use chrono::{NaiveDateTime, NaiveDate, NaiveTime, DateTime, Utc, Duration};

// #[allow(unused_imports)]
// #[macro_use]
// extern crate sql_db_mapper_derive;
// #[doc(hidden)]
pub use sql_db_mapper_derive::*;

// #[allow(unused_imports)]
// #[macro_use]
// extern crate postgres_derive;
// #[doc(hidden)]
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

//
// use std::collections::{
// 	HashMap,
// 	hash_map::Entry,
// };
// /// Wrapper around Client that caches prepared statements
// ///
// /// Derefs to and is freely convertable to and from Client
// use std::sync::{Arc};
// use parking_lot::Mutex;
//
//
// pub struct CachedClient {
// 	client : Client,
// 	statements : Mutex<HashMap<&'static str, Statement>>,
// }
//
// impl CachedClient {
// 	// pub async fn prepare_cached<'a>(&'a self, stmt_str : &'static str) -> Result<&'a Statement, SqlError> {
// 	// 	let mut lock = self.statements.lock();
// 	// 	match lock.entry(stmt_str) {
// 	// 		Entry::Occupied(v) => Ok(v.into_mut()),
// 	// 		Entry::Vacant(v) => Ok(v.insert(self.client.prepare(stmt_str).await?)),
// 	// 	}
// 	// }
// 	pub fn get_client(self) -> Client {
// 		self.client
// 	}
// 	pub fn clear_cache(&self) {
// 		self.statements.lock().clear()
// 	}
// }
// impl AsRef<Client> for CachedClient {
// 	fn as_ref(&self) -> &Client {
// 		&self.client
// 	}
// }
// impl AsMut<Client> for CachedClient {
// 	fn as_mut(&mut self) -> &mut Client {
// 		self.clear_cache();
// 		&mut self.client
// 	}
// }
// impl From<CachedClient> for Client {
// 	fn from(input : CachedClient) -> Client {
// 		input.client
// 	}
// }
// impl From<Client> for CachedClient {
// 	fn from(input : Client) -> CachedClient {
// 		CachedClient {
// 			client : input,
// 			statements : Mutex::new(HashMap::new()),
// 		}
// 	}
// }

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
