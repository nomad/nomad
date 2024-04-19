use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Block, ItemFn, LitStr, Token};

#[inline]
pub fn test(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    item
}
