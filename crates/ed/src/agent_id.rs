use core::cmp::Ordering;
use core::fmt;
use core::num::NonZeroU64;

/// TODO: docs.
#[derive(Copy, Clone)]
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
    pub(crate) fn new(id: NonZeroU64) -> Self {
        Self(id.into())
    }

    #[inline]
    pub(crate) fn post_inc(&mut self) -> Self {
        let id = self.0;
        self.0 += 1;
        Self(id)
    }
}

impl fmt::Debug for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let field: &dyn fmt::Debug =
            if self.is_unknown() { &format_args!("UNKNOWN") } else { &self.0 };
        f.debug_tuple("AgentId").field(field).finish()
    }
}

impl Default for AgentId {
    #[inline]
    fn default() -> Self {
        Self::UNKNOWN
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
