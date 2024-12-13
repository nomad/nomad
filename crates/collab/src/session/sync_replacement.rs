use collab_server::message::Message;
use nvimx::ctx::{ShouldDetach, TextBufferCtx};
use nvimx::event::OnBytesArgs;
use nvimx::plugin::{action_name, Action, ActionName};
use nvimx::Shared;

use super::Project;
use crate::Collab;

pub(super) struct SyncReplacement {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) project: Shared<Project>,
    pub(super) should_detach: Shared<ShouldDetach>,
}

impl Action for SyncReplacement {
    const NAME: ActionName = action_name!("synchronize-replacement");
    type Args = OnBytesArgs;
    type Ctx<'a> = TextBufferCtx<'a>;
    type Docs = ();
    type Module = Collab;
    type Return = ShouldDetach;

    fn execute<'a>(
        &'a mut self,
        args: Self::Args,
        _: Self::Ctx<'a>,
    ) -> Self::Return {
        let message = self.project.with_mut(|proj| {
            if args.actor_id == proj.actor_id {
                return None;
            }

            let Some(mut file) = proj.file(args.buffer_id) else {
                panic!("couldn't get file of {:?}", args.buffer_id)
            };

            Some(file.sync_replacement(args.replacement).into())
        });

        if let Some(message) = message {
            if self.message_tx.send(message).is_err() {
                self.should_detach.set(ShouldDetach::Yes);
            }
        }

        self.should_detach.get()
    }

    fn docs(&self) {}
}
