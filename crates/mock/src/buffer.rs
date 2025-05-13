use core::ops::{Deref, DerefMut, Range};
use std::borrow::Cow;

use abs_path::AbsPath;
use ed::ByteOffset;
use ed::backend::{self, AgentId, Buffer as _, Edit, Replacement};
use slotmap::SlotMap;

use crate::mock::{self, CallbackKind, Callbacks};

type AnnotationId = slotmap::DefaultKey;

pub struct Buffer<'a> {
    pub(crate) inner: &'a mut BufferInner,
    pub(crate) callbacks: &'a Callbacks,
    pub(crate) current_buffer: &'a mut Option<BufferId>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BufferId(pub(crate) u64);

pub struct Cursor<'a> {
    pub(crate) buffer: Buffer<'a>,
    pub(crate) cursor_id: CursorId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CursorId {
    buffer_id: BufferId,
    id_in_buffer: AnnotationId,
}

pub struct Selection<'a> {
    pub(crate) buffer: Buffer<'a>,
    pub(crate) selection_id: SelectionId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SelectionId {
    buffer_id: BufferId,
    id_in_buffer: AnnotationId,
}

#[doc(hidden)]
pub struct BufferInner {
    pub(crate) cursors: SlotMap<AnnotationId, CursorInner>,
    pub(crate) contents: String,
    pub(crate) id: BufferId,
    pub(crate) name: String,
    pub(crate) selections: SlotMap<AnnotationId, SelectionInner>,
}

#[doc(hidden)]
pub struct CursorInner {
    pub(crate) offset: ByteOffset,
}

#[doc(hidden)]
pub struct SelectionInner {
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
            .then_some(Cursor { buffer: self, cursor_id })
    }

    pub(crate) fn into_selection(
        self,
        selection_id: SelectionId,
    ) -> Option<Selection<'a>> {
        debug_assert_eq!(selection_id.buffer_id(), self.id());
        self.selections
            .contains_key(selection_id.id_in_buffer)
            .then_some(Selection { buffer: self, selection_id })
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
    pub(crate) fn new(id: BufferId, name: String, contents: String) -> Self {
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
    /// Updates the cursor's offset in the buffer in response to the given
    /// replacement being applied to it.
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
    /// Updates the selections's offset range in the buffer in response to the
    /// given replacement being applied to it.
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
    type EventHandle = mock::EventHandle;
    type Id = BufferId;

    fn byte_len(&self) -> ByteOffset {
        self.contents.len().into()
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
            self.contents.replace_range(
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
                if let CallbackKind::BufferEdited(buf_id, fun) = cb_kind {
                    if *buf_id == self.id() {
                        fun(self, &edit);
                    }
                }
            }
        });
    }

    fn focus(&mut self) {
        *self.current_buffer = Some(self.id);
    }

    fn on_edited<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, &Edit) + 'static,
    {
        let cb_kind = CallbackKind::BufferEdited(self.id(), Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::BufferRemoved(self.id(), Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn on_saved<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::BufferSaved(self.id(), Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn path(&self) -> Cow<'_, AbsPath> {
        todo!();
        // Cow::Borrowed(&self.name)
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

impl backend::Cursor for Cursor<'_> {
    type Backend = mock::Mock;
    type EventHandle = mock::EventHandle;
    type Id = CursorId;

    fn byte_offset(&self) -> ByteOffset {
        self.offset
    }

    fn id(&self) -> Self::Id {
        self.cursor_id
    }

    fn on_moved<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Cursor<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::CursorMoved(self.id(), Box::new(fun));
        self.buffer.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Cursor<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::CursorRemoved(self.id(), Box::new(fun));
        self.buffer.callbacks.insert(cb_kind)
    }
}

impl Deref for Cursor<'_> {
    type Target = CursorInner;

    fn deref(&self) -> &Self::Target {
        self.buffer
            .cursors
            .get(self.cursor_id.id_in_buffer)
            .expect("cursor exists")
    }
}

impl DerefMut for Cursor<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .cursors
            .get_mut(self.cursor_id.id_in_buffer)
            .expect("cursor exists")
    }
}

impl backend::Selection for Selection<'_> {
    type Backend = mock::Mock;
    type EventHandle = mock::EventHandle;
    type Id = SelectionId;

    fn byte_range(&self) -> Range<ByteOffset> {
        self.offset_range.clone()
    }

    fn id(&self) -> Self::Id {
        self.selection_id
    }

    fn on_moved<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Selection<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::SelectionMoved(self.id(), Box::new(fun));
        self.buffer.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(&self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Selection<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::SelectionRemoved(self.id(), Box::new(fun));
        self.buffer.callbacks.insert(cb_kind)
    }
}

impl Deref for Selection<'_> {
    type Target = SelectionInner;

    fn deref(&self) -> &Self::Target {
        self.buffer
            .selections
            .get(self.selection_id.id_in_buffer)
            .expect("selection exists")
    }
}

impl DerefMut for Selection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .selections
            .get_mut(self.selection_id.id_in_buffer)
            .expect("selection exists")
    }
}
