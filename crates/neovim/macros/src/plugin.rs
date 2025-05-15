use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[inline]
pub(crate) fn plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fun = parse_macro_input!(item as ItemFn);

    let fun_name = &fun.sig.ident;
    let fun_body = &fun.block;
    let augroup_name = fun_name.to_string();

    quote! {
        #[::neovim::oxi::plugin(nvim_oxi = ::neovim::oxi)]
        fn #fun_name() -> ::neovim::oxi::Dictionary {
            let plugin = #fun_body;
            let neovim = ::neovim::Neovim::init(#augroup_name);
            ::ed::plugin::Plugin::api(plugin, neovim).into()
        }
    }
    .into()
}
