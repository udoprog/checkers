extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::num::NonZeroUsize;

/// Marks a function to be run as a test in a checkers test suite.
/// 
/// # Examples
/// 
/// ```rust
/// # use checkers_macros as checkers;
/// #
/// #[checkers::test]
/// fn test_leaky_box() {
///     let _ = Box::into_raw(Box::new(42));
/// }
/// ```
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let body = &input.block;
    let attrs = &input.attrs;
    let vis = input.vis;

    for attr in attrs {
        if attr.path.is_ident("test") {
            let msg = "second test attribute is supplied";
            return syn::Error::new_spanned(&attr, msg)
                .to_compile_error()
                .into();
        }
    }

    if !input.sig.inputs.is_empty() {
        let msg = "the test function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.inputs, msg)
            .to_compile_error()
            .into();
    }

    let mut capacity = NonZeroUsize::new(1024).unwrap();

    for arg in args {
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(namevalue)) = arg {
            let ident = namevalue.path.get_ident();
            if ident.is_none() {
                let msg = "Must have specified ident";
                return syn::Error::new_spanned(namevalue.path, msg).to_compile_error().into();
            }
            match ident.unwrap().to_string().to_lowercase().as_str() {
                "capacity" => {
                    match &namevalue.lit {
                        syn::Lit::Int(expr) => {
                            capacity = match expr.base10_parse::<NonZeroUsize>() {
                                Ok(n) => n,
                                _ => {
                                    return syn::Error::new_spanned(
                                        expr,
                                        "capacity argument is not valid",
                                    ).to_compile_error().into();
                                }
                            }
                        }
                        _ => {
                            return syn::Error::new_spanned(
                                namevalue,
                                "capacity argument must be an int",
                            ).to_compile_error().into();
                        }
                    }
                },
                name => {
                    let msg = format!("Unknown attribute {} is specified", name);
                    return syn::Error::new_spanned(namevalue.path, msg).to_compile_error().into();
                }
            }
        }
    }

    let capacity = capacity.get();

    let result = quote! {
        #[test]
        #(#attrs)*
        #vis fn #name() #ret {
            checkers::STATE.with(|state| {
                state.reserve(#capacity);
                state.with(|| { #body });
                checkers::verify!(state);
            })
        }
    };

    result.into()
}
