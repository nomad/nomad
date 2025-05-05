use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct AgentId(u64);

impl AgentId {
    /// TODO: docs.
    pub const UNKNOWN: Self = Self(0);

    /// TODO: docs.
    #[inline]
    pub fn is_unknown(self) -> bool {
        self.0 == Self::UNKNOWN.0
    }

    #[inline]
    pub(crate) fn post_inc(&mut self) -> Self {
        let id = self.0;
        self.0 += 1;
        Self(id)
    }
}

impl Default for AgentId {
    #[inline]
    fn default() -> Self {
        Self(1)
    }
}

impl PartialEq for AgentId {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl PartialOrd for AgentId {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_unknown() || other.is_unknown() {
            None
        } else {
            self.0.partial_cmp(&other.0)
        }
    }
}

impl Hash for AgentId {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u64(self.0);
    }
}
