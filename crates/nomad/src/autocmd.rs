use fxhash::FxHashMap;
use nvim_oxi::api::{self, opts, types};

use crate::buffer_id::BufferId;
use crate::ctx::{AutoCommandCtx, NeovimCtx};
use crate::diagnostics::{DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::{Action, ActorId, Module};

/// TODO: docs.
pub trait AutoCommand: Sized {
    /// TODO: docs.
    type Action: Action<
        Args: for<'a> From<(ActorId, &'a AutoCommandCtx<'a>)>,
        Return: Into<ShouldDetach>,
    >;

    /// TODO: docs.
    fn into_action(self) -> Self::Action;

    /// TODO: docs.
    fn on_event(&self) -> AutoCommandEvent;

    /// TODO: docs.
    fn on_buffer(&self) -> Option<BufferId>;

    /// TODO: docs.
    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId;

    /// TODO: docs.
    fn into_callback(
        self,
    ) -> impl for<'a> FnMut(ActorId, &'a AutoCommandCtx<'a>) -> ShouldDetach + 'static
    {
        let on_buffer = self.on_buffer();
        let mut action = self.into_action();
        move |actor_id, ctx: &AutoCommandCtx| {
            if let Some(buffer_id) = on_buffer {
                if BufferId::new(ctx.args().buffer.clone()) != buffer_id {
                    return ShouldDetach::No;
                }
            }
            let args = (actor_id, ctx).into();
            match action.execute(args).into_result() {
                Ok(res) => res.into(),
                Err(err) => {
                    let mut source = DiagnosticSource::new();
                    source
                        .push_segment(
                            <<Self::Action as Action>::Module as Module>::NAME
                                .as_str(),
                        )
                        .push_segment(Self::Action::NAME.as_str());
                    err.into().emit(Level::Error, source);
                    ShouldDetach::Yes
                },
            }
        }
    }
}

/// TODO: docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutoCommandEvent {
    BufAdd,
    BufEnter,
    BufLeave,
    BufUnload,
    CursorMoved,
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
        ctx: NeovimCtx<'static>,
    ) {
        let event = autocmd.on_event();
        let mut has_event_been_registered = true;
        let callbacks = self.inner.entry(event).or_insert_with(|| {
            has_event_been_registered = false;
            Vec::new()
        });
        callbacks.push(Box::new(autocmd.into_callback()));
        if !has_event_been_registered {
            register_autocmd::<A>(event, ctx.clone());
        }
    }
}

fn register_autocmd<A: AutoCommand>(
    event: AutoCommandEvent,
    ctx: NeovimCtx<'static>,
) {
    let augroup_id = ctx.augroup_id();

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

impl From<crate::Shared<Self>> for ShouldDetach {
    fn from(should_detach: crate::Shared<Self>) -> Self {
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
