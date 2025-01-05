use smallvec::SmallVec;

use crate::Name;

/// TODO: docs.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Namespace {
    action: Option<Name>,
    modules: SmallVec<[Name; 2]>,
}

impl Namespace {
    /// TODO: docs.
    #[inline]
    pub fn action(&self) -> Option<Name> {
        self.action
    }

    /// TODO: docs.
    #[inline]
    pub fn components(&self) -> impl Iterator<Item = Name> + '_ {
        self.modules().chain(self.action)
    }

    /// TODO: docs.
    #[inline]
    pub fn modules(&self) -> impl Iterator<Item = Name> + '_ {
        self.modules.iter().copied()
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn pop(&mut self) {
        self.modules.pop();
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn push_module(&mut self, module_name: Name) {
        self.modules.push(module_name);
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn set_action(&mut self, action_name: Name) {
        debug_assert!(self.action.is_none());
        self.action = Some(action_name);
    }
}
