use core::fmt::{Display, Formatter, Result as FmtResult};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, LitStr};

const MIN_LENGTH: usize = 2;

const MAX_LENGTH: usize = 64;

pub fn action_name(name: LitStr) -> Result<TokenStream, Error> {
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

    for ch in name_str.chars() {
        if ch.is_ascii_whitespace() {
            return Err(Error::new_spanned(name, ContainsWhitespace));
        }
        if ch.is_ascii_digit() {
            return Err(Error::new_spanned(name, ContainsDigit));
        }
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
            return Err(Error::new_spanned(name, ContainsOther));
        }
    }

    Ok(quote! { ::nomad::ActionName::from_str(#name) })
}

struct ContainsDigit;

impl Display for ContainsDigit {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "action name cannot contain a digit")
    }
}

struct ContainsOther;

impl Display for ContainsOther {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "action name can only contain lowercase ASCII letters, digits \
             and dashes"
        )
    }
}

struct ContainsUnicode;

impl Display for ContainsUnicode {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "action name must be ASCII")
    }
}

struct ContainsWhitespace;

impl Display for ContainsWhitespace {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "action name must not contain whitespace characters")
    }
}

struct TooLong;

impl Display for TooShort {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "action name must be at least {} characters long",
            MIN_LENGTH
        )
    }
}

struct TooShort;

impl Display for TooLong {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "action name must be at most {} characters long", MAX_LENGTH)
    }
}
