use core::convert::Infallible;
use core::mem;

use ::serde::{Deserialize, Serialize};
use abs_path::AbsPath;
use editor::module::Plugin;
use editor::notify::Namespace;
use editor::{AccessMut, AgentId, Buffer, Editor};

use crate::buffer::{
    BufferId,
    HighlightRange,
    HighlightRangeHandle,
    NeovimBuffer,
};
use crate::buffer_ext::BufferExt;
use crate::cursor::NeovimCursor;
use crate::decoration_provider::DecorationProvider;
use crate::events::{self, EventHandle, Events};
use crate::selection::NeovimSelection;
use crate::{api, executor, notify, oxi, serde, value};

/// TODO: docs.
pub struct Neovim {
    /// TODO: docs.
    pub(crate) decoration_provider: DecorationProvider,

    /// TODO: docs.
    pub(crate) events: Events,

    /// TODO: docs.
    emitter: notify::NeovimEmitter,

    /// TODO: docs.
    executor: executor::NeovimExecutor,

    /// TODO: docs.
    reinstate_panic_hook: bool,

    #[cfg(feature = "test")]
    pub(crate) scratch_buffer_count: u32,
}

impl Neovim {
    /// TODO: docs.
    #[inline]
    pub fn highlight_range<'a>(
        &'a mut self,
        handle: &'a HighlightRangeHandle,
    ) -> Option<HighlightRange<'a>> {
        self.buffer(handle.buffer_id())
            .map(|buffer| HighlightRange::new(buffer.clone(), handle))
    }

    /// Returns the namespace ID used by this `Neovim` instance.
    #[inline]
    pub fn namespace_id(&self) -> u32 {
        self.decoration_provider.namespace_id()
    }

    /// TODO: docs.
    #[inline]
    pub fn set_notifier(&mut self, emitter: impl Into<notify::NeovimEmitter>) {
        self.emitter = emitter.into();
    }

    /// Returns a new instance of the [`TracingLayer`](crate::TracingLayer).
    #[cfg(feature = "tracing")]
    #[inline]
    pub fn tracing_layer<S>(&mut self) -> crate::TracingLayer<S> {
        crate::TracingLayer::new(self)
    }

    /// Should only be called by the `#[neovim::plugin]` macro.
    #[doc(hidden)]
    #[track_caller]
    #[inline]
    pub fn new_plugin(plugin_name: &str) -> Self {
        Self::new_inner(plugin_name, false)
    }

    #[inline]
    pub(crate) fn create_buffer_sync<This: AccessMut<Self> + ?Sized>(
        this: &mut This,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> Result<BufferId, oxi::api::Error> {
        this.with_mut(|this| {
            if this.events.contains(&events::BufferCreated) {
                this.events.agent_ids.created_buffer = agent_id;
            }
        });

        let buffer = oxi::api::call_function::<_, oxi::api::Buffer>(
            "bufadd",
            (file_path.as_str(),),
        )?;

        // We expect an integer because 'bufload' returns 0 on success.
        oxi::api::call_function::<_, u8>("bufload", (buffer.handle(),))?;

        Ok(BufferId::from(buffer))
    }

    #[track_caller]
    #[cfg(feature = "test")]
    pub(crate) fn new_test(test_name: &str) -> Self {
        Self::new_inner(test_name, true)
    }

    #[track_caller]
    #[inline]
    fn new_inner(plugin_name: &str, reinstate_panic_hook: bool) -> Self {
        let augroup_id = oxi::api::create_augroup(
            plugin_name,
            &oxi::api::opts::CreateAugroupOpts::builder().clear(true).build(),
        )
        .expect("couldn't create augroup");

        let namespace_id = oxi::api::create_namespace(plugin_name);

        Self {
            decoration_provider: DecorationProvider::new(namespace_id),
            events: Events::new(augroup_id),
            emitter: Default::default(),
            executor: Default::default(),
            reinstate_panic_hook,
            #[cfg(feature = "test")]
            scratch_buffer_count: 0,
        }
    }
}

impl Editor for Neovim {
    type Api = api::NeovimApi;
    type Buffer<'a> = NeovimBuffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = NeovimCursor<'a>;
    type CursorId = BufferId;
    type Fs = real_fs::RealFs;
    type Emitter<'this> = &'this mut notify::NeovimEmitter;
    type Executor = executor::NeovimExecutor;
    type EventHandle = EventHandle;
    type Selection<'a> = NeovimSelection<'a>;
    type SelectionId = BufferId;

    type BufferSaveError = Infallible;
    type CreateBufferError = oxi::api::Error;
    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf_id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        NeovimBuffer::new(buf_id, self)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        for buf_id in oxi::api::list_bufs().map(Into::into) {
            let Some(buffer) = self.buffer(buf_id) else { continue };
            if &*buffer.path() == path {
                // SAFETY: Rust is dumb.
                let buffer = unsafe {
                    mem::transmute::<Self::Buffer<'_>, Self::Buffer<'_>>(
                        buffer,
                    )
                };
                return Some(buffer);
            }
        }
        None
    }

    #[inline]
    fn fs(&mut self) -> Self::Fs {
        Self::Fs::default()
    }

    #[inline]
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.buffer(BufferId::from(oxi::api::Buffer::current()))
    }

    #[inline]
    fn for_each_buffer<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>),
    {
        for buf_id in oxi::api::list_bufs().map(Into::into) {
            if let Some(buffer) = self.buffer(buf_id) {
                fun(buffer);
            }
        }
    }

    #[inline]
    async fn create_buffer(
        mut this: impl AccessMut<Self>,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        Self::create_buffer_sync(&mut this, file_path, agent_id)
    }

    #[inline]
    fn cursor(&mut self, buf_id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.is_focused().then_some(buffer.into())
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }

    #[inline]
    fn executor(&mut self) -> &mut Self::Executor {
        &mut self.executor
    }

    #[inline]
    fn selection(
        &mut self,
        buf_id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.selection().is_some().then_some(buffer.into())
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
    fn on_buffer_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(Self::Buffer<'_>, AgentId) + 'static,
    {
        self.events.insert(
            events::BufferCreated,
            move |(buf, created_by)| fun(buf, created_by),
            this,
        )
    }

    #[inline]
    fn on_cursor_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(Self::Cursor<'_>, AgentId) + 'static,
    {
        self.events.insert(
            events::CursorCreated,
            move |(buf, created_by)| fun(NeovimCursor::from(buf), created_by),
            this,
        )
    }

    #[inline]
    fn on_selection_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(Self::Selection<'_>, AgentId) + 'static,
    {
        self.events.insert(
            events::ModeChanged,
            move |(buf, old_mode, new_mode, changed_by)| {
                if new_mode.has_selected_range()
                    // A selection is only created if the old mode wasn't
                    // already displaying a selected range.
                    && !old_mode.has_selected_range()
                {
                    fun(NeovimSelection::from(buf), changed_by);
                }
            },
            this,
        )
    }

    #[inline]
    fn reinstate_panic_hook(&self) -> bool {
        self.reinstate_panic_hook
    }

    #[inline]
    fn remove_event(&mut self, event_handle: Self::EventHandle) {
        self.events.remove_event(event_handle);
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
