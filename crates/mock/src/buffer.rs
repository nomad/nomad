use core::future;
use core::ops::{Deref, DerefMut, Range};
use std::borrow::Cow;

use abs_path::{AbsPath, AbsPathBuf};
use editor::{
    self,
    AccessMut,
    AgentId,
    Buffer as _,
    ByteOffset,
    Chunks,
    Edit,
    Replacement,
    Shared,
};
use fs::{Directory, File, Fs};
use slotmap::SlotMap;

use crate::fs::MockFs;
use crate::mock::{self, CallbackKind, Callbacks};

type AnnotationId = slotmap::DefaultKey;

pub struct Buffer<'a> {
    pub(crate) inner: &'a mut BufferInner,
    pub(crate) callbacks: &'a Callbacks,
    pub(crate) current_buffer: &'a mut Option<BufferId>,
    pub(crate) fs: &'a MockFs,
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
    pub(crate) file_path: AbsPathBuf,
    pub(crate) id: BufferId,
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

    fn create_cursor(
        &mut self,
        byte_offset: ByteOffset,
        agent_id: AgentId,
    ) -> Cursor<'_> {
        let id_in_buffer =
            self.cursors.insert(CursorInner { offset: byte_offset });

        let on_cursor_created = self.callbacks.with(|callbacks| {
            callbacks
                .values()
                .filter_map(|cb_kind| match cb_kind {
                    CallbackKind::CursorCreated(fun) => Some(fun.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        let mut cursor = Cursor {
            cursor_id: CursorId { buffer_id: self.id(), id_in_buffer },
            buffer: self.reborrow(),
        };

        for callback in on_cursor_created {
            callback.with_mut(|cb| cb(&mut cursor, agent_id));
        }

        cursor
    }

    fn reborrow(&mut self) -> Buffer<'_> {
        Buffer {
            inner: self.inner,
            callbacks: self.callbacks,
            current_buffer: self.current_buffer,
            fs: self.fs,
        }
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
    pub(crate) fn new(
        id: BufferId,
        file_path: AbsPathBuf,
        contents: String,
    ) -> Self {
        Self {
            cursors: Default::default(),
            contents,
            id,
            file_path,
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

impl<'a> editor::Buffer for Buffer<'a> {
    type Editor = mock::Mock;

    fn byte_len(&self) -> ByteOffset {
        self.contents.len()
    }

    fn get_text_range(&self, byte_range: Range<ByteOffset>) -> impl Chunks {
        &self.contents[byte_range]
    }

    fn id(&self) -> BufferId {
        self.id
    }

    fn for_each_cursor<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(Cursor<'_>),
    {
        let buffer_id = self.id();

        let cursor_ids = self.inner.cursors.keys().collect::<Vec<_>>();

        for id_in_buffer in cursor_ids {
            fun(Cursor {
                buffer: self.reborrow(),
                cursor_id: CursorId { buffer_id, id_in_buffer },
            });
        }
    }

    fn on_edited<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, &Edit) + 'static,
    {
        let cb_kind =
            CallbackKind::BufferEdited(self.id(), Shared::new(Box::new(fun)));
        self.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(BufferId, AgentId) + 'static,
    {
        let cb_kind =
            CallbackKind::BufferRemoved(self.id(), Shared::new(Box::new(fun)));
        self.callbacks.insert(cb_kind)
    }

    fn on_saved<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(&Buffer<'_>, AgentId) + 'static,
    {
        let cb_kind =
            CallbackKind::BufferSaved(self.id(), Shared::new(Box::new(fun)));
        self.callbacks.insert(cb_kind)
    }

    fn path(&self) -> Cow<'_, AbsPath> {
        Cow::Borrowed(&self.file_path)
    }

    fn schedule_edit<R>(
        &mut self,
        replacements: R,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static
    where
        R: IntoIterator<Item = Replacement>,
    {
        let edit = Edit {
            made_by: agent_id,
            replacements: replacements.into_iter().collect(),
        };

        for replacement in &edit.replacements {
            self.contents.replace_range(
                replacement.removed_range(),
                replacement.inserted_text(),
            );
            for cursor in self.cursors.values_mut() {
                cursor.react_to_replacement(replacement);
            }
            for selection in self.selections.values_mut() {
                selection.react_to_replacement(replacement);
            }
        }

        let on_buffer_edited = self.callbacks.with(|callbacks| {
            callbacks
                .values()
                .filter_map(|cb_kind| match cb_kind {
                    CallbackKind::BufferEdited(buf_id, fun)
                        if *buf_id == self.id() =>
                    {
                        Some(fun.clone())
                    },
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        for callback in on_buffer_edited {
            callback.with_mut(|cb| cb(self, &edit));
        }

        future::ready(())
    }

    fn schedule_focus(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        *self.current_buffer = Some(self.id);

        if self.cursors.is_empty() {
            self.create_cursor(0, agent_id);
        }

        future::ready(())
    }

    fn schedule_save(
        &mut self,
        _agent_id: AgentId,
    ) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let contents = self.contents.clone();
        let fs = self.fs.clone();
        let file_path = self.file_path.clone();

        async move {
            let mut file = match fs.node_at_path(&file_path).await? {
                Some(fs::Node::File(file)) => file,

                Some(other) => {
                    anyhow::bail!(
                        "expected a file at {}, found {:?}",
                        file_path,
                        other.kind()
                    );
                },

                None => {
                    let (parent_path, file_name) =
                        file_path.split_last().expect("file path is not root");

                    let parent = match fs
                        .node_at_path(parent_path)
                        .await?
                        .ok_or_else(|| {
                            anyhow::anyhow!("no directory at {parent_path}")
                        })? {
                        fs::Node::Directory(dir) => dir,
                        other => {
                            anyhow::bail!(
                                "expected a directory at {parent_path}, \
                                 found {:?}",
                                other.kind()
                            );
                        },
                    };

                    parent.create_file(file_name).await?
                },
            };

            file.write(contents).await?;

            todo!("trigger on_saved callbacks");
        }
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

impl editor::Cursor for Cursor<'_> {
    type Editor = mock::Mock;

    fn buffer_id(&self) -> BufferId {
        self.buffer.id()
    }

    fn byte_offset(&self) -> ByteOffset {
        self.offset
    }

    fn id(&self) -> CursorId {
        self.cursor_id
    }

    fn schedule_move(
        &mut self,
        offset: ByteOffset,
        agent_id: AgentId,
    ) -> impl future::Future<Output = ()> + 'static {
        self.offset = offset;

        let on_cursor_moved = self.buffer.callbacks.with(|callbacks| {
            callbacks
                .values()
                .filter_map(|cb_kind| match cb_kind {
                    CallbackKind::CursorMoved(cursor_id, fun)
                        if *cursor_id == self.id() =>
                    {
                        Some(fun.clone())
                    },
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        for callback in on_cursor_moved {
            callback.with_mut(|cb| cb(self, agent_id));
        }

        future::ready(())
    }

    fn on_moved<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(&Cursor<'_>, AgentId) + 'static,
    {
        let cb_kind =
            CallbackKind::CursorMoved(self.id(), Shared::new(Box::new(fun)));
        self.buffer.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(CursorId, AgentId) + 'static,
    {
        let cb_kind =
            CallbackKind::CursorRemoved(self.id(), Shared::new(Box::new(fun)));
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

impl editor::Selection for Selection<'_> {
    type Editor = mock::Mock;

    fn buffer_id(&self) -> BufferId {
        self.buffer.id()
    }

    fn byte_range(&self) -> Range<ByteOffset> {
        self.offset_range.clone()
    }

    fn id(&self) -> SelectionId {
        self.selection_id
    }

    fn on_moved<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(&Selection<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::SelectionMoved(
            self.id(),
            Shared::new(Box::new(fun)),
        );
        self.buffer.callbacks.insert(cb_kind)
    }

    fn on_removed<Fun>(
        &mut self,
        fun: Fun,
        _: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> mock::EventHandle
    where
        Fun: FnMut(SelectionId, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::SelectionRemoved(
            self.id(),
            Shared::new(Box::new(fun)),
        );
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
