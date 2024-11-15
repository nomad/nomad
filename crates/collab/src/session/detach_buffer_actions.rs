use nvimx::action::{action_name, Action, ActionName};
use nvimx::common::Shared;
use nvimx::ctx::{BufferId, NeovimCtx, ShouldDetach};
use nvimx::event::BufUnloadArgs;

use super::Project;
use crate::Collab;

pub(super) struct DetachBufferActions {
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

impl Action for DetachBufferActions {
    const NAME: ActionName = action_name!("detach-buffer-actions");
    type Args = BufUnloadArgs;
    type Ctx<'a> = NeovimCtx<'a>;
    type Docs = ();
    type Return = ();

    fn execute(&mut self, args: Self::Args, _: NeovimCtx<'a>) {
        self.detach_actions(args.buffer_id);
    }

    fn docs(&self) {}
}

// [mad.BufUnload.detach-buffer-actions]
