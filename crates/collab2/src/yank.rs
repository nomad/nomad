use nvimx2::AsyncCtx;
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;

use crate::backend::CollabBackend;
use crate::collab::Collab;
use crate::sessions::Sessions;

/// An `Action` that pastes the [`SessionId`] of any active session to the
/// user's clipboard.
#[derive(Clone)]
pub struct Yank {
    _sessions: Sessions,
}

impl<B: CollabBackend> AsyncAction<B> for Yank {
    const NAME: Name = "yank";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), B::PasteSessionIdError> {
        todo!();
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Yank {
    fn to_completion_fn(&self) {}
}

impl<B: CollabBackend> From<&Collab<B>> for Yank {
    fn from(collab: &Collab<B>) -> Self {
        Self { _sessions: collab.sessions.clone() }
    }
}
