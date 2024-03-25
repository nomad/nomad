use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn ready(input: DeriveInput) -> TokenStream {
    let symbol_name = &input.ident;

    quote! {
        impl ::nomad::maybe_future::MaybeFuture for #symbol_name {
            type Output = Self;

            type Future = ::core::future::Ready<Self>;

            fn into_enum(self) ->::nomad::maybe_future::MaybeFutureEnum<Self::Future, #symbol_name> {
                ::nomad::maybe_future::MaybeFutureEnum::Ready(self)
            }
        }
    }
}
