use core::any::type_name;

use collab_server::message::Message;
use nomad::buf_attach::BufAttachArgs;
use nomad::ctx::BufferCtx;
use nomad::{action_name, Action, ActionName, Shared, ShouldDetach};

use super::Project;
use crate::Collab;

pub(super) struct SyncReplacement {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) project: Shared<Project>,
    pub(super) should_detach: Shared<ShouldDetach>,
}

impl<'a> Action<BufferCtx<'a>> for SyncReplacement {
    const NAME: ActionName = action_name!("synchronize-replacement");
    type Args = BufAttachArgs;
    type Docs = ();
    type Module = Collab;
    type Return = ShouldDetach;

    fn execute(&mut self, args: Self::Args, _: BufferCtx<'a>) -> Self::Return {
        let message = self.project.with_mut(|project| {
            if args.actor_id == project.actor_id {
                return None;
            }

            let Some(mut file) = project.file_mut_of_buffer_id(args.buffer_id)
            else {
                unreachable!(
                    "couldn't convert BufferId to file in {}",
                    type_name::<Self>()
                );
            };

            let edit = file.sync_edited_text([args.replacement.into()]);

            let file_id = file.id();
            project.refresh_cursors(file_id);
            project.refresh_selections(file_id);

            Some(Message::EditedBuffer(edit))
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
