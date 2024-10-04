use alloc::borrow::Cow;
use core::hash::Hash;
use core::ops::RangeBounds;

use collab_fs::AbsUtf8Path;

use crate::{ByteOffset, Editor, Text};

/// TODO: docs.
pub trait Buffer<E: Editor + ?Sized> {
    /// TODO: docs.
    type Id: Clone + PartialEq + Hash + Ord;

    /// TODO: docs.
    fn byte_len(&self) -> usize;

    /// TODO: docs.
    fn get_text<R>(&self, byte_range: R) -> Text
    where
        R: RangeBounds<ByteOffset>;

    /// TODO: docs.
    fn path(&self) -> Option<Cow<'_, AbsUtf8Path>>;

    /// TODO: docs.
    fn set_text<R, T>(&mut self, replaced_range: R, new_text: T) -> Text
    where
        R: RangeBounds<ByteOffset>,
        T: AsRef<str>;
}
