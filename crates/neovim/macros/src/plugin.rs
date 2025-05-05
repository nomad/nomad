use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[inline]
pub(crate) fn plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fun = parse_macro_input!(item as ItemFn);
    let fun_name = &fun.sig.ident;
    let fun_body = &fun.block;

    quote! {
        #[::neovim::oxi::plugin(nvim_oxi = ::neovim::oxi)]
        fn #fun_name() -> ::neovim::oxi::Dictionary {
            let __plugin = #fun_body;
            let __backend = ::neovim::Neovim::init();
            ::ed::plugin::Plugin::api(__plugin, __backend).into()
        }
    }
    .into()
}
