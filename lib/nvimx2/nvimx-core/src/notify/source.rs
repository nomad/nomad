use crate::notify::{ModulePath, Name};

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Source<'path> {
    /// TODO: docs.
    pub module_path: &'path ModulePath,
    /// TODO: docs.
    pub action_name: Option<Name>,
}
