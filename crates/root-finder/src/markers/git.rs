use abs_path::NodeName;
use fs::NodeKind;

use crate::Marker;

/// TODO: docs.
pub struct Git;

impl Git {
    const GIT_DIR: &'static str = ".git";
}

impl Marker for Git {
    fn matches(&self, node_name: &NodeName, node_kind: NodeKind) -> bool {
        node_kind.is_dir() && node_name.as_str() == Self::GIT_DIR
    }
}
