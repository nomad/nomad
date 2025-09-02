use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[inline]
pub(crate) fn plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fun = parse_macro_input!(item as ItemFn);

    let fun_name = &fun.sig.ident;
    let fun_body = &fun.block;
    let plugin_name = fun_name.to_string();

    quote! {
        #[::neovim::oxi::plugin(nvim_oxi = ::neovim::oxi)]
        fn #fun_name() -> ::neovim::oxi::Dictionary {
            // Guard against the user resetting package.loaded.<plugin> to nil
            // after 'require'ing the plugin for the first time.
            ::std::thread_local! {
                static API: ::core::cell::LazyCell::<::neovim::oxi::Dictionary>
                    = ::core::cell::LazyCell::new(|| {
                        let plugin = #fun_body;
                        let neovim = ::neovim::Neovim::new_plugin(#plugin_name);
                        ::editor::module::Plugin::api(plugin, neovim).into()
                    });
            };
            API.with(|api| ::core::cell::LazyCell::force(api).clone())
        }
    }
    .into()
}
