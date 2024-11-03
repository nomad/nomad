use fs::{FsNodeKind, FsNodeName};

use crate::Marker;

/// TODO: docs.
pub struct Git;

impl Git {
    const GIT_DIR: &'static str = ".git";
}

impl Marker for Git {
    fn matches(
        &self,
        fs_node_name: &FsNodeName,
        fs_node_kind: FsNodeKind,
    ) -> bool {
        fs_node_kind.is_directory() && fs_node_name.as_str() == Self::GIT_DIR
    }
}
