use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token, braced, parse_macro_input, token};

pub(crate) fn fs(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let root = parse_macro_input!(input as RootDirectory);
    quote! { ::ed::mock::fs::MockFs::new(#root) }.into()
}

struct RootDirectory {
    inner: Directory,
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

impl Parse for RootDirectory {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Allow both `fs!({})` and `fs! {}`.
        let inner = if input.peek(token::Brace) {
            Directory::parse(input)?
        } else {
            let tokens = input.parse::<TokenStream>()?;
            syn::parse2::<Directory>(quote!({ #tokens }))?
        };

        Ok(Self { inner })
    }
}

impl Parse for Directory {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        braced!(content in input);

        let mut children = Vec::new();

        while !content.is_empty() {
            let child_name = content.parse::<Expr>()?;
            content.parse::<Token![:]>()?;
            let child = content.parse::<FsNode>()?;
            children.push((child_name, child));
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self { children })
    }
}

impl Parse for File {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Expr::parse(input).map(|contents| Self { contents })
    }
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

impl ToTokens for RootDirectory {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.inner.to_tokens(tokens);
    }
}

impl ToTokens for Directory {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let variable_name = Ident::new("__dir", Span::call_site());

        let mut definition = quote! {
            let mut #variable_name = ::ed::mock::fs::DirectoryInner::new();
        };

        for (child_name, child) in self.children.iter() {
            definition.extend(quote! {
                #variable_name.insert_child(
                    <&::ed::fs::NodeName>::try_from(#child_name).unwrap(),
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
        let contents = &self.contents;
        quote! { ::ed::mock::fs::FileInner::new(#contents) }.to_tokens(tokens);
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
