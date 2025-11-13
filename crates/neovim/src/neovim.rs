use core::convert::Infallible;
use core::mem;
use std::io;

use ::executor::{BackgroundSpawner, Executor};
use ::serde::{Deserialize, Serialize};
use abs_path::{AbsPath, AbsPathBuf};
use clipboard::{FallibleInitClipboard, arboard};
use editor::module::Plugin;
use editor::notify::Namespace;
use editor::{AccessMut, AgentId, Buffer, Editor};
use either::Either;
use nvim_oxi::api;

use crate::api::NeovimApi;
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
use crate::{executor, notify, serde, value};

/// The type of error returned by [`Neovim::data_dir_path()`].
pub type DataDirError = Either<api::Error, abs_path::NormalizeError>;

type HttpClient = http_client::UreqClient<
    <<Neovim as Editor>::Executor as Executor>::BackgroundSpawner,
>;

/// TODO: docs.
pub struct Neovim {
    /// TODO: docs.
    pub(crate) decoration_provider: DecorationProvider,
    /// TODO: docs.
    pub(crate) events: Events,
    clipboard: FallibleInitClipboard<arboard::Clipboard>,
    executor: executor::NeovimExecutor,
    http_client: HttpClient,
    #[cfg(feature = "test")]
    pub(crate) scratch_buffer_count: u32,
}

impl Neovim {
    /// Returns the path to the user's data directory used by Neovim.
    #[inline]
    pub fn data_dir_path(&self) -> Result<AbsPathBuf, DataDirError> {
        api::call_function::<_, String>("stdpath", ("data",))
            .map_err(Either::Left)
            .and_then(|path| {
                // I've seen 'stdpath' return paths like
                // /Users/noib3/dev/repro/.repro//data/nvim when using Lazy's
                // 'lazy.minit' module, so normalize the path to be safe.
                AbsPath::normalize(&path)
                    .map_err(Either::Right)
                    .map(|path| path.into_owned())
            })
    }

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
    pub fn new(plugin_name: &str) -> Self {
        let augroup_id = api::create_augroup(
            plugin_name,
            &api::opts::CreateAugroupOpts::builder().clear(true).build(),
        )
        .expect("couldn't create augroup");

        let namespace_id = api::create_namespace(plugin_name);

        let mut executor = executor::NeovimExecutor::default();

        Self {
            clipboard: Default::default(),
            decoration_provider: DecorationProvider::new(namespace_id),
            events: Events::new(augroup_id),
            http_client: HttpClient::new(
                ureq::Agent::new_with_defaults(),
                executor.background_spawner().clone(),
            ),
            executor,
            #[cfg(feature = "test")]
            scratch_buffer_count: 0,
        }
    }

    #[inline]
    pub(crate) fn create_buffer_sync<This: AccessMut<Self> + ?Sized>(
        this: &mut This,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> BufferId {
        this.with_mut(|this| {
            if this.events.contains(&events::BufferCreated) {
                this.events.agent_ids.created_buffer = agent_id;
            }
            if let Some(callbacks) = &mut this.events.on_cursor_created {
                // 'bufload' triggers BufEnter, so make CursorCreated skip the
                // next event.
                callbacks.register_output_mut().set_skip_next();
            }
        });

        let buffer = api::call_function::<_, api::Buffer>(
            "bufadd",
            (file_path.as_str(),),
        )
        .expect("couldn't bufadd");

        api::set_option_value(
            "buflisted",
            true,
            &api::opts::OptionOpts::builder().buf(buffer.clone()).build(),
        )
        .expect("couldn't set 'buflisted' on new buffer");

        // We expect an integer because 'bufload' returns 0 on success.
        api::call_function::<_, u8>("bufload", (buffer.clone(),))
            .expect("couldn't bufload");

        BufferId::from(buffer)
    }
}

impl Editor for Neovim {
    type Api = NeovimApi;
    type Buffer<'a> = NeovimBuffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = NeovimCursor<'a>;
    type CursorId = BufferId;
    type Clipboard = FallibleInitClipboard<arboard::Clipboard>;
    type Fs = real_fs::RealFs;
    type Emitter<'ex> = notify::NeovimEmitter<'ex>;
    type Executor = executor::NeovimExecutor;
    type EventHandle = EventHandle;
    type HttpClient = HttpClient;
    type Selection<'a> = NeovimSelection<'a>;
    type SelectionId = BufferId;

    type BufferSaveError = Infallible;
    type CreateBufferError = core::convert::Infallible;
    type OpenUrlError = io::Error;
    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf_id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        NeovimBuffer::new(buf_id, self)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        for buf_id in api::list_bufs().map(Into::into) {
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
        self.buffer(BufferId::from(api::Buffer::current()))
    }

    #[inline]
    fn for_each_buffer<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>),
    {
        for buf_id in api::list_bufs().map(Into::into) {
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
        Ok(Self::create_buffer_sync(&mut this, file_path, agent_id))
    }

    #[inline]
    fn cursor(&mut self, buf_id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.is_focused().then(|| buffer.into())
    }

    #[inline]
    fn clipboard(&mut self) -> &mut Self::Clipboard {
        &mut self.clipboard
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        let namespace_id = self.namespace_id();
        notify::NeovimEmitter::new(
            self.executor().local_spawner(),
            namespace_id,
        )
    }

    #[inline]
    fn executor(&mut self) -> &mut Self::Executor {
        &mut self.executor
    }

    #[inline]
    fn http_client(&self) -> &Self::HttpClient {
        &self.http_client
    }

    #[inline]
    fn selection(
        &mut self,
        buf_id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.selection().is_some().then(|| buffer.into())
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
    fn open_url(
        &mut self,
        url: url::Url,
    ) -> impl Future<Output = Result<(), Self::OpenUrlError>> + use<> {
        let spawner = self.executor().background_spawner().clone();

        #[cfg(not(target_os = "macos"))]
        let future = async move { webbrowser::open(url.as_str()) };

        #[cfg(target_os = "macos")]
        let future = async move {
            use objc2_app_kit::NSWorkspace;
            use objc2_foundation::{NSString, NSURL};
            let try_block = || {
                let str = NSString::from_str(url.as_str());
                let url = NSURL::URLWithString(&str).ok_or("invalid URL")?;
                let workspace = NSWorkspace::sharedWorkspace();
                let success = workspace.openURL(&url);
                if success { Ok(()) } else { Err("operation failed") }
            };
            try_block().map_err(|err_msg| {
                io::Error::other(format!(
                    "couldn't open {}: {err_msg}",
                    url.as_str()
                ))
            })
        };

        async move { spawner.spawn(future).await }
    }

    #[inline]
    fn reinstate_panic_hook(&self) -> bool {
        cfg!(feature = "test")
    }

    #[inline]
    fn remove_event(&mut self, event_handle: Self::EventHandle) {
        self.events.remove_event(event_handle);
    }

    #[inline]
    fn rng_seed(&self) -> Option<u64> {
        cfg!(feature = "test").then_some(42)
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
