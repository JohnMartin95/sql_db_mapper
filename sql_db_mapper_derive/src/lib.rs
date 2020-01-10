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
		syn::Data::Enum(_) => panic!("Cannot derive TryFrom<Row> automatically for enum types"),
		syn::Data::Union(_) => panic!("Cannot derive TryFrom<Row> automatically for union types"),
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
			panic!("Cannot derive TryFrom<Row> automatically for unit types");
		},
	};

	let expanded = quote! {
		impl #impl_generics core::convert::TryFrom<Row> for #name #ty_generics #where_clause {
			type Error = SqlError;
			fn try_from(row: Row) -> core::result::Result<Self, Self::Error> {
				#from_row_code
			}
		}
	};

	expanded.into()
}
