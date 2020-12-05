//! Functions for generating rust functions
use super::{
	super::{sql_tree::*, Opt, Tuples},
	format_heck,
	Case::*,
};
use proc_macro2::TokenStream;
use quote::quote;

/// Takes a SQL procedure and turns it into a rust function
///
/// Implemented on a Vec of SqlProc which contains information on all Procs with the same name
///
/// ```ignore
/// // a non overloaded sql proc
/// // the sql string sent to the database
/// const MY_FUNCTION_SQL: &str = "SELECT * FROM \"schema\".\"my_function\"($1,$2)";
/// // Return struct only generated if the procedure returns an anonymous type
/// #[derive(Debug, Clone, TryFromRow, ToSql, FromSql)]
/// pub struct my_functionReturn {=
/// 	pub field0: super::pg_catalog::varchar,
/// 	pub field1: super::schema::typ,
/// }
/// // fn can be sync as well
/// pub async fn my_function(
/// 	// client is & is async and &mut if sync (mirrors client's methods between tokio-postgres and postgres)
/// 	client: &mut Client,
/// 	param0: &super::pg_catalog::varchar,
/// 	param1: &super::pg_catalog::varchar,
/// // if the function did not return a set the return type would be Result<Option<T>, SqlError> instead
/// ) -> Result<Vec<my_functionReturn>, SqlError> {
/// 	/* implementation */
/// }
///
/// // an overloaded sql proc
/// // called like overloaded_function((client, other_params)) i.e. it takes a single tuple as input
/// pub fn overloaded_function<T: overloaded_function::OverloadTrait>(input: T) -> T::Output {
/// 	<T as overloaded_function::OverloadTrait>::tmp(input)
/// }
/// // A private module with a public trait inside is used to hide implementation details
/// mod overloaded_function {
/// 	use super::*;
/// 	would use #[async_trait] in an async mapping
/// 	pub trait OverloadTrait {
/// 		type Output;
/// 		fn tmp(self)-> TokenStream;
/// 	}
/// 	const OVERLOAD_FUNCTION0_SQL: &str = "SELECT * FROM \"schema\".\"overloaded_function\"($1)";
/// 	impl<'a> OverloadTrait for (&'a mut Client, &'a super::pg_catalog::int4) {
/// 		type Output = Result<Option<super::super::pg_catalog::void>, SqlError>;
/// 		fn tmp(self)-> TokenStream {
/// 			/* implementation */
/// 		}
/// 	}
/// 	//impls for other input params
/// }
/// ```
pub fn proc_to_rust(proc: &[SqlProc], opt: &Opt, is_sync: bool) -> TokenStream {
	if proc.len() == 0 {
		if opt.debug {
			println!("Error; retrieved an empty Vec of SqlProcs")
		};
		return quote! {};
	}

	match opt.use_tuples {
		Tuples::ForOverloads => {
			if proc.len() == 1 {
				single_proc_to_rust(&proc[0], &proc[0].name, false, opt, is_sync)
			} else {
				to_many_fns(proc, opt, is_sync)
			}
		},
		Tuples::ForAll => to_many_fns(proc, opt, is_sync),
		Tuples::NoOverloads => {
			if proc.len() == 1 {
				single_proc_to_rust(&proc[0], &proc[0].name, false, opt, is_sync)
			} else {
				if opt.debug {
					println!("Overloaded Proc: '{}' not mapped", proc[0].name)
				};
				quote! {}
			}
		},
		Tuples::OldestOverload => single_proc_to_rust(&proc[0], &proc[0].name, false, opt, is_sync),
	}
}

/// Turns an overloaded SQL function to a rough equicvalent in rust
fn to_many_fns(procs: &[SqlProc], opt: &Opt, is_sync: bool) -> TokenStream {
	let name_type = format_heck(&procs[0].name, opt, SnakeCase);
	let doc_comments = to_overload_doc(&procs, opt);
	let fn_docs = quote! {
		/// This is an overloaded SQL function, it takes one tuple parameter.
		///
		/// Valid input types for this function are:
		#doc_comments
	};

	// output type depending on wether the code is async
	let fn_code = if is_sync {
		quote! {
			#fn_docs
			pub fn #name_type<T:#name_type::OverloadTrait>(input : T) -> T::Output {
				<T as #name_type::OverloadTrait>::tmp(input)
			}
		}
	} else {
		quote! {
			#fn_docs
			pub fn #name_type<T:#name_type::OverloadTrait>(input : T) -> impl Future<Output = T::Output> {
				async {
					<T as #name_type::OverloadTrait>::tmp(input).await
				}
			}
		}
	};

	let (is_async_trait, async_fn) = if is_sync {
		(quote! {}, quote! {})
	} else {
		(
			quote! {
				use async_trait::async_trait;
				#[async_trait]
			},
			quote! { async },
		)
	};

	let trait_impls = procs.iter().enumerate().map(|(i, p)| to_trait_impl(i, p, opt, is_sync));

	quote! {
		#fn_code
		mod #name_type {
			use super::*;
			#is_async_trait
			pub trait OverloadTrait {
				type Output;
				#async_fn fn tmp(self)-> Self::Output;
			}
			#(#trait_impls)*
		}
	}
}

/// For overloaded functions get the function implementation
fn to_trait_impl(index: usize, proc: &SqlProc, opt: &Opt, is_sync: bool) -> TokenStream {
	//build SQL string to call proc
	let new_name = format!("{}{}", proc.name, index);
	single_proc_to_rust(proc, &new_name, true, opt, is_sync)
}
/// gets the type of the input to one variant for an overloaded function
fn to_tuple_type(types: &[TypeAndName], opt: &Opt, is_sync: bool) -> TokenStream {
	let tuple_middle = types.iter().map(|tan| {
		let tmp = tan.typ.to_tokens(opt);
		quote! { &'a #tmp }
	});

	if is_sync {
		quote! { (&'a mut Client, #(#tuple_middle),* ) }
	} else {
		quote! { (&'a Client, #(#tuple_middle),* ) }
	}
}

fn to_tuple_pattern(types: &[TypeAndName], opt: &Opt) -> TokenStream {
	let tuple_middle = types.iter().map(|tan| format_heck(&tan.name, opt, SnakeCase));
	quote! {
		(client, #(#tuple_middle),* )
	}
}
/// Get a doc comment for an overloaded procedure
fn to_overload_doc(procs: &[SqlProc], opt: &Opt) -> TokenStream {
	procs
		.iter()
		.map(|v| {
			let name = &v.name;
			let func_parms = v.inputs.as_function_params(opt);
			let ret_type_name = v.outputs.to_tokens(opt).to_string();
			let new_ret_type_name = if v.returns_set {
				format!("Vec<{}>", ret_type_name)
			} else {
				format!("Option<{}>", ret_type_name)
			};
			let doc_comment = format!(
				"{}(( client : &Client, {} )) -> {}",
				name, func_parms, new_ret_type_name
			);
			quote! {
				#[doc = #doc_comment]
			}
		})
		.collect()
}


fn single_proc_to_rust(proc: &SqlProc, name: &str, is_overide: bool, opt: &Opt, is_sync: bool) -> TokenStream {
	let name_type = format_heck(name, opt, SnakeCase);

	//build SQL string to call proc
	let call_string_name = format_heck(&format!("{}_SQL", name), opt, ShoutySnake);

	let call_string = make_call_string(&proc.ns_name, &proc.name, proc.num_args as usize);
	let call_string = quote! { const #call_string_name : &str = #call_string; };

	//get the output type name
	let ret_type_name = if proc.outputs.schema == "pg_catalog" && proc.outputs.name == "record" {
		if opt.debug {
			println!(
				"Cannot make wrapper for procedure {} which returns pg_catalog::record",
				name
			)
		};
		return quote! {};
	} else {
		let typ = proc.outputs.to_tokens(opt);
		quote! { #typ }
	};
	//get the return type properly wrapped in a Vec or Option
	let new_ret_type_name = if proc.returns_set {
		quote! { Vec<#ret_type_name> }
	} else {
		quote! { Option<#ret_type_name> }
	};

	let func_params = proc.inputs.as_function_params(opt);
	let query_params = as_query_params(&proc.inputs.0, opt);

	let (opt_async, opt_await, is_async_trait, client_type) = if is_sync {
		(quote! {}, quote! {}, quote! {}, quote! { &mut Client })
	} else {
		(
			quote! { async },
			quote! { .await },
			quote! { #[async_trait] },
			quote! { &Client },
		)
	};

	//the body of the function
	let body = if proc.returns_set {
		quote! {
			let stmt = client.prepare(#call_string_name)#opt_await?;
			client
				.query(&stmt, &[#query_params])#opt_await?
				.iter()
				.map(#ret_type_name::from_row)
				.collect()
		}
	} else {
		quote! {
			let stmt = client.prepare(#call_string_name)#opt_await?;
			Ok(client
				.query_opt(&stmt, &[#query_params])#opt_await?
				.as_ref()
				.map(#ret_type_name::from_row)
				.transpose()?
			)
		}
	};
	//the wrappings on the body
	let func_text = if is_overide {
		let tuple_type = to_tuple_type(&proc.inputs.0, opt, is_sync);
		let tuple_pattern = to_tuple_pattern(&proc.inputs.0, opt);
		quote! {
			#is_async_trait
			impl<'a> OverloadTrait for #tuple_type {
				type Output = Result<#new_ret_type_name, SqlError>;
				#opt_async fn tmp(self)-> Self::Output {
					let #tuple_pattern = self;
					#body
				}
			}
		}
	} else {
		quote! {
			pub #opt_async fn #name_type(
				client : #client_type,
				#func_params
			) -> Result<#new_ret_type_name, SqlError> {
				#body
			}
		}
	};
	quote! {
		#call_string
		#func_text
	}
}


fn make_call_string(namespace: &str, function: &str, len: usize) -> String {
	let mut ret = format!(r#"SELECT * FROM "{}"."{}"("#, namespace, function);
	for i in 1..len {
		ret += &format!("${},", i);
	}
	ret += &format!("${})", len);
	ret
}

fn as_query_params(inputs: &[TypeAndName], opt: &Opt) -> TokenStream {
	let names = inputs.iter().map(|tan| format_heck(&tan.name, opt, SnakeCase));

	quote! {
		#(#names),*
	}
}
