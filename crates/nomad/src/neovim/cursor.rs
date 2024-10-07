use core::cmp::Ordering;

use nvim_oxi::api;

use super::{BufferId, Neovim, Point};
use crate::{ActorId, ByteOffset, Context, Emitter, Event, Shared};

/// TODO: docs.
#[derive(Clone)]
pub struct Cursor {
    action: CursorAction,
    moved_by: ActorId,
}

impl Cursor {
    /// TODO: docs.
    pub fn action(&self) -> CursorAction {
        self.action
    }

    /// TODO: docs.
    pub fn moved_by(&self) -> ActorId {
        self.moved_by
    }
}

/// TODO: docs.
#[derive(Clone, Copy)]
pub enum CursorAction {
    /// The cursor has been moved into the buffer at the given point.
    Created(Point),

    /// The cursor has been moved to the given point.
    Moved(Point),

    /// The cursor has been moved away from the buffer.
    Removed,
}

/// TODO: docs.
pub struct CursorEvent {
    pub(super) id: BufferId,
    pub(super) next_cursor_moved_by: Shared<Option<ActorId>>,
}

impl PartialEq for CursorEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id.cmp(&other.id) == Ordering::Equal
    }
}

impl Eq for CursorEvent {}

impl PartialOrd for CursorEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CursorEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Event<Neovim> for CursorEvent {
    type Payload = Cursor;
    type SubscribeCtx = Vec<u32>;

    #[allow(clippy::too_many_lines)]
    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        _: &Context<Neovim>,
    ) -> Self::SubscribeCtx {
        let just_entered_buf = Shared::new(false);

        let cursor_moved_opts = api::opts::CreateAutocmdOpts::builder()
            .buffer(self.id.as_nvim().clone())
            .callback({
                let just_entered_buf = just_entered_buf.clone();
                let next_cursor_moved_by = self.next_cursor_moved_by.clone();
                let emitter = emitter.clone();
                move |_| {
                    let (row, col) = api::Window::current()
                        .get_cursor()
                        .expect("never fails(?)");

                    let point = Point {
                        line_idx: row - 1,
                        byte_offset: ByteOffset::new(col),
                    };

                    let just_entered_buf =
                        just_entered_buf.with_mut(|entered| {
                            let just_entered = *entered;
                            *entered = false;
                            just_entered
                        });

                    let action = if just_entered_buf {
                        CursorAction::Created(point)
                    } else {
                        CursorAction::Moved(point)
                    };

                    let moved_by = next_cursor_moved_by
                        .with_mut(Option::take)
                        .unwrap_or(ActorId::unknown());

                    emitter.send(Cursor { action, moved_by });
                    false
                }
            })
            .build();

        let cursor_moved_id = api::create_autocmd(
            ["CursorMoved", "CursorMovedI"],
            &cursor_moved_opts,
        )
        .expect("all arguments are valid");

        let buf_enter_opts = api::opts::CreateAutocmdOpts::builder()
            .buffer(self.id.as_nvim().clone())
            .callback({
                let just_entered_buf = just_entered_buf.clone();
                move |_| {
                    just_entered_buf.set(true);
                    false
                }
            })
            .build();

        let buf_entered_id =
            api::create_autocmd(["BufEnter"], &buf_enter_opts)
                .expect("all arguments are valid");

        let buf_leave_opts = api::opts::CreateAutocmdOpts::builder()
            .buffer(self.id.as_nvim().clone())
            .callback({
                move |_| {
                    emitter.send(Cursor {
                        action: CursorAction::Removed,
                        moved_by: ActorId::unknown(),
                    });
                    false
                }
            })
            .build();

        let buf_leave_id = api::create_autocmd(["BufLeave"], &buf_leave_opts)
            .expect("all arguments are valid");

        vec![cursor_moved_id, buf_entered_id, buf_leave_id]
    }

    fn unsubscribe(
        &mut self,
        autocmd_ids: Self::SubscribeCtx,
        _: &Context<Neovim>,
    ) {
        for id in autocmd_ids {
            // Will fail if the autocmd has already been deleted by the user or
            // another plugin.
            let _ = api::del_autocmd(id);
        }
    }
}
