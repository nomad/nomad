//! .

mod fs;

use proc_macro::TokenStream;

/// TODO: docs.
#[proc_macro]
pub fn fs(input: TokenStream) -> TokenStream {
    fs::fs(input)
}
