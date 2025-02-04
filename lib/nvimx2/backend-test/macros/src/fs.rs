use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, parse_macro_input, token};

pub(crate) fn fs(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let root = parse_macro_input!(input as Directory);
    quote! { ::nvimx2::tests::fs::TestFs::new(#root) }.into()
}

struct Directory {
    children: Vec<(Expr, FsNode)>,
}

struct File {
    contents: Expr,
}

enum FsNode {
    File(File),
    Directory(Directory),
}

impl Parse for FsNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Brace) {
            Directory::parse(input).map(Self::Directory)
        } else {
            File::parse(input).map(Self::File)
        }
    }
}

impl Parse for Directory {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

impl Parse for File {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Expr::parse(input).map(|contents| Self { contents })
    }
}

impl ToTokens for Directory {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let variable_name = Ident::new("__dir", Span::call_site());

        let mut definition = quote! {
            let mut #variable_name = ::nvimx2::tests::fs::TestDirectory::new();
        };

        for (child_name, child) in self.children.iter() {
            definition.extend(quote! {
                #variable_name.insert_child(
                    <&::nvimx2::fs::FsNodeName>::try_from(#child_name).unwrap(),
                    #child,
                );
            });
        }

        definition.extend(quote! {
            #variable_name
        });

        quote! {{ #definition }}.to_tokens(tokens);
    }
}

impl ToTokens for File {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! { ::nvimx2::tests::fs::TestFile::new(#self.contents) }
            .to_tokens(tokens);
    }
}

impl ToTokens for FsNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::File(file) => file.to_tokens(tokens),
            Self::Directory(dir) => dir.to_tokens(tokens),
        }
    }
}
