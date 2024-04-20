use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Block, Ident, ItemFn, LitInt, Signature};

#[inline]
pub fn test(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn { sig, block, .. } = parse_macro_input!(item as syn::ItemFn);

    let test_body = test_body(&sig, &block);

    let test_name = sig.ident;

    let test_fn_name =
        Ident::new(&format!("__test_fn_{}", test_name), test_name.span());

    quote! {
        #[::nomad::nvim::test(nvim_oxi = ::nomad::nvim, test_fn = #test_fn_name)]
        fn #test_name() {}

        fn #test_fn_name() {
            #test_body
        }
    }
    .into()
}

fn test_body(
    test_sig: &Signature,
    test_body: &Block,
) -> proc_macro2::TokenStream {
    let seed = Seed::new();

    let define_seed = seed.definition();

    let err_msg = Ident::new("err_msg", Span::call_site());

    let eprintln = if let Seed::None = seed {
        quote! { eprintln!("{:?}", #err_msg) }
    } else {
        let seed_name = seed.name();
        quote! { eprintln!("failed on seed {}: {:?}", #seed_name, #err_msg) }
    };

    let into_result = into_result();

    let test_fn = Ident::new("__test_fn", Span::call_site());

    let unwind_body = unwind_body(&seed, &test_fn);

    let inputs = &test_sig.inputs;

    let output = &test_sig.output;

    quote! {
        #into_result

        fn #test_fn(#inputs) #output {
            #test_body
        }

        #define_seed

        let result = ::std::panic::catch_unwind(|| {
            #unwind_body
        });

        let #err_msg: &dyn ::core::fmt::Debug = match &result {
            Ok(Ok(())) => ::std::process::exit(0),
            Ok(Err(err)) => err,
            Err(panic) => panic,
        };

        #eprintln;
        ::std::process::exit(1);
    }
}

fn unwind_body(seed: &Seed, test_fn: &Ident) -> proc_macro2::TokenStream {
    let seed = seed.name();

    quote! {
        let mut generator = ::nomad::tests::Generator::new(#seed);
        let res = #test_fn(&mut generator);
        __IntoResult::into_result(res)
    }
}

enum Seed {
    None,
    RandomlyGenerated,
    Specified(LitInt),
    FromEnv,
}

impl Seed {
    /// Returns the `let seed = ...;` definition.
    fn definition(&self) -> proc_macro2::TokenStream {
        match self {
            Self::None => quote! {},

            Self::RandomlyGenerated => quote! {
                let seed = ::nomad::tests::random_seed();
            },

            Self::Specified(seed) => {
                quote! {
                    let seed = #seed;
                }
            },

            Self::FromEnv => {
                quote! {
                    let seed = {
                        let Some(env) = ::std::env::var_os("SEED") else {
                            eprintln!("$SEED not set");
                            ::std::process::exit(1);
                        };
                        let Some(str) = env.to_str() else {
                            eprintln!("$SEED is not UTF-8");
                            ::std::process::exit(1);
                        };
                        match str.parse::<u64>() {
                            Ok(seed) => seed,
                            Err(err) => {
                                eprintln!("couldn't parse $SEED: {err}");
                                ::std::process::exit(1);
                            }
                        };
                    };
                }
            },
        }
    }

    fn name(&self) -> Ident {
        Ident::new("seed", Span::call_site())
    }

    fn new() -> Self {
        Self::RandomlyGenerated
    }
}

/// Defines the `__IntoResult` trait and implements it for `()` and `Result<(),
/// E>` where `E` is `Debug`.
fn into_result() -> proc_macro2::TokenStream {
    quote! {
        trait __IntoResult {
            type Error: ::core::fmt::Debug;
            fn into_result(self) -> ::core::result::Result<(), Self::Error>;
        }
        impl __IntoResult for () {
            type Error = ::core::convert::Infallible;
            fn into_result(self) -> ::core::result::Result<(), Self::Error> {
                Ok(())
            }
        }
        impl<E: ::core::fmt::Debug> __IntoResult for ::core::result::Result<(), E> {
            type Error = E;
            fn into_result(self) -> ::core::result::Result<(), E> {
                self
            }
        }
    }
}
