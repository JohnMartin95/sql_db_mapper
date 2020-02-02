//! Contains a derive macro for [`TryFromRow`] which converts from a tokio-postgres Row
//!
//! When feature `full` is enabled also contains an macro [`sql_db_mapper`] which is simply a macro version of the CLI provided by the [crate of the same name]
//!
//! [`TryFromRow`]: https://docs.rs/sql_db_mapper_core/0.0.4/sql_db_mapper_core/trait.TryFromRow.html
//! [`sql_db_mapper`]: ./macro.sql_db_mapper.html
//! [crate of the same name]: https://docs.rs/sql_db_mapper/0.0.4/sql_db_mapper/

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
/// A derive macro for [`TryFromRow`] which converts from a tokio-postgres Row
///
/// [`TryFromRow`]: https://docs.rs/sql_db_mapper_core/0.0.4/sql_db_mapper_core/trait.TryFromRow.html
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
			quote!{ Ok(Self { #tmp }) }
		},
		syn::Fields::Unnamed(_) => {
			let tmp : TokenStream = fields
				.iter()
				.enumerate()
				.map(|(i,_v)| {
					quote!{ row.try_get(#i)?, }
				}).collect();
			quote!{ Ok(Self ( #tmp )) }
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


#[cfg(feature = "full")]
mod full_derive {
	use syn::{
		punctuated::Punctuated,
		LitStr,
		token::Comma,
		parse::{
			Parse,
			ParseStream,
		},
		Result,
	};

	pub struct MyStruct{
		pub args : Vec<String>
	}
	impl Parse for MyStruct {
		fn parse(input: ParseStream) -> Result<Self> {
			let ast_node : Punctuated<LitStr, Comma> = Punctuated::parse_terminated(input)?;
			let mut args : Vec<_> = ast_node.iter()
				.map(LitStr::value)
				.collect();
			// StructOpt effectively ignores the first argument
			args.insert(0, String::from("sql_db_mapper"));

			Ok(MyStruct{
				args
			})
		}
	}
}

#[cfg(feature = "full")]
/// Creates a module called db containing the mapping for the databases
///
/// For a call to sql_db_mapper like `sql_db_mapper -d -s -f --serde` this macro would be called `sql_db_mapper_derive::sql_db_mapper!("-d", "-s", "-f", "--serde");`
///
/// See [`sql_db_mapper`] for specifics on the options
///
/// This macro connects to the database and performs a lot of code generation on every compilation, it is recommended to instead use the CLI and generate the actual code into a file for both compilation speed and debugging purposes
///
/// Ignores `--dir` flag and the `output` argument
///
/// [`sql_db_mapper`]: https://docs.rs/sql_db_mapper/0.0.4/sql_db_mapper/
#[proc_macro]
pub fn sql_db_mapper(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	use sql_db_mapper::*;
	use ast_convert::ConvertToAst;
	use structopt::StructOpt;

	// let test : MyStruct = parse(item).unwrap();
	let input = parse_macro_input!(item as full_derive::MyStruct);
	let mut opt = Opt::from_iter(input.args);
	//since this is a proc_macro we're generating in just one file
	opt.dir = false;
	opt.output = None;

	if opt.debug {
		println!("{}", opt.get_cargo_toml());
	}

	let mut client = opt.get_client();
	let full_db = client.get_all();
	let tokens = full_db.to_rust_tokens(&opt);
	(quote!{
		pub mod db {
			#tokens
		}
	}).into()
}
