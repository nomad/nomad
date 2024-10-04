use core::fmt::{Display, Formatter, Result as FmtResult};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, LitStr};

const MIN_LENGTH: usize = 2;

const MAX_LENGTH: usize = 16;

pub fn module_name(name: LitStr) -> Result<TokenStream, Error> {
    let name_str = name.value();

    if !name_str.is_ascii() {
        return Err(Error::new_spanned(name, ContainsUnicode));
    }

    if name_str.len() < MIN_LENGTH {
        return Err(Error::new_spanned(name, TooShort));
    }

    if name_str.len() > MAX_LENGTH {
        return Err(Error::new_spanned(name, TooLong));
    }

    if name_str.starts_with(|ch: char| ch.is_ascii_digit()) {
        return Err(Error::new_spanned(name, StartsWithDigit));
    }

    for ch in name_str.chars() {
        if ch.is_ascii_whitespace() {
            return Err(Error::new_spanned(name, ContainsWhitespace));
        }

        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
            return Err(Error::new_spanned(name, ContainsOther));
        }
    }

    Ok(quote! { ::nomad::ModuleName::from_str(#name) })
}

struct ContainsOther;

impl Display for ContainsOther {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "module name can only contain lowercase ASCII letters and digits"
        )
    }
}

struct ContainsUnicode;

impl Display for ContainsUnicode {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "module name must be ASCII")
    }
}

struct ContainsWhitespace;

impl Display for ContainsWhitespace {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "module name must not contain whitespace characters")
    }
}

struct StartsWithDigit;

impl Display for StartsWithDigit {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "module name cannot start with a digit")
    }
}

struct TooLong;

impl Display for TooShort {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "module name must be at least {} characters long",
            MIN_LENGTH
        )
    }
}

struct TooShort;

impl Display for TooLong {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "module name must be at most {} characters long", MAX_LENGTH)
    }
}
