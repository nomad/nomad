use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Token, braced, parse_macro_input, token};

pub(crate) fn fs(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let MockFs { root } = parse_macro_input!(input as MockFs);
    quote! { ::mock::fs::MockFs::new(#root) }.into()
}

struct MockFs {
    root: Directory,
}

struct Directory {
    children: Vec<(NodeName, Node)>,
}

struct File {
    contents: Expr,
}

struct Symlink {
    contents: Expr,
}

enum NodeName {
    Ident(Ident),
    Lit(LitStr),
    Reference(syn::ExprReference),
}

enum Node {
    File(File),
    Directory(Directory),
    Symlink(Symlink),
}

impl Parse for MockFs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Allow both `fs!({})` and `fs! {}`.
        let inner = if input.peek(token::Brace) {
            Directory::parse(input)?
        } else {
            let tokens = input.parse::<TokenStream>()?;
            syn::parse2::<Directory>(quote!({ #tokens }))?
        };

        Ok(Self { root: inner })
    }
}

impl Parse for Directory {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        braced!(content in input);

        let mut children = Vec::new();

        while !content.is_empty() {
            let child_name = content.parse::<NodeName>()?;
            let child = content.parse::<Node>()?;
            children.push((child_name, child));
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self { children })
    }
}

impl Parse for NodeName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            input.parse::<LitStr>().map(Self::Lit)
        } else if input.peek(Ident) {
            input.parse::<Ident>().map(Self::Ident)
        } else if input.peek(Token![&]) {
            input.parse::<syn::ExprReference>().map(Self::Reference)
        } else {
            Err(input.error("invalide node name"))
        }
    }
}

impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![:]) {
            input.parse::<Token![:]>().expect("just checked");

            if input.peek(token::Brace) {
                Directory::parse(input).map(Self::Directory)
            } else {
                let contents = input.parse::<Expr>()?;
                Ok(Self::File(File { contents }))
            }
        } else if input.peek(Token![->]) {
            input.parse::<Token![->]>().expect("just checked");
            let contents = input.parse::<Expr>()?;
            Ok(Self::Symlink(Symlink { contents }))
        } else {
            Err(input.error("expected `:` or `->`"))
        }
    }
}

impl ToTokens for Directory {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let variable_name = Ident::new("__dir", Span::call_site());

        let mut definition = quote! {
            let mut #variable_name = ::mock::fs::DirectoryInner::new();
        };

        for (child_name, child) in self.children.iter() {
            definition.extend(quote! {
                #variable_name.insert_child(
                    <&::mock::NodeName>::try_from(#child_name).unwrap(),
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
        quote! { ::mock::fs::FileInner::new(#contents) }.to_tokens(tokens);
    }
}

impl ToTokens for Symlink {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let contents = &self.contents;
        quote! { ::mock::fs::SymlinkInner::new(#contents) }.to_tokens(tokens);
    }
}

impl ToTokens for NodeName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            NodeName::Ident(name) => name.to_tokens(tokens),
            NodeName::Lit(name) => name.to_tokens(tokens),
            NodeName::Reference(name) => name.to_tokens(tokens),
        }
    }
}

impl ToTokens for Node {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::File(file) => file.to_tokens(tokens),
            Self::Directory(dir) => dir.to_tokens(tokens),
            Self::Symlink(symlink) => symlink.to_tokens(tokens),
        }
    }
}
