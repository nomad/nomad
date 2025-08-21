use core::ops::{Deref, DerefMut};

use abs_path::AbsPath;

use crate::notify::MaybeResult;
use crate::{AccessMut, AgentId, Editor};

/// TODO: docs.
pub trait EditorAdapter: 'static + Sized + DerefMut<Target: Editor> {}

impl<Ed: EditorAdapter> Editor for Ed {
    type Api = <<Self as Deref>::Target as Editor>::Api;
    type Buffer<'a> = <<Self as Deref>::Target as Editor>::Buffer<'a>;
    type BufferId = <<Self as Deref>::Target as Editor>::BufferId;
    type Cursor<'a> = <<Self as Deref>::Target as Editor>::Cursor<'a>;
    type CursorId = <<Self as Deref>::Target as Editor>::CursorId;
    type Fs = <<Self as Deref>::Target as Editor>::Fs;
    type Emitter<'a> = <<Self as Deref>::Target as Editor>::Emitter<'a>;
    type Executor = <<Self as Deref>::Target as Editor>::Executor;
    type EventHandle = <<Self as Deref>::Target as Editor>::EventHandle;
    type Selection<'a> = <<Self as Deref>::Target as Editor>::Selection<'a>;
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
        self.deref_mut().buffer(id)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.deref_mut().buffer_at_path(path)
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
        self.deref_mut().current_buffer()
    }

    #[inline]
    fn for_each_buffer<Fun>(&mut self, fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>),
    {
        self.deref_mut().for_each_buffer(fun)
    }

    #[inline]
    fn cursor(&mut self, id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        self.deref_mut().cursor(id)
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
        fun: Fun,
        access: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static,
    {
        self.deref_mut().on_buffer_created(
            fun,
            access.map_mut(Deref::deref, DerefMut::deref_mut),
        )
    }

    #[inline]
    fn on_cursor_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Cursor<'_>, AgentId) + 'static,
    {
        self.deref_mut().on_cursor_created(fun)
    }

    #[inline]
    fn on_selection_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static,
    {
        self.deref_mut().on_selection_created(fun)
    }

    #[inline]
    fn reinstate_panic_hook(&self) -> bool {
        self.deref().reinstate_panic_hook()
    }

    #[inline]
    fn selection(
        &mut self,
        id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        self.deref_mut().selection(id)
    }

    #[inline]
    fn serialize<T>(
        &mut self,
        value: &T,
    ) -> impl MaybeResult<crate::ApiValue<Self>, Error = Self::SerializeError>
    where
        T: ?Sized + serde::Serialize,
    {
        self.deref_mut().serialize(value)
    }

    #[inline]
    fn deserialize<'de, T>(
        &mut self,
        value: crate::ApiValue<Self>,
    ) -> impl MaybeResult<T, Error = Self::DeserializeError>
    where
        T: serde::Deserialize<'de>,
    {
        self.deref_mut().deserialize(value)
    }
}
