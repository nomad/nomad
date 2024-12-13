use fxhash::FxHashMap;
use nvim_oxi::api::{self, opts, types};
use nvimx_common::Shared;
use nvimx_diagnostics::{DiagnosticMessage, DiagnosticSource, Level};

use crate::actor_id::ActorId;
use crate::autocmd_ctx::AutoCommandCtx;
use crate::buffer_id::BufferId;
use crate::neovim_ctx::NeovimCtx;

/// TODO: docs.
pub trait AutoCommand: Sized {
    /// TODO: docs.
    const MODULE_NAME: Option<&'static str>;

    /// TODO: docs.
    const CALLBACK_NAME: Option<&'static str>;

    /// TODO: docs.
    fn on_event(&self) -> AutoCommandEvent;

    /// TODO: docs.
    fn on_buffer(&self) -> Option<BufferId>;

    /// TODO: docs.
    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId;

    /// TODO: docs.
    fn into_callback(
        self,
    ) -> impl for<'ctx> FnMut(
        ActorId,
        &'ctx AutoCommandCtx<'ctx>,
    ) -> Result<ShouldDetach, DiagnosticMessage>
           + 'static;
}

/// TODO: docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutoCommandEvent {
    /// TODO: docs.
    BufAdd,
    /// TODO: docs.
    BufEnter,
    /// TODO: docs.
    BufLeave,
    /// TODO: docs.
    BufUnload,
    /// TODO: docs.
    CursorMoved,
    /// TODO: docs.
    CursorMovedI,
}

/// TODO: docs.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ShouldDetach {
    /// TODO: docs.
    Yes,
    /// TODO: docs.
    No,
}

/// TODO: docs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AugroupId(u32);

#[derive(Default)]
pub(crate) struct AutoCommandMap {
    inner: FxHashMap<AutoCommandEvent, Vec<AutoCommandCallback>>,
}

type AutoCommandCallback =
    Box<dyn for<'a> FnMut(ActorId, &'a AutoCommandCtx<'a>) -> ShouldDetach>;

impl AutoCommandMap {
    pub(crate) fn register<A: AutoCommand>(
        &mut self,
        autocmd: A,
        augroup_id: AugroupId,
        ctx: NeovimCtx<'static>,
    ) {
        let event = autocmd.on_event();
        let buffer_id = autocmd.on_buffer();
        let mut has_event_been_registered = true;
        let callbacks = self.inner.entry(event).or_insert_with(|| {
            has_event_been_registered = false;
            Vec::new()
        });
        let callback = {
            let mut callback = autocmd.into_callback();
            move |actor_id, autocmd_ctx: &AutoCommandCtx<'_>| {
                if let Some(buffer_id) = buffer_id {
                    if buffer_id
                        != BufferId::new(autocmd_ctx.args().buffer.clone())
                    {
                        return ShouldDetach::No;
                    }
                }
                match callback(actor_id, autocmd_ctx) {
                    Ok(shoud_detach) => shoud_detach,
                    Err(message) => {
                        let mut source = DiagnosticSource::new();
                        if let Some(module_name) = A::MODULE_NAME {
                            source.push_segment(module_name);
                        }
                        source.push_segment(event.as_str());
                        if let Some(callback_name) = A::CALLBACK_NAME {
                            source.push_segment(callback_name);
                        }
                        message.emit(Level::Error, source);
                        ShouldDetach::Yes
                    },
                }
            }
        };
        callbacks.push(Box::new(callback));
        if !has_event_been_registered {
            register_autocmd::<A>(event, augroup_id, ctx.clone());
        }
    }
}

fn register_autocmd<A: AutoCommand>(
    event: AutoCommandEvent,
    augroup_id: AugroupId,
    ctx: NeovimCtx<'static>,
) {
    let callback = move |args: types::AutocmdCallbackArgs| {
        debug_assert_eq!(args.event, event.as_str());
        let autocmd_ctx = AutoCommandCtx::new(args, event, ctx.reborrow());
        let actor_id = A::take_actor_id(&autocmd_ctx);
        ctx.with_autocmd_map(|m| {
            let Some(callbacks) = m.inner.get_mut(&event) else {
                panic!(
                    "Neovim executed an unregistered autocommand: {event:?}"
                );
            };
            let mut idx = 0;
            loop {
                let Some(callback) = callbacks.get_mut(idx) else {
                    break;
                };
                if callback(actor_id, &autocmd_ctx).into() {
                    let _ = callbacks.remove(idx);
                } else {
                    idx += 1;
                }
            }
            let should_detach = callbacks.is_empty();
            if should_detach {
                m.inner.remove(&event);
            }
            should_detach
        })
    };

    let opts = opts::CreateAutocmdOpts::builder()
        .group(augroup_id)
        .callback(callback)
        .build();

    let _autocmd_id = api::create_autocmd([event.as_str()], &opts)
        .expect("all arguments are valid");
}

impl AutoCommandEvent {
    const BUF_ADD: &'static str = "BufAdd";
    const BUF_ENTER: &'static str = "BufEnter";
    const BUF_LEAVE: &'static str = "BufLeave";
    const BUF_UNLOAD: &'static str = "BufUnload";
    const CURSOR_MOVED: &'static str = "CursorMoved";
    const CURSOR_MOVED_I: &'static str = "CursorMovedI";

    fn as_str(&self) -> &'static str {
        match self {
            Self::BufAdd => Self::BUF_ADD,
            Self::BufEnter => Self::BUF_ENTER,
            Self::BufLeave => Self::BUF_LEAVE,
            Self::BufUnload => Self::BUF_UNLOAD,
            Self::CursorMoved => Self::CURSOR_MOVED,
            Self::CursorMovedI => Self::CURSOR_MOVED_I,
        }
    }
}

impl From<()> for ShouldDetach {
    fn from(_: ()) -> Self {
        Self::No
    }
}

impl From<Shared<Self>> for ShouldDetach {
    fn from(should_detach: Shared<Self>) -> Self {
        should_detach.get()
    }
}

impl From<ShouldDetach> for bool {
    fn from(should_detach: ShouldDetach) -> bool {
        matches!(should_detach, ShouldDetach::Yes)
    }
}

impl From<u32> for AugroupId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl api::StringOrInt for AugroupId {
    fn to_object(self) -> nvim_oxi::Object {
        self.0.to_object()
    }
}
