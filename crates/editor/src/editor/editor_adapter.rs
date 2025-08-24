use core::mem;
use core::ops::{Deref, DerefMut, Range};

use abs_path::AbsPath;

use crate::AccessMut;
use crate::editor::{self, AgentId, Buffer, Cursor, Editor, Selection};

/// TODO: docs.
pub trait EditorAdapter: 'static + Sized + DerefMut<Target: Editor> {}

/// TODO: docs.
#[repr(transparent)]
pub struct BufferAdapter<'a, Ed: EditorAdapter> {
    inner: <<Ed as Deref>::Target as Editor>::Buffer<'a>,
}

/// TODO: docs.
#[repr(transparent)]
pub struct CursorAdapter<'a, Ed: EditorAdapter> {
    inner: <<Ed as Deref>::Target as Editor>::Cursor<'a>,
}

/// TODO: docs.
#[repr(transparent)]
pub struct SelectionAdapter<'a, Ed: EditorAdapter> {
    inner: <<Ed as Deref>::Target as Editor>::Selection<'a>,
}

impl<'a, Ed: EditorAdapter> BufferAdapter<'a, Ed> {
    #[inline]
    fn from_ref<'b>(
        inner: &'b <<Ed as Deref>::Target as Editor>::Buffer<'a>,
    ) -> &'b Self {
        // SAFETY: `Self` is repr(transparent).
        unsafe { mem::transmute(inner) }
    }

    #[inline]
    fn from_mut<'b>(
        inner: &'b mut <<Ed as Deref>::Target as Editor>::Buffer<'a>,
    ) -> &'b mut Self {
        // SAFETY: `Self` is repr(transparent).
        unsafe { mem::transmute(inner) }
    }

    #[inline]
    fn new(buffer: <<Ed as Deref>::Target as Editor>::Buffer<'a>) -> Self {
        Self { inner: buffer }
    }
}

impl<'a, Ed: EditorAdapter> CursorAdapter<'a, Ed> {
    #[inline]
    fn from_mut<'b>(
        inner: &'b mut <<Ed as Deref>::Target as Editor>::Cursor<'a>,
    ) -> &'b mut Self {
        // SAFETY: `Self` is repr(transparent).
        unsafe { mem::transmute(inner) }
    }

    #[inline]
    fn from_ref<'b>(
        inner: &'b <<Ed as Deref>::Target as Editor>::Cursor<'a>,
    ) -> &'b Self {
        // SAFETY: `Self` is repr(transparent).
        unsafe { mem::transmute(inner) }
    }

    #[inline]
    fn new(inner: <<Ed as Deref>::Target as Editor>::Cursor<'a>) -> Self {
        Self { inner }
    }
}

impl<'a, Ed: EditorAdapter> SelectionAdapter<'a, Ed> {
    #[inline]
    fn from_ref<'b>(
        inner: &'b <<Ed as Deref>::Target as Editor>::Selection<'a>,
    ) -> &'b Self {
        // SAFETY: `Self` is repr(transparent).
        unsafe { mem::transmute(inner) }
    }

    #[inline]
    fn from_mut<'b>(
        inner: &'b mut <<Ed as Deref>::Target as Editor>::Selection<'a>,
    ) -> &'b mut Self {
        // SAFETY: `Self` is repr(transparent).
        unsafe { mem::transmute(inner) }
    }

    #[inline]
    fn new(inner: <<Ed as Deref>::Target as Editor>::Selection<'a>) -> Self {
        Self { inner }
    }
}

impl<Ed: EditorAdapter> Editor for Ed {
    type Api = <<Self as Deref>::Target as Editor>::Api;
    type Buffer<'a> = BufferAdapter<'a, Self>;
    type BufferId = <<Self as Deref>::Target as Editor>::BufferId;
    type Cursor<'a> = CursorAdapter<'a, Self>;
    type CursorId = <<Self as Deref>::Target as Editor>::CursorId;
    type Fs = <<Self as Deref>::Target as Editor>::Fs;
    type Emitter<'a> = <<Self as Deref>::Target as Editor>::Emitter<'a>;
    type Executor = <<Self as Deref>::Target as Editor>::Executor;
    type EventHandle = <<Self as Deref>::Target as Editor>::EventHandle;
    type Selection<'a> = SelectionAdapter<'a, Self>;
    type SelectionId = <<Self as Deref>::Target as Editor>::SelectionId;
    type BufferSaveError =
        <<Self as Deref>::Target as Editor>::BufferSaveError;
    type CreateBufferError =
        <<Self as Deref>::Target as Editor>::CreateBufferError;
    type SerializeError = <<Self as Deref>::Target as Editor>::SerializeError;
    type DeserializeError =
        <<Self as Deref>::Target as Editor>::DeserializeError;

    #[inline]
    fn buffer(&mut self, id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        self.deref_mut().buffer(id).map(BufferAdapter::new)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.deref_mut().buffer_at_path(path).map(BufferAdapter::new)
    }

    #[inline]
    fn create_buffer(
        this: impl AccessMut<Self>,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> impl Future<Output = Result<Self::BufferId, Self::CreateBufferError>>
    {
        <<Self as Deref>::Target>::create_buffer(
            this.map_mut(Deref::deref, DerefMut::deref_mut),
            file_path,
            agent_id,
        )
    }

    #[inline]
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.deref_mut().current_buffer().map(BufferAdapter::new)
    }

    #[inline]
    fn for_each_buffer<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>),
    {
        self.deref_mut().for_each_buffer(|buffer| {
            fun(BufferAdapter::new(buffer));
        });
    }

    #[inline]
    fn cursor(&mut self, id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        self.deref_mut().cursor(id).map(CursorAdapter::new)
    }

    #[inline]
    fn fs(&mut self) -> Self::Fs {
        self.deref_mut().fs()
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        self.deref_mut().emitter()
    }

    #[inline]
    fn executor(&mut self) -> &mut Self::Executor {
        self.deref_mut().executor()
    }

    #[inline]
    fn on_buffer_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&mut Self::Buffer<'_>, AgentId) + 'static,
    {
        self.deref_mut().on_buffer_created(
            move |inner, agent_id| {
                fun(BufferAdapter::from_mut(inner), agent_id);
            },
            this.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_cursor_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&mut Self::Cursor<'_>, AgentId) + 'static,
    {
        self.deref_mut().on_cursor_created(
            move |inner, agent_id| {
                fun(CursorAdapter::from_mut(inner), agent_id);
            },
            this.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_selection_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&mut Self::Selection<'_>, AgentId) + 'static,
    {
        self.deref_mut().on_selection_created(
            move |inner, agent_id| {
                fun(SelectionAdapter::from_mut(inner), agent_id);
            },
            this.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn reinstate_panic_hook(&self) -> bool {
        self.deref().reinstate_panic_hook()
    }

    #[inline]
    fn remove_event(&mut self, event_handle: Self::EventHandle) {
        self.deref_mut().remove_event(event_handle);
    }

    #[inline]
    fn selection(
        &mut self,
        id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        self.deref_mut().selection(id).map(SelectionAdapter::new)
    }

    #[inline]
    fn serialize<T>(
        &mut self,
        value: &T,
    ) -> Result<editor::ApiValue<Self>, Self::SerializeError>
    where
        T: ?Sized + serde::Serialize,
    {
        self.deref_mut().serialize(value)
    }

    #[inline]
    fn deserialize<'de, T>(
        &mut self,
        value: editor::ApiValue<Self>,
    ) -> Result<T, Self::DeserializeError>
    where
        T: serde::Deserialize<'de>,
    {
        self.deref_mut().deserialize(value)
    }
}

impl<'a, Ed: EditorAdapter> Buffer for BufferAdapter<'a, Ed> {
    type Editor = Ed;

    #[inline]
    fn byte_len(&self) -> editor::ByteOffset {
        self.inner.byte_len()
    }

    #[inline]
    fn get_text_range(
        &self,
        byte_range: Range<editor::ByteOffset>,
    ) -> impl editor::Chunks {
        self.inner.get_text_range(byte_range)
    }

    #[inline]
    fn id(&self) -> <Self::Editor as Editor>::BufferId {
        self.inner.id()
    }

    #[inline]
    fn for_each_cursor<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(<Self::Editor as Editor>::Cursor<'_>),
    {
        self.inner.for_each_cursor(move |inner| {
            fun(CursorAdapter::new(inner));
        });
    }

    #[inline]
    fn on_edited<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Buffer<'_>, &editor::Edit)
            + 'static,
    {
        self.inner.on_edited(
            move |inner, edit| {
                fun(BufferAdapter::from_ref(inner), edit);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_removed<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::BufferId, AgentId) + 'static,
    {
        self.inner.on_removed(
            move |buffer_id, agent_id| {
                fun(buffer_id, agent_id);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_saved<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Buffer<'_>, AgentId) + 'static,
    {
        self.inner.on_saved(
            move |inner, agent_id| {
                fun(BufferAdapter::from_ref(inner), agent_id);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn path(&self) -> std::borrow::Cow<'_, AbsPath> {
        self.inner.path()
    }

    #[inline]
    fn schedule_edit<R>(
        &mut self,
        replacements: R,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static
    where
        R: IntoIterator<Item = editor::Replacement>,
    {
        self.inner.schedule_edit(replacements, agent_id)
    }

    #[inline]
    fn schedule_focus(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        self.inner.schedule_focus(agent_id)
    }

    #[inline]
    fn schedule_save(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<
        Output = Result<(), <Self::Editor as Editor>::BufferSaveError>,
    > + 'static {
        self.inner.schedule_save(agent_id)
    }
}

impl<'a, Ed: EditorAdapter> Cursor for CursorAdapter<'a, Ed> {
    type Editor = Ed;

    #[inline]
    fn buffer_id(&self) -> <Self::Editor as Editor>::BufferId {
        self.inner.buffer_id()
    }

    #[inline]
    fn byte_offset(&self) -> editor::ByteOffset {
        self.inner.byte_offset()
    }

    #[inline]
    fn id(&self) -> <Self::Editor as Editor>::CursorId {
        self.inner.id()
    }

    #[inline]
    fn on_moved<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Cursor<'_>, AgentId) + 'static,
    {
        self.inner.on_moved(
            move |inner, agent_id| {
                fun(CursorAdapter::from_ref(inner), agent_id);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_removed<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::CursorId, AgentId) + 'static,
    {
        self.inner.on_removed(
            move |cursor_id, agent_id| {
                fun(cursor_id, agent_id);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn schedule_move(
        &mut self,
        offset: editor::ByteOffset,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        self.inner.schedule_move(offset, agent_id)
    }
}

impl<'a, Ed: EditorAdapter> Selection for SelectionAdapter<'a, Ed> {
    type Editor = Ed;

    #[inline]
    fn buffer_id(&self) -> <Self::Editor as Editor>::BufferId {
        self.inner.buffer_id()
    }

    #[inline]
    fn byte_range(&self) -> Range<editor::ByteOffset> {
        self.inner.byte_range()
    }

    #[inline]
    fn id(&self) -> <Self::Editor as Editor>::SelectionId {
        self.inner.id()
    }

    #[inline]
    fn on_moved<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun:
            FnMut(&<Self::Editor as Editor>::Selection<'_>, AgentId) + 'static,
    {
        self.inner.on_moved(
            move |inner, agent_id| {
                fun(SelectionAdapter::from_ref(inner), agent_id);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_removed<Fun>(
        &mut self,
        mut fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::SelectionId, AgentId) + 'static,
    {
        self.inner.on_removed(
            move |selection_id, agent_id| {
                fun(selection_id, agent_id);
            },
            editor.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }
}
