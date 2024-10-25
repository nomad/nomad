use core::any::type_name;

use nomad::events::{Cursor, CursorAction};
use nomad::{action_name, Action, ActionName, Shared, ShouldDetach};
use nomad_server::Message;

use super::SessionCtx;
use crate::Collab;

#[derive(Clone)]
pub(super) struct SyncCursor {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) session_ctx: Shared<SessionCtx>,
    pub(super) should_detach: Shared<ShouldDetach>,
}

impl Action for SyncCursor {
    const NAME: ActionName = action_name!("synchronize-cursor");
    type Args = Cursor;
    type Docs = ();
    type Module = Collab;
    type Return = ShouldDetach;

    fn execute(&mut self, cursor: Self::Args) -> Self::Return {
        let message = self.session_ctx.with_mut(|session_ctx| {
            if cursor.moved_by == session_ctx.actor_id {
                return None;
            }

            let Some(mut file) =
                session_ctx.file_mut_of_buffer_id(cursor.buffer_id)
            else {
                unreachable!(
                    "couldn't convert BufferId to file in {}",
                    type_name::<Self>()
                );
            };

            Some(match cursor.action {
                CursorAction::Created(byte_offset) => {
                    file.sync_created_cursor(byte_offset);
                    todo!();
                },
                CursorAction::Moved(byte_offset) => {
                    session_ctx
                        .local_cursor_mut()
                        .expect("cursor is being moved, so it must exist")
                        .sync_relocated(byte_offset.into_u64());
                    todo!();
                },
                CursorAction::Removed => {
                    session_ctx
                        .local_cursor_mut()
                        .expect("cursor is being removed, so it must exist")
                        .sync_removed();
                    todo!();
                },
            })
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
