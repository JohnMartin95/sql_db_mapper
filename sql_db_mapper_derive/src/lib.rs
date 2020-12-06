#![forbid(unsafe_code)]
//! Contains a derive macro for [`TryFromRow`] which converts from a [`tokio-postgres::Row`]
//!
//! When feature `full` is enabled also contains an macro [`sql_db_mapper`] which is simply a macro version of the CLI provided by the [crate of the same name]
//!
//! [`TryFromRow`]: https://docs.rs/sql_db_mapper_core/0.0.4/sql_db_mapper_core/trait.TryFromRow.html
//! [`sql_db_mapper`]: ./macro.sql_db_mapper.html
//! [crate of the same name]: https://docs.rs/sql_db_mapper/0.0.4/sql_db_mapper/
//! [`tokio-postgres::Row`]: https://docs.rs/tokio-postgres/0.5.2/tokio_postgres/row/struct.Row.html

extern crate proc_macro;

use proc_macro2::TokenStream;

use quote::quote;

use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(TryFromRow)]
/// A derive macro for [`TryFromRow`] which converts from a [`tokio-postgres::Row`]
///
/// [`TryFromRow`]: https://docs.rs/sql_db_mapper_core/0.0.4/sql_db_mapper_core/trait.TryFromRow.html
/// [`tokio-postgres::Row`]: https://docs.rs/tokio-postgres/0.5.2/tokio_postgres/row/struct.Row.html
pub fn try_from_tokio_postgres_row(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	// get the name of the type we want to implement the trait for
	let name = &input.ident;
	let generics = input.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let fields = match input.data {
		syn::Data::Struct(x) => x.fields,
		syn::Data::Enum(_) => {
			return quote! {
				impl #impl_generics TryFromRow for #name #ty_generics #where_clause {
					fn from_row(row: &Row) -> ::core::result::Result<Self, SqlError> {
						row.try_get(0)
					}
				}
			}
			.into();
		},
		syn::Data::Union(_) => panic!("Cannot derive TryFromRow automatically for union types"),
	};

	let from_row_code = match fields {
		syn::Fields::Named(_) => {
			let tmp: TokenStream = fields
				.iter()
				.map(|v| v.ident.as_ref().unwrap())
				.enumerate()
				.map(|(i, v)| {
					quote! { #v : row.try_get(#i)?, }
				})
				.collect();
			quote! { Ok(Self { #tmp }) }
		},
		syn::Fields::Unnamed(_) => {
			let tmp: TokenStream = fields
				.iter()
				.enumerate()
				.map(|(i, _v)| {
					quote! { row.try_get(#i)?, }
				})
				.collect();
			quote! { Ok(Self ( #tmp )) }
		},
		syn::Fields::Unit => {
			return quote! {
				impl #impl_generics TryFromRow for #name #ty_generics #where_clause {
					fn from_row(_row: &Row) -> ::core::result::Result<Self, SqlError> {
						Ok(Self)
					}
				}
			}
			.into();
		},
	};

	let expanded = quote! {
		impl #impl_generics TryFromRow for #name #ty_generics #where_clause {
			fn from_row(row: &Row) -> ::core::result::Result<Self, SqlError> {
				#from_row_code
			}
		}
	};

	expanded.into()
}