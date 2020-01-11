extern crate proc_macro;

use proc_macro2::{
	TokenStream,
};

use quote::quote;

use syn::{
	parse_macro_input,
	DeriveInput,
};

#[proc_macro_derive(TryFromRow)]
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
					fn from_row(row: Row) -> ::core::result::Result<Self, SqlError> {
						row.try_get(0)
					}
				}
			}.into();
		},
		syn::Data::Union(_) => panic!("Cannot derive TryFromRow automatically for union types"),
	};

	let from_row_code = match fields {
		syn::Fields::Named(_) => {
			let tmp : TokenStream = fields
				.iter()
				.map(|v| v.ident.as_ref().unwrap())
				.enumerate()
				.map(|(i,v)| {
					quote!{ #v : row.try_get(#i)?, }
				}).collect();
			quote!{ Self { #tmp } }
		},
		syn::Fields::Unnamed(_) => {
			let tmp : TokenStream = fields
				.iter()
				.enumerate()
				.map(|(i,_v)| {
					quote!{ row.try_get(#i)?, }
				}).collect();
			quote!{ Self ( #tmp ) }
		},
		syn::Fields::Unit => {
			return quote! {
				impl #impl_generics TryFromRow for #name #ty_generics #where_clause {
					fn from_row(_row: Row) -> ::core::result::Result<Self, SqlError> {
						Ok(Self)
					}
				}
			}.into();
		},
	};

	let expanded = quote! {
		impl #impl_generics TryFromRow for #name #ty_generics #where_clause {
			fn from_row(row: Row) -> ::core::result::Result<Self, SqlError> {
				#from_row_code
			}
		}
	};

	expanded.into()
}
