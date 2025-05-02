use ::serde::{Deserialize, Serialize};
use ed_core::backend::{Backend, Buffer};
use ed_core::fs::{self, AbsPath};
use ed_core::notify::Namespace;
use ed_core::plugin::Plugin;
use nvim_oxi::api::Window;

use crate::buffer::NeovimBuffer;
use crate::{api, autocmd, executor, notify, oxi, serde, value};

/// TODO: docs.
pub struct Neovim {
    augroup_id: u32,
    callbacks: autocmd::Callbacks,
    emitter: notify::NeovimEmitter,
    local_executor: executor::NeovimLocalExecutor,
    background_executor: executor::NeovimBackgroundExecutor,
}

impl Neovim {
    /// TODO: docs.
    #[inline]
    pub fn init() -> Self {
        Self {
            augroup_id: oxi::api::create_augroup(
                "",
                &oxi::api::opts::CreateAugroupOpts::builder()
                    .clear(true)
                    .build(),
            )
            .expect("couldn't create augroup"),
            callbacks: Default::default(),
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
    type Buffer<'a> = NeovimBuffer;
    type BufferId = NeovimBuffer;
    type Cursor<'a> = NeovimBuffer;
    type CursorId = NeovimBuffer;
    type Fs = fs::os::OsFs;
    type LocalExecutor = executor::NeovimLocalExecutor;
    type BackgroundExecutor = executor::NeovimBackgroundExecutor;
    type Emitter<'this> = &'this mut notify::NeovimEmitter;
    type EventHandle = autocmd::EventHandle;
    type Selection<'a> = NeovimBuffer;
    type SelectionId = NeovimBuffer;

    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf: NeovimBuffer) -> Option<Self::Buffer<'_>> {
        buf.exists().then_some(buf)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.buffer_ids().find(|buf| &*buf.name() == path)
    }

    #[inline]
    fn buffer_ids(&mut self) -> impl Iterator<Item = NeovimBuffer> + use<> {
        oxi::api::list_bufs()
            .filter(|buf| buf.is_loaded())
            .map(NeovimBuffer::new)
    }

    #[inline]
    fn fs(&mut self) -> Self::Fs {
        Self::Fs::default()
    }

    #[inline]
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        Some(NeovimBuffer::current())
    }

    #[inline]
    fn cursor(&mut self, buf: Self::CursorId) -> Option<Self::Cursor<'_>> {
        (buf.exists() && buf.is_focused()).then_some(buf)
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

        Some(NeovimBuffer::new(buf))
    }

    #[inline]
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        &mut self.background_executor
    }

    #[inline]
    fn selection(
        &mut self,
        buf: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        buf.exists().then(|| buf.selection()).is_some().then_some(buf)
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
        self.callbacks.insert_callback_for(autocmd::BufReadPost, fun)
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
