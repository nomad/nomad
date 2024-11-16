use nvimx::ctx::NeovimCtx;
use nvimx::diagnostics::DiagnosticMessage;
use nvimx::plugin::{action_name, Action, ActionName};
use nvimx::Shared;

use crate::session_status::SessionStatus;
use crate::Collab;

#[derive(Clone)]
pub(crate) struct Yank {
    session_status: Shared<SessionStatus>,
}

impl Yank {
    pub(crate) fn new(session_status: Shared<SessionStatus>) -> Self {
        Self { session_status }
    }
}

impl Action for Yank {
    const NAME: ActionName = action_name!("yank");

    type Args = ();
    type Ctx<'a> = NeovimCtx<'a>;
    type Docs = ();
    type Module = Collab;
    type Return = ();

    fn execute<'a>(
        &'a mut self,
        _: Self::Args,
        _: NeovimCtx<'a>,
    ) -> Result<(), YankError> {
        let session_id = self.session_status.with(|s| match s {
            SessionStatus::InSession(project) => {
                Ok(project.with(|p| p.session_id()))
            },
            _ => Err(YankError::NoSession),
        })?;

        clipboard::set(session_id)?;

        Ok(())
    }

    fn docs(&self) {}
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum YankError {
    #[error("couldn't copy session ID to clipboard: {0}")]
    Clipboard(#[from] clipboard::ClipboardError),

    #[error("there's no active collaborative editing session")]
    NoSession,
}

impl From<YankError> for DiagnosticMessage {
    fn from(err: YankError) -> Self {
        let mut message = Self::new();
        message.push_str(err.to_string());
        message
    }
}
