use fxhash::FxHashMap;
use nvim_oxi::api::{self, opts, types};
use smallvec::SmallVec;

use crate::ctx::{AutoCommandCtx, NeovimCtx};
use crate::diagnostics::{DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::{Action, ActorId, Module};

/// TODO: docs.
pub trait AutoCommand {
    type Action: Action<
            Args: for<'a> From<(ActorId, &'a AutoCommandCtx<'a>)>,
            Return: Into<ShouldDetach>,
        > + Clone;

    /// TODO: docs.
    fn into_action(self) -> Self::Action;

    /// TODO: docs.
    fn on_events(&self) -> impl IntoIterator<Item = AutoCommandEvent>;

    /// TODO: docs.
    fn take_actor_id(ctx: &AutoCommandCtx<'_>) -> ActorId;
}

/// TODO: docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutoCommandEvent {
    BufAdd,
    BufEnter,
    BufUnload,
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
        let mut events = autocmd
            .on_events()
            .into_iter()
            .map(|event| (event, true))
            .collect::<SmallVec<[_; 4]>>();
        let events_len = events.len();

        let mut action = autocmd.into_action();
        let callback = move |actor_id, ctx: &AutoCommandCtx| {
            let args = (actor_id, ctx).into();
            match action.execute(args).into_result() {
                Ok(res) => res.into(),
                Err(err) => {
                    let mut source = DiagnosticSource::new();
                    source
                        .push_segment(
                            <<A::Action as Action>::Module as Module>::NAME
                                .as_str(),
                        )
                        .push_segment(A::Action::NAME.as_str());
                    err.into().emit(Level::Error, source);
                    ShouldDetach::Yes
                },
            }
        };

        for (event, has_event_been_registered) in events.iter_mut() {
            let callbacks = self.inner.entry(*event).or_insert_with(|| {
                *has_event_been_registered = false;
                Vec::with_capacity(events_len)
            });
            callbacks.push(Box::new(callback.clone()));
        }

        for (event, has_event_been_registered) in events {
            if !has_event_been_registered {
                register_autocmd::<A>(event, ctx.clone());
            }
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
        let autocmd_ctx = AutoCommandCtx::new(args, event, ctx.as_ref());
        let actor_id = ActorId::unknown();
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
                    callbacks.remove(idx);
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
    const BUF_UNLOAD: &'static str = "BufUnload";

    fn as_str(&self) -> &'static str {
        match self {
            Self::BufAdd => Self::BUF_ADD,
            Self::BufEnter => Self::BUF_ENTER,
            Self::BufUnload => Self::BUF_UNLOAD,
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
