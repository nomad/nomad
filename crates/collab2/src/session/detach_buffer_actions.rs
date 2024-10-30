use collab_server::message::Message;
use nomad::autocmds::BufUnloadArgs;
use nomad::ctx::NeovimCtx;
use nomad::{action_name, Action, ActionName, BufferId, Shared, ShouldDetach};

use super::Project;
use crate::Collab;

pub(super) struct DetachBufferActions {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) project: Shared<Project>,
}

impl DetachBufferActions {
    fn detach_actions(&mut self, buffer_id: BufferId) {
        self.project.with_mut(|project| {
            if let Some(should_detach) = project.buffer_actions.get(&buffer_id)
            {
                should_detach.set(ShouldDetach::Yes);
                project
                    .remote_tooltips
                    .retain(|_, tooltip| tooltip.buffer_id() != buffer_id);
                project
                    .remote_selections
                    .retain(|_, selection| selection.buffer_id() != buffer_id);
            }
        });
    }
}

impl<'a> Action<NeovimCtx<'a>> for DetachBufferActions {
    const NAME: ActionName = action_name!("detach-buffer-actions");
    type Args = BufUnloadArgs;
    type Docs = ();
    type Module = Collab;
    type Return = ();

    fn execute(&mut self, args: Self::Args, _: NeovimCtx<'a>) {
        self.detach_actions(args.buffer_id);
    }

    fn docs(&self) {}
}
