use abs_path::NodeName;
use fs::NodeKind;

/// TODO: docs.
pub trait Marker {
    /// TODO: docs.
    fn matches(&self, fs_node_name: &NodeName, fs_node_kind: NodeKind)
    -> bool;
}

impl<M: Marker> Marker for &M {
    fn matches(
        &self,
        fs_node_name: &NodeName,
        fs_node_kind: NodeKind,
    ) -> bool {
        (*self).matches(fs_node_name, fs_node_kind)
    }
}

impl<M1, M2> Marker for (M1, M2)
where
    M1: Marker,
    M2: Marker,
{
    fn matches(
        &self,
        fs_node_name: &NodeName,
        fs_node_kind: NodeKind,
    ) -> bool {
        let (m1, m2) = self;
        m1.matches(fs_node_name, fs_node_kind)
            || m2.matches(fs_node_name, fs_node_kind)
    }
}
