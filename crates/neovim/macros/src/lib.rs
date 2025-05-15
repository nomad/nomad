//! TODO: docs.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod plugin;
mod test;

/// TODO: docs.
#[proc_macro_attribute]
pub fn plugin(attr: TokenStream, item: TokenStream) -> TokenStream {
    plugin::plugin(attr, item)
}

/// TODO: docs.
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemFn);
    test::test(attr, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
