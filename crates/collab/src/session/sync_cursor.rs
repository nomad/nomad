use collab_server::message::Message;
use nvimx::ctx::{BufferCtx, ShouldDetach};
use nvimx::event::{CursorArgs, CursorKind};
use nvimx::plugin::{action_name, Action, ActionName};
use nvimx::Shared;

use super::Project;
use crate::Collab;

#[derive(Clone)]
pub(super) struct SyncCursor {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) project: Shared<Project>,
    pub(super) should_detach: Shared<ShouldDetach>,
}

impl Action for SyncCursor {
    const NAME: ActionName = action_name!("synchronize-cursor");

    type Args = CursorArgs;
    type Ctx<'a> = BufferCtx<'a>;
    type Docs = ();
    type Module = Collab;
    type Return = ShouldDetach;

    fn execute<'a>(
        &'a mut self,
        cursor: Self::Args,
        _: Self::Ctx<'a>,
    ) -> Self::Return {
        let maybe_message = self.project.with_mut(|proj| {
            if cursor.moved_by == proj.actor_id {
                return None;
            }

            Some(match cursor.kind {
                CursorKind::Created(offset) => {
                    let Some(mut file) = proj.file(cursor.buffer_id) else {
                        panic!("couldn't get file of {:?}", cursor.buffer_id)
                    };
                    file.sync_created_cursor(offset).into()
                },
                CursorKind::Moved(offset) => {
                    proj.local_cursor().sync_relocated(offset)?.into()
                },
                CursorKind::Removed => {
                    proj.local_cursor().sync_removed().into()
                },
            })
        });

        if let Some(message) = maybe_message {
            if self.message_tx.send(message).is_err() {
                self.should_detach.set(ShouldDetach::Yes);
            }
        }

        self.should_detach.get()
    }

    fn docs(&self) {}
}
