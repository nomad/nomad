use fxhash::FxHashMap;
use nvim_oxi::api::{self, opts, types};
use smallvec::SmallVec;

use crate::ctx::{AutocmdCtx, NeovimCtx};
use crate::diagnostics::{DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::{Action, ActorId, Module, Shared};

/// TODO: docs.
pub trait Autocmd: Sized {
    type Action: Action<
            Args: for<'a> From<(ActorId, &'a AutocmdCtx<'a>)>,
            Return: Into<ShouldDetach>,
        > + Clone;

    /// TODO: docs.
    fn into_action(self) -> Self::Action;

    /// TODO: docs.
    fn on_events(&self) -> impl IntoIterator<Item = AutocmdEvent>;

    /// TODO: docs.
    fn take_actor_id(ctx: &AutocmdCtx<'_>) -> ActorId;
}

/// TODO: docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutocmdEvent {
    BufAdd,
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

/// TODO: docs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AutocmdId(u32);

type AutocmdCallback =
    Box<dyn for<'a> FnMut(ActorId, &'a AutocmdCtx<'a>) -> ShouldDetach>;

#[derive(Clone)]
pub(crate) struct AutocmdMap {
    map: Shared<FxHashMap<AutocmdEvent, Vec<AutocmdCallback>>>,
}

impl AutocmdMap {
    pub(crate) fn register<A: Autocmd>(
        &mut self,
        autocmd: A,
        ctx: NeovimCtx<'_>,
    ) {
        let mut events = autocmd
            .on_events()
            .into_iter()
            .map(|event| (event, true))
            .collect::<SmallVec<[_; 4]>>();
        let events_len = events.len();
        let action = autocmd.into_action();

        self.map.with_mut(|map| {
            for (event, has_event_been_registered) in events.iter_mut() {
                let callbacks = map.entry(*event).or_insert_with(|| {
                    *has_event_been_registered = false;
                    Vec::with_capacity(events_len)
                });
                let mut action = action.clone();
                let callback = move |actor_id, ctx: &AutocmdCtx| {
                    let args = (actor_id, ctx).into();
                    match action.execute(args).into_result() {
                        Ok(res) => res.into(),
                        Err(err) => {
                            let mut source = DiagnosticSource::new();
                            source
                                .push_segment(<<A::Action as Action>::Module as Module>::NAME.as_str())
                                .push_segment(A::Action::NAME.as_str());
                            err.into().emit(Level::Error, source);
                            ShouldDetach::Yes
                        },
                    }
                };
                callbacks.push(Box::new(callback));
            }
        });

        for (event, has_event_been_registered) in events {
            if !has_event_been_registered {
                self.register_autocmd::<A>(event, ctx.to_static());
            }
        }
    }

    fn register_autocmd<A: Autocmd>(
        &self,
        event: AutocmdEvent,
        ctx: NeovimCtx<'static>,
    ) {
        let this = self.clone();

        let augroup_id = ctx.augroup_id();

        let callback = move |args: types::AutocmdCallbackArgs| {
            debug_assert_eq!(args.event, event.as_str());
            let ctx = AutocmdCtx::new(args, event, ctx.as_ref());
            let actor_id = ActorId::unknown();
            this.map.with_mut(|map| {
                let Some(callbacks) = map.get_mut(&event) else {
                    panic!(
                        "Neovim executed callback for unregistered event \
                         {event:?}"
                    )
                };
                let mut idx = 0;
                loop {
                    let Some(callback) = callbacks.get_mut(idx) else {
                        break;
                    };
                    if callback(actor_id, &ctx).into() {
                        callbacks.remove(idx);
                    } else {
                        idx += 1;
                    }
                }
                let should_detach = callbacks.is_empty();
                if should_detach {
                    map.remove(&event);
                }
                should_detach
            })
        };

        let opts = opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .callback(nvim_oxi::Function::from_fn_mut(callback))
            .build();

        let _ = api::create_autocmd([event.as_str()], &opts)
            .expect("all arguments are valid");
    }
}

impl AutocmdEvent {
    const BUF_ADD: &'static str = "BufAdd";

    fn as_str(&self) -> &'static str {
        match self {
            Self::BufAdd => Self::BUF_ADD,
        }
    }
}

impl From<()> for ShouldDetach {
    fn from(_: ()) -> Self {
        Self::No
    }
}

impl From<bool> for ShouldDetach {
    fn from(b: bool) -> Self {
        if b {
            Self::Yes
        } else {
            Self::No
        }
    }
}

impl From<ShouldDetach> for bool {
    fn from(should_detach: ShouldDetach) -> bool {
        matches!(should_detach, ShouldDetach::Yes)
    }
}

impl From<u32> for AutocmdId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl api::StringOrInt for AugroupId {
    fn to_object(self) -> nvim_oxi::Object {
        self.0.to_object()
    }
}
