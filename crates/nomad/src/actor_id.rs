use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

/// TODO: docs.
#[derive(Copy, Clone, Debug)]
pub struct ActorId(u64);

impl ActorId {
    const UNKNOWN: u64 = 0;

    /// TODO: docs.
    pub fn into_u64(self) -> u64 {
        self.0
    }

    /// TODO: docs.
    pub fn is_unknown(self) -> bool {
        self.0 == Self::UNKNOWN
    }

    /// TODO: docs.
    #[track_caller]
    pub fn new(id: u64) -> Self {
        assert!(id != Self::UNKNOWN, "ActorId cannot be {}", Self::UNKNOWN);
        Self(id)
    }

    /// TODO: docs.
    pub fn unknown() -> Self {
        Self(Self::UNKNOWN)
    }
}

impl PartialEq for ActorId {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

impl PartialOrd for ActorId {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_unknown() || other.is_unknown() {
            None
        } else {
            self.0.partial_cmp(&other.0)
        }
    }
}

impl Hash for ActorId {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u64(self.0);
    }
}
