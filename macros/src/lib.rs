extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::num::NonZeroUsize;

/// Marks a function to be run as a test in a checkers test suite.
///
/// # Attributes
///
/// The `test` macro has the following attributes:
/// * `capacity` - Reserve capacity for the specified number of events
///   beforehand. Checkers will otherwise grow it as necessary using the system
///   allocator directly.
/// * `verify` - Use a custom verification function (see below).
///
/// # Examples
///
/// ```rust
/// #[global_allocator]
/// static CHECKED: checkers::Allocator = checkers::Allocator;
///
/// #[checkers::test]
/// fn test_leaky_box() {
///     let _ = Box::into_raw(Box::new(42));
/// }
/// ```
///
/// Reserve capacity for the specified number of events up front:
///
/// ```rust
/// #[global_allocator]
/// static CHECKED: checkers::Allocator = checkers::Allocator;
///
/// #[checkers::test(capacity = 10_000)]
/// fn test_custom_verify() {
///     for i in 0..1000 {
///         let v = Box::into_raw(vec![1, 2, 3, 4, 5].into_boxed_slice());
///         let _ = unsafe { Box::from_raw(v) };
///     }
/// }
/// ```
///
/// Using a custom verifier:
///
/// ```rust
/// #[global_allocator]
/// static CHECKED: checkers::Allocator = checkers::Allocator;
///
/// fn verify_test_custom_verify(state: &mut checkers::State) {
///    let mut violations = Vec::new();
///    state.validate(&mut violations);
///    assert_eq!(1, violations.len());
///    assert!(violations[0].is_dangling_region(|region| region.size == 20 && region.align == 4));
/// }
///
/// #[checkers::test(verify = "verify_test_custom_verify")]
/// fn test_custom_verify() {
///     let _ = Box::into_raw(vec![1, 2, 3, 4, 5].into_boxed_slice());
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
    let mut verify = None::<syn::Ident>;

    for arg in args {
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(namevalue)) = arg {
            let ident = namevalue.path.get_ident();
            if ident.is_none() {
                let msg = "Must have specified ident";
                return syn::Error::new_spanned(namevalue.path, msg)
                    .to_compile_error()
                    .into();
            }
            match ident.unwrap().to_string().to_lowercase().as_str() {
                "capacity" => match &namevalue.lit {
                    syn::Lit::Int(expr) => {
                        capacity = match expr.base10_parse::<NonZeroUsize>() {
                            Ok(n) => n,
                            _ => {
                                return syn::Error::new_spanned(
                                    expr,
                                    "capacity argument is not valid",
                                )
                                .to_compile_error()
                                .into();
                            }
                        }
                    }
                    _ => {
                        return syn::Error::new_spanned(
                            namevalue,
                            "capacity argument must be an int",
                        )
                        .to_compile_error()
                        .into();
                    }
                },
                "verify" => match &namevalue.lit {
                    syn::Lit::Str(expr) => {
                        verify = Some(match expr.parse::<syn::Ident>() {
                            Ok(ident) => ident,
                            Err(..) => {
                                return syn::Error::new_spanned(
                                    expr,
                                    "verify argument is not valid",
                                )
                                .to_compile_error()
                                .into();
                            }
                        });
                    }
                    _ => {
                        return syn::Error::new_spanned(
                            namevalue,
                            "verify argument must be a string",
                        )
                        .to_compile_error()
                        .into();
                    }
                },
                name => {
                    let msg = format!("Unknown attribute {} is specified", name);
                    return syn::Error::new_spanned(namevalue.path, msg)
                        .to_compile_error()
                        .into();
                }
            }
        }
    }

    let capacity = capacity.get();

    let verify = match verify {
        Some(verify) => {
            quote! {
                #verify(state);
            }
        }
        None => quote! {
            checkers::verify!(state);
        },
    };

    let result = quote! {
        #[test]
        #(#attrs)*
        #vis fn #name() #ret {
            checkers::with_state(|s| {
                {
                    let mut s = s.borrow_mut();
                    s.clear();
                    s.reserve(#capacity);
                }

                checkers::with_unmuted(|| #body);

                let state = &mut *s.borrow_mut();
                #verify
            });
        }
    };

    result.into()
}
