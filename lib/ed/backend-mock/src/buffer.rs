use core::ops::{Deref, DerefMut, Range};
use std::borrow::Cow;

use crop::Rope;
use ed_core::ByteOffset;
use ed_core::backend::{self, AgentId, Buffer as _, Edit, Replacement};
use slotmap::SlotMap;

use crate::mock::{self, CallbackKind, Callbacks};

type AnnotationId = slotmap::DefaultKey;

/// TODO: docs.
pub struct Buffer<'a> {
    pub(crate) inner: &'a mut BufferInner,
    pub(crate) callbacks: &'a Callbacks,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BufferId(pub(crate) u64);

/// TODO: docs.
pub struct Cursor<'a> {
    pub(crate) buffer: &'a mut BufferInner,
    pub(crate) cursor_id: CursorId,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CursorId {
    buffer_id: BufferId,
    id_in_buffer: AnnotationId,
}

/// TODO: docs.
pub struct Selection<'a> {
    pub(crate) buffer: &'a mut BufferInner,
    pub(crate) selection_id: SelectionId,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SelectionId {
    buffer_id: BufferId,
    id_in_buffer: AnnotationId,
}

/// TODO: docs.
#[doc(hidden)]
pub struct BufferInner {
    pub(crate) cursors: SlotMap<AnnotationId, CursorInner>,
    pub(crate) contents: Rope,
    pub(crate) id: BufferId,
    pub(crate) name: String,
    pub(crate) selections: SlotMap<AnnotationId, SelectionInner>,
}

/// TODO: docs.
#[doc(hidden)]
pub struct CursorInner {
    pub(crate) id_in_buffer: AnnotationId,
    pub(crate) offset: ByteOffset,
}

/// TODO: docs.
#[doc(hidden)]
pub struct SelectionInner {
    pub(crate) id_in_buffer: AnnotationId,
    pub(crate) offset_range: Range<ByteOffset>,
}

impl<'a> Buffer<'a> {
    pub(crate) fn into_cursor(
        self,
        cursor_id: CursorId,
    ) -> Option<Cursor<'a>> {
        debug_assert_eq!(cursor_id.buffer_id(), self.id());
        self.cursors
            .contains_key(cursor_id.id_in_buffer)
            .then_some(Cursor { buffer: self.inner, cursor_id })
    }

    pub(crate) fn into_selection(
        self,
        selection_id: SelectionId,
    ) -> Option<Selection<'a>> {
        debug_assert_eq!(selection_id.buffer_id(), self.id());
        self.selections
            .contains_key(selection_id.id_in_buffer)
            .then_some(Selection { buffer: self.inner, selection_id })
    }
}

impl BufferId {
    pub(crate) fn post_inc(&mut self) -> Self {
        let id = *self;
        self.0 += 1;
        id
    }
}

impl CursorId {
    pub(crate) fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }
}

impl SelectionId {
    pub(crate) fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }
}

impl BufferInner {
    pub(crate) fn new(id: BufferId, name: String, contents: Rope) -> Self {
        Self {
            cursors: Default::default(),
            contents,
            id,
            name,
            selections: Default::default(),
        }
    }
}

impl CursorInner {
    pub(crate) fn react_to_replacement(&mut self, replacement: &Replacement) {
        if replacement.removed_range().start <= self.offset {
            self.offset = if self.offset <= replacement.removed_range().end {
                // The cursor falls within the deleted range.
                replacement.removed_range().start
            } else {
                // The cursor is after the deleted range.
                let range = replacement.removed_range();
                let range_len = range.end - range.start;
                self.offset - range_len
            } + replacement.inserted_text().len();
        }
    }
}

impl SelectionInner {
    pub(crate) fn react_to_replacement(&mut self, replacement: &Replacement) {
        if self.offset_range.end <= replacement.removed_range().start {
            // <selection><deletion>
            return;
        }

        if self.offset_range.start <= replacement.removed_range().start {
            // One of:
            //
            // <selection>           <selection>     <----selection---->
            //       <deletion>      <deletion->         <deletion>
            self.offset_range.end = replacement.removed_range().start;
        } else if self.offset_range.start < replacement.removed_range().end {
            // One of:
            //
            //    <selection>            <selection>
            // <---deletion---->    <deletion>
            let len_selection =
                self.offset_range.end - self.offset_range.start;
            let len_overlap =
                replacement.removed_range().end.min(self.offset_range.end)
                    - self.offset_range.start;
            self.offset_range.start = replacement.removed_range().start
                + replacement.inserted_text().len();
            self.offset_range.end =
                self.offset_range.start + len_selection - len_overlap;
        } else {
            // <deletion><selection>
            let len_deletion = replacement.removed_range().end
                - replacement.removed_range().start;
            self.offset_range.start -= len_deletion;
            self.offset_range.end -= len_deletion;

            let len_insertion =
                ByteOffset::from(replacement.inserted_text().len());
            self.offset_range.start += len_insertion;
            self.offset_range.end += len_insertion;
        }
    }
}

impl backend::Buffer for Buffer<'_> {
    type Backend = mock::Mock;
    type Id = BufferId;
    type EventHandle = mock::EventHandle;

    fn byte_len(&self) -> ByteOffset {
        self.contents.byte_len().into()
    }

    fn id(&self) -> Self::Id {
        self.id
    }

    fn edit<R>(&mut self, replacements: R, agent_id: AgentId)
    where
        R: IntoIterator<Item = Replacement>,
    {
        let edit = Edit {
            made_by: agent_id,
            replacements: replacements.into_iter().collect(),
        };

        for replacement in &edit.replacements {
            let range = replacement.removed_range();
            self.contents.replace(
                usize::from(range.start)..usize::from(range.end),
                replacement.inserted_text(),
            );
            for cursor in self.cursors.values_mut() {
                cursor.react_to_replacement(replacement);
            }
            for selection in self.selections.values_mut() {
                selection.react_to_replacement(replacement);
            }
        }

        self.callbacks.with_mut(|callbacks| {
            for cb_kind in callbacks.values_mut() {
                if let CallbackKind::OnBufferEdited(buf_id, fun) = cb_kind {
                    if *buf_id == self.id() {
                        fun(self, &edit);
                    }
                }
            }
        });
    }

    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }

    fn on_edited<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, &Edit) + 'static,
    {
        let cb_kind = CallbackKind::OnBufferEdited(self.id(), Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::OnBufferRemoved(self.id(), Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn on_saved<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::OnBufferSaved(self.id(), Box::new(fun));
        self.callbacks.insert(cb_kind)
    }
}

impl Deref for Buffer<'_> {
    type Target = BufferInner;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
