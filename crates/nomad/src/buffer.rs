use alloc::borrow::Cow;
use core::hash::Hash;
use core::ops::RangeBounds;

use collab_fs::AbsUtf8Path;
use futures_util::Stream;

use crate::{ActorId, ByteOffset, Context, Edit, Editor, Text};

/// TODO: docs.
pub trait Buffer<E: Editor> {
    /// TODO: docs.
    type EditStream: Stream<Item = Edit>;

    /// TODO: docs.
    type Id: Clone + PartialEq + Hash + Ord;

    /// TODO: docs.
    fn edit_stream(&mut self, ctx: &Context<E>) -> Self::EditStream;

    /// TODO: docs.
    fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>;

    /// TODO: docs.
    fn id(&self) -> Self::Id;

    /// TODO: docs.
    fn path(&self) -> Option<Cow<'_, AbsUtf8Path>>;

    /// TODO: docs.
    fn set_text<R, T>(
        &mut self,
        replaced_range: R,
        new_text: T,
        actor_id: ActorId,
    ) where
        R: RangeBounds<ByteOffset>,
        T: AsRef<str>;
}
