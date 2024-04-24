use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::Range;

use cola::{Anchor, Deletion, Insertion, Replica};
use crop::Rope;

use crate::editor::{RemoteDeletion, RemoteInsertion};
use crate::streams::{AppliedDeletion, AppliedInsertion};
use crate::{BufferSnapshot, ByteOffset, Edit, Point, Replacement};

/// TODO: docs
#[derive(Clone)]
pub(crate) struct BufferState {
    /// TODO: docs
    inner: Rc<RefCell<BufferInner>>,
}

impl BufferState {
    #[inline]
    pub fn edit<E>(&self, edit: E) -> E::Diff
    where
        E: Edit<BufferInner>,
    {
        self.with_mut(|inner| inner.edit(edit))
    }

    #[inline]
    pub fn new(text: impl Into<Rope>, replica: Replica) -> Self {
        Self { inner: Rc::new(RefCell::new(BufferInner::new(text, replica))) }
    }

    #[inline]
    pub fn snapshot(&self) -> BufferSnapshot {
        self.with(|inner| inner.snapshot())
    }

    #[inline]
    pub(crate) fn with<R>(&self, f: impl FnOnce(&BufferInner) -> R) -> R {
        let inner = self.inner.borrow();
        f(&inner)
    }

    #[inline]
    pub(crate) fn with_mut<R>(
        &self,
        f: impl FnOnce(&mut BufferInner) -> R,
    ) -> R {
        let mut inner = self.inner.borrow_mut();
        f(&mut inner)
    }
}

/// TODO: docs
#[derive(Clone)]
pub(super) struct BufferInner {
    /// TODO: docs
    replica: Replica,

    /// TODO: docs
    text: Rope,
}

impl BufferInner {
    /// TODO: docs
    #[inline]
    pub fn delete(&mut self, range: Range<ByteOffset>) -> Deletion {
        let range: Range<usize> = range.start.into()..range.end.into();
        self.text.delete(range.clone());
        self.replica.deleted(range)
    }

    /// TODO: docs
    #[inline]
    pub fn edit<E>(&mut self, edit: E) -> E::Diff
    where
        E: Edit<Self>,
    {
        edit.apply(self)
    }

    /// TODO: docs
    #[inline]
    pub fn insert(&mut self, offset: ByteOffset, text: &str) -> Insertion {
        self.text.insert(offset.into(), text);
        self.replica.inserted(offset.into(), text.len())
    }

    /// TODO: docs
    #[inline]
    pub fn integrate_deletion(
        &mut self,
        deletion: &Deletion,
    ) -> Vec<Range<ByteOffset>> {
        let byte_ranges = self.replica.integrate_deletion(deletion);
        byte_ranges.iter().rev().for_each(|r| self.text.delete(r.clone()));
        unsafe { core::mem::transmute(byte_ranges) }
    }

    /// TODO: docs
    #[inline]
    pub fn integrate_insertion(
        &mut self,
        insertion: &Insertion,
        text: &str,
    ) -> Option<ByteOffset> {
        let offset = self.replica.integrate_insertion(insertion)?;
        self.text.insert(offset, text);
        Some(ByteOffset::new(offset))
    }

    #[inline]
    fn new(text: impl Into<Rope>, replica: Replica) -> Self {
        let text = text.into();

        assert_eq!(
            text.byte_len(),
            replica.len(),
            "text and replica out of sync"
        );

        Self { replica, text }
    }

    /// Transforms the 1-dimensional byte offset into a 2-dimensional
    /// [`Point`].
    #[inline]
    pub fn point_of_offset(
        &self,
        byte_offset: ByteOffset,
    ) -> Point<ByteOffset> {
        let row = self.text.line_of_byte(byte_offset.into());
        let row_offset = ByteOffset::new(self.text.byte_of_line(row));
        let col = byte_offset - row_offset;
        Point::new(row, col)
    }

    /// Returns an exclusive reference to the buffer's [`Replica`].
    #[inline]
    pub(crate) fn replica_mut(&mut self) -> &mut Replica {
        &mut self.replica
    }

    /// TODO: docs
    #[inline]
    pub fn resolve_anchor(&self, anchor: &Anchor) -> Option<ByteOffset> {
        self.replica.resolve_anchor(*anchor).map(ByteOffset::new)
    }

    /// Returns a shared reference to the buffer's [`Rope`].
    #[inline]
    pub(crate) fn rope(&self) -> &Rope {
        &self.text
    }

    /// Returns an exclusive reference to the buffer's [`Rope`].
    #[inline]
    pub(crate) fn rope_mut(&mut self) -> &mut Rope {
        &mut self.text
    }

    /// TODO: docs
    #[inline]
    pub fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot::new(self.replica.clone(), self.text.clone())
    }
}

impl Edit<BufferInner> for &Replacement<ByteOffset> {
    type Diff = (Option<AppliedDeletion>, Option<AppliedInsertion>);

    #[inline]
    fn apply(self, buf: &mut BufferInner) -> Self::Diff {
        let mut applied_del = None;
        let mut applied_ins = None;

        if !self.range().is_empty() {
            let del = buf.delete(self.range());
            applied_del = Some(AppliedDeletion::new(del));
        }

        if !self.replacement().is_empty() {
            let ins = buf.insert(self.range().start, self.replacement());
            applied_ins = Some(AppliedInsertion::new(
                ins,
                self.replacement().to_owned(),
            ));
        }

        (applied_del, applied_ins)
    }
}

/// TODO: docs
pub struct LocalDeletion {
    range: Range<Anchor>,
}

impl LocalDeletion {
    #[inline]
    pub fn new(range: Range<Anchor>) -> Self {
        Self { range }
    }
}

impl Edit<BufferInner> for &LocalDeletion {
    type Diff = Option<(AppliedDeletion, Range<Point<ByteOffset>>)>;

    fn apply(self, buf: &mut BufferInner) -> Self::Diff {
        let start_anchor = &self.range.start;

        let end_anchor = &self.range.end;

        let Some(start_offset) = buf.resolve_anchor(start_anchor) else {
            panic_couldnt_resolve_anchor(start_anchor);
        };

        let Some(end_offset) = buf.resolve_anchor(end_anchor) else {
            panic_couldnt_resolve_anchor(end_anchor);
        };

        if start_offset == end_offset {
            return None;
        }

        let start_point = buf.point_of_offset(start_offset);

        let end_point = buf.point_of_offset(end_offset);

        let deletion = buf.delete(start_offset..end_offset);

        Some((AppliedDeletion::new(deletion), start_point..end_point))
    }
}

/// TODO: docs
pub struct LocalInsertion {
    insert_at: Anchor,
    text: String,
}

impl LocalInsertion {
    #[inline]
    pub fn new(insert_at: Anchor, text: String) -> Self {
        Self { insert_at, text }
    }
}

impl Edit<BufferInner> for LocalInsertion {
    type Diff = (AppliedInsertion, Point<ByteOffset>);

    fn apply(self, buf: &mut BufferInner) -> Self::Diff {
        let Some(byte_offset) = buf.resolve_anchor(&self.insert_at) else {
            panic_couldnt_resolve_anchor(&self.insert_at);
        };

        let point = buf.point_of_offset(byte_offset);

        let insertion = buf.insert(byte_offset, &self.text);

        (AppliedInsertion::new(insertion, self.text), point)
    }
}

impl Edit<BufferInner> for &RemoteDeletion {
    type Diff = Vec<Range<Point<ByteOffset>>>;

    fn apply(self, buf: &mut BufferInner) -> Self::Diff {
        let buf_prev = buf.clone();

        let byte_ranges = buf.integrate_deletion(self.inner());

        byte_ranges
            .into_iter()
            .map(|range| {
                let start = buf_prev.point_of_offset(range.start);
                let end = buf_prev.point_of_offset(range.end);
                start..end
            })
            .collect()
    }
}

impl Edit<BufferInner> for &RemoteInsertion {
    type Diff = Option<Point<ByteOffset>>;

    fn apply(self, buf: &mut BufferInner) -> Self::Diff {
        let buf_prev = buf.clone();
        let offset = buf.integrate_insertion(self.inner(), self.text())?;
        let point = buf_prev.point_of_offset(offset);
        Some(point)
    }
}

#[inline(never)]
fn panic_couldnt_resolve_anchor(anchor: &Anchor) -> ! {
    panic!("{anchor:?} couldn't be resolved");
}
