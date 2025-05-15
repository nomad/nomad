use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;
use syn::parse::Nothing;
use syn::spanned::Spanned;

#[inline]
pub(crate) fn test(
    attr: proc_macro::TokenStream,
    item: ItemFn,
) -> syn::Result<TokenStream> {
    syn::parse::<Nothing>(attr)?;

    if item.sig.inputs.len() != 1 {
        return Err(syn::Error::new(
            item.sig.ident.span(),
            "expected exactly one argument",
        ));
    }

    let asyncness = &item.sig.asyncness;
    let test_name = &item.sig.ident;
    let test_body = &item.block;
    let test_output = &item.sig.output;

    let ctx_name = match item.sig.inputs.first().expect("just checked") {
        syn::FnArg::Typed(arg) => match &*arg.pat {
            syn::Pat::Ident(pat_ident) => &pat_ident.ident,
            _ => {
                return Err(syn::Error::new(
                    arg.pat.span(),
                    "expected a named function argument",
                ));
            },
        },
        syn::FnArg::Receiver(arg) => {
            return Err(syn::Error::new(
                arg.self_token.span,
                "expected a named function argument, not self",
            ));
        },
    };

    let ctx_ty = if asyncness.is_some() {
        quote! { &mut ::ed::AsyncCtx<'_, ::neovim::Neovim> }
    } else {
        quote! { &mut ::ed::EditorCtx<'_, ::neovim::Neovim> }
    };

    let augroup_name = test_name.to_string();

    Ok(quote! {
        #[::neovim::oxi::test(nvim_oxi = ::neovim::oxi)]
        #asyncness fn #test_name() #test_output {
            #[inline]
            #asyncness fn inner(#ctx_name: #ctx_ty) #test_output {
                #test_body
            }
            let neovim = ::neovim::Neovim::init(#augroup_name);
            ::ed::backend::Backend::with_ctx(neovim, inner)
        }
    })
}
