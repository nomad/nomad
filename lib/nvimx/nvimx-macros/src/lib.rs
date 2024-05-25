//! TODO: docs

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod test;

/// TODO: docs
#[proc_macro_attribute]
pub fn test(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attrs as test::Args);
    let item = parse_macro_input!(input as syn::ItemFn);
    match test::test(args, item) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
