use proc_macro2::TokenStream;
use syn::ItemFn;

/// TODO: docs.
pub(crate) fn test(
    _args: TokenStream,
    item: ItemFn,
) -> syn::Result<TokenStream> {
    todo!();
}
