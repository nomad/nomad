use core::error::Error;
use core::fmt;

#[derive(Debug)]
pub(crate) enum SessionError {}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!();
    }
}

impl Error for SessionError {}
