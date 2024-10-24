use std::vec::IntoIter;

use nvim_oxi::api::types;

/// TODO: docs.
pub struct CommandArgs {
    inner: IntoIter<String>,
}

impl CommandArgs {
    /// TODO: docs.
    pub fn as_slice(&self) -> &[String] {
        self.inner.as_slice()
    }

    /// TODO: docs.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// TODO: docs.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// TODO: docs.
    pub fn pop_front(&mut self) -> Option<String> {
        self.inner.next()
    }
}

impl From<types::CommandArgs> for CommandArgs {
    fn from(args: types::CommandArgs) -> Self {
        Self { inner: args.fargs.into_iter() }
    }
}
