use ::serde::{Deserialize, Serialize};
use ed::Shared;
use ed::backend::{Backend, Buffer};
use ed::fs::{self, AbsPath};
use ed::notify::Namespace;
use ed::plugin::Plugin;
use nvim_oxi::api::Window;

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{self, EventHandle, Events};
use crate::{api, executor, notify, oxi, serde, value};

/// TODO: docs.
pub struct Neovim {
    emitter: notify::NeovimEmitter,
    events: Shared<Events>,
    local_executor: executor::NeovimLocalExecutor,
    background_executor: executor::NeovimBackgroundExecutor,
}

impl Neovim {
    /// TODO: docs.
    #[inline]
    pub fn init() -> Self {
        Self {
            events: Shared::new(Events::new("")),
            emitter: notify::NeovimEmitter::default(),
            local_executor: executor::NeovimLocalExecutor::init(),
            background_executor: executor::NeovimBackgroundExecutor::init(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn set_emitter(&mut self, emitter: impl Into<notify::NeovimEmitter>) {
        self.emitter = emitter.into();
    }
}

impl Backend for Neovim {
    const REINSTATE_PANIC_HOOK: bool = false;

    type Api = api::NeovimApi;
    type Buffer<'a> = NeovimBuffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = NeovimBuffer<'a>;
    type CursorId = BufferId;
    type Fs = fs::os::OsFs;
    type LocalExecutor = executor::NeovimLocalExecutor;
    type BackgroundExecutor = executor::NeovimBackgroundExecutor;
    type Emitter<'this> = &'this mut notify::NeovimEmitter;
    type EventHandle = EventHandle;
    type Selection<'a> = NeovimBuffer<'a>;
    type SelectionId = BufferId;

    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf_id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        buf_id.is_valid().then_some(NeovimBuffer::new(buf_id, &self.events))
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.buffer_ids()
            .map(|buf_id| NeovimBuffer::new(buf_id, &self.events))
            .find(|buf| &*buf.name() == path)
    }

    #[inline]
    fn buffer_ids(&mut self) -> impl Iterator<Item = BufferId> + use<> {
        oxi::api::list_bufs().filter(|buf| buf.is_loaded()).map(BufferId::new)
    }

    #[inline]
    fn fs(&mut self) -> Self::Fs {
        Self::Fs::default()
    }

    #[inline]
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        Some(NeovimBuffer::current(&self.events))
    }

    #[inline]
    fn cursor(&mut self, buf_id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.is_focused().then_some(buffer)
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }

    #[inline]
    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        &mut self.local_executor
    }

    #[inline]
    fn focus_buffer_at(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        let buf = oxi::api::call_function::<_, oxi::api::Buffer>(
            "bufadd",
            (path.as_str(),),
        )
        .ok()?;

        if !buf.is_loaded() {
            oxi::api::set_option_value(
                "buflisted",
                true,
                &oxi::api::opts::OptionOpts::builder()
                    .buffer(buf.clone())
                    .build(),
            )
            .ok()?;
        }

        Window::current().set_buf(&buf).ok()?;

        Some(NeovimBuffer::new(BufferId::new(buf), &self.events))
    }

    #[inline]
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        &mut self.background_executor
    }

    #[inline]
    fn selection(
        &mut self,
        buf_id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.selection().is_some().then_some(buffer)
    }

    #[inline]
    fn serialize<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<value::NeovimValue, serde::NeovimSerializeError> {
        serde::serialize(value)
    }

    #[inline]
    fn deserialize<'de, T: Deserialize<'de>>(
        &mut self,
        value: value::NeovimValue,
    ) -> Result<T, serde::NeovimDeserializeError> {
        serde::deserialize(value)
    }

    #[inline]
    fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>) + 'static,
    {
        Events::insert(self.events.clone(), events::BufReadPost, fun)
    }

    #[inline]
    fn emit_deserialize_error_in_config<P: Plugin<Self>>(
        &mut self,
        config_path: &Namespace,
        namespace: &Namespace,
        mut err: Self::DeserializeError,
    ) {
        err.set_config_path(config_path.clone());
        self.emit_err(namespace, err);
    }
}
