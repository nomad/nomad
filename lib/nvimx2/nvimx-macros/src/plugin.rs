use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[inline]
pub(crate) fn plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fun = parse_macro_input!(item as ItemFn);
    let fun_name = &fun.sig.ident;
    let fun_body = &fun.block;

    quote! {
        #[::nvimx2::neovim::oxi::plugin(nvim_oxi = ::nvimx2::neovim::oxi)]
        fn #fun_name() -> ::nvimx2::neovim::oxi::Dictionary {
            let __plugin = #fun_body;
            let __backend: ::nvimx2::neovim::Neovim = ::nvimx2::backend::Backend::init();
            ::nvimx2::plugin::Plugin::api(__plugin, __backend).into()
        }
    }
    .into()
}
