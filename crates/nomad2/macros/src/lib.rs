use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemFn, ItemImpl, LitStr};

mod module_name;

/// A proc macro that turns a string literal into a `ModuleName`.
///
/// If the string is not a valid module name, the macro will generate a compile
/// error with a message explaining the problem.
///
/// # Examples
///
/// ```no_run
/// # use crate::module_name;
/// let name: ModuleName = module_name!("foo");
/// ```
///
/// ```compile_fail
/// # use crate::module_name;
/// // Fails to compile because the string contains a whitespace.
/// let name: ModuleName = module_name!("foo bar");
/// ```
#[proc_macro]
pub fn module_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);

    match module_name::module_name(input) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
