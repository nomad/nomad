use core::marker::PhantomData;

use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, notify};
use smallvec::SmallVec;

use crate::backend::CollabBackend;
use crate::collab::Collab;
use crate::sessions::{SessionState, Sessions};

/// An `Action` that pastes the [`SessionId`] of any active session to the
/// user's clipboard.
#[derive(Clone)]
pub struct Yank {
    sessions: Sessions,
}

impl<B: CollabBackend> AsyncAction<B> for Yank {
    const NAME: Name = "yank";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), YankError<B>> {
        let active_sessions = self
            .sessions
            .iter()
            .filter_map(|(root, state)| match state {
                SessionState::Active(session_id) => Some((root, session_id)),
                _ => None,
            })
            .collect::<SmallVec<[_; 1]>>();

        if active_sessions.is_empty() {
            return Err(YankError::no_active_session());
        }

        todo!();
    }
}

/// The type of error that can occur when [`Yank`]ing fails.
pub enum YankError<B: CollabBackend> {
    NoActiveSession(NoActiveSessionError<B>),
    PasteSessionId(B::PasteSessionIdError),
}

pub struct NoActiveSessionError<B>(PhantomData<B>);

impl<B: CollabBackend> ToCompletionFn<B> for Yank {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> From<&Collab<B>> for Yank {
    fn from(collab: &Collab<B>) -> Self {
        Self { sessions: collab.sessions.clone() }
    }
}

impl<B: CollabBackend> YankError<B> {
    fn no_active_session() -> Self {
        Self::NoActiveSession(NoActiveSessionError(PhantomData))
    }
}

impl<B: CollabBackend> notify::Error for YankError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            YankError::NoActiveSession(err) => err.to_message(),
            YankError::PasteSessionId(err) => err.to_message(),
        }
    }
}

impl<B> notify::Error for NoActiveSessionError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        let msg = "there's no active collaborative editing session";
        (notify::Level::Error, notify::Message::from_str(msg))
    }
}
