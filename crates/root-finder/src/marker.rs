use fs::{FsNodeKind, FsNodeName};

/// TODO: docs.
pub trait Marker {
    /// TODO: docs.
    fn matches(
        &self,
        fs_node_name: &FsNodeName,
        fs_node_kind: FsNodeKind,
    ) -> bool;
}

impl<M1, M2> Marker for (M1, M2)
where
    M1: Marker,
    M2: Marker,
{
    fn matches(
        &self,
        fs_node_name: &FsNodeName,
        fs_node_kind: FsNodeKind,
    ) -> bool {
        let (m1, m2) = self;
        m1.matches(fs_node_name, fs_node_kind)
            || m2.matches(fs_node_name, fs_node_kind)
    }
}
