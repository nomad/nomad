use smallvec::{SmallVec, smallvec};

use crate::notify::Name;

/// TODO: docs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    names: SmallVec<[Name; 2]>,
}

impl ModulePath {
    /// TODO: docs.
    #[inline]
    pub fn names(&self) -> impl ExactSizeIterator<Item = Name> + '_ {
        self.names.iter().copied()
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn new(base_module: Name) -> Self {
        Self { names: smallvec![base_module] }
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn push(&mut self, module_name: Name) {
        self.names.push(module_name);
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn pop(&mut self) {
        self.names.pop();
    }
}
