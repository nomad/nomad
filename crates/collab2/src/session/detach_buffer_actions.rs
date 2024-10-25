use nomad::autocmds::BufUnloadArgs;
use nomad::{action_name, Action, ActionName, BufferId, Shared, ShouldDetach};
use nomad_server::Message;

use super::SessionCtx;
use crate::Collab;

pub(super) struct DetachBufferActions {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) session_ctx: Shared<SessionCtx>,
}

impl DetachBufferActions {
    fn detach_actions(&mut self, buffer_id: BufferId) {
        self.session_ctx.with_mut(|session_ctx| {
            if let Some(should_detach) =
                session_ctx.buffer_actions.get(&buffer_id)
            {
                should_detach.set(ShouldDetach::Yes);

                todo!("remove tooltips for cursors in this buffer");
            }
        });
    }
}

impl Action for DetachBufferActions {
    const NAME: ActionName = action_name!("detach-buffer-actions");
    type Args = BufUnloadArgs;
    type Docs = ();
    type Module = Collab;
    type Return = ();

    fn execute(&mut self, args: Self::Args) {
        self.detach_actions(args.buffer_id);
    }

    fn docs(&self) {}
}
