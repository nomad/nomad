use smallvec::{SmallVec, smallvec};

use crate::notify::Name;

/// TODO: docs.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Namespace {
    names: SmallVec<[Name; 2]>,
}

impl Namespace {
    /// TODO: docs.
    #[inline]
    pub fn names(&self) -> impl ExactSizeIterator<Item = Name> + '_ {
        self.names.iter().copied()
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn new(plugin_name: Name) -> Self {
        Self { names: smallvec![plugin_name] }
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
