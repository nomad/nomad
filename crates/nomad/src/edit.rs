use core::ops::Range;

use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::{ActorId, ByteOffset};

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct Edit {
    actor_id: ActorId,
    hunks: SmallVec<[Hunk; 1]>,
}

impl Edit {
    /// TODO: docs.
    pub fn new<I>(actor_id: ActorId, hunks: I) -> Self
    where
        I: IntoIterator<Item = Hunk>,
    {
        Self { actor_id, hunks: hunks.into_iter().collect() }
    }
}

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct Hunk {
    deleted_range: Range<ByteOffset>,
    inserted_text: SmolStr,
}

impl Hunk {
    /// TODO: docs.
    pub fn new(
        deleted_range: Range<ByteOffset>,
        inserted_text: impl Into<SmolStr>,
    ) -> Self {
        Self { deleted_range, inserted_text: inserted_text.into() }
    }
}
