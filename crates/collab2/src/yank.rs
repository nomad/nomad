use core::marker::PhantomData;

use collab_server::SessionId;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::{self, Name};
use nvimx2::{AsyncCtx, fs};
use smallvec::SmallVec;

use crate::backend::{ActionForSelectedSession, CollabBackend};
use crate::collab::Collab;
use crate::sessions::{SessionState, Sessions};

/// An `Action` that pastes the [`SessionId`] of any active session to the
/// user's clipboard.
#[derive(Clone)]
pub struct Yank {
    session_selector: SessionSelector,
}

#[derive(Clone)]
pub(crate) struct SessionSelector {
    sessions: Sessions,
}

impl<B: CollabBackend> AsyncAction<B> for Yank {
    const NAME: Name = "yank";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), YankError<B>> {
        let Some((_, session_id)) = self
            .session_selector
            .select(ctx)
            .await
            .map_err(YankError::NoActiveSession)?
        else {
            return Ok(());
        };

        B::copy_session_id(session_id, ctx)
            .await
            .map_err(YankError::PasteSessionId)
    }
}

impl SessionSelector {
    pub(crate) fn new(sessions: Sessions) -> Self {
        Self { sessions }
    }

    pub(crate) async fn select<B: CollabBackend>(
        &self,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Option<(fs::AbsPathBuf, SessionId)>, NoActiveSessionError<B>>
    {
        let active_sessions = self
            .sessions
            .iter()
            .filter_map(|(root, state)| match state {
                SessionState::Active(session_id) => Some((root, session_id)),
                _ => None,
            })
            .collect::<SmallVec<[_; 1]>>();

        let session = match &*active_sessions {
            [] => return Err(NoActiveSessionError::new()),
            [single] => single,
            sessions => match B::select_session(
                sessions,
                ActionForSelectedSession::CopySessionId,
                ctx,
            )
            .await
            {
                Some(session) => session,
                None => return Ok(None),
            },
        };

        Ok(Some(session.clone()))
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
        Self {
            session_selector: SessionSelector::new(collab.sessions.clone()),
        }
    }
}

impl<B: CollabBackend> YankError<B> {
    fn no_active_session() -> Self {
        Self::NoActiveSession(NoActiveSessionError::new())
    }
}

impl<B> NoActiveSessionError<B> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
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
