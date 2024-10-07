use core::cmp::Ordering;

use nvim_oxi::api;

use crate::neovim::{BufferId, Neovim};
use crate::{ActorId, Context, Emitter, Event, Shared};

/// TODO: docs.
pub struct FocusBuffer {
    focused_by: ActorId,
    id: BufferId,
}

/// TODO: docs.
pub struct FocusBufferEvent {
    send_current: bool,
    next_buffer_focused_by: Shared<Option<ActorId>>,
}

impl FocusBuffer {
    /// TODO: docs.
    pub fn focused_by(&self) -> ActorId {
        self.focused_by
    }

    /// TODO: docs.
    pub fn id(&self) -> BufferId {
        self.id.clone()
    }
}

impl PartialEq for FocusBufferEvent {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for FocusBufferEvent {}

impl PartialOrd for FocusBufferEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FocusBufferEvent {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl Event<Neovim> for FocusBufferEvent {
    type Payload = FocusBuffer;
    type SubscribeCtx = u32;

    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        _: &Context<Neovim>,
    ) -> Self::SubscribeCtx {
        if self.send_current {
            let id = BufferId::new(api::Buffer::current());

            if id.is_of_text_buffer() {
                let focused_by = self
                    .next_buffer_focused_by
                    .with_mut(Option::take)
                    .unwrap_or(ActorId::unknown());
                emitter.send(FocusBuffer { focused_by, id });
            }
        }

        let opts = api::opts::CreateAutocmdOpts::builder()
            .callback({
                let next_buffer_focused_by =
                    self.next_buffer_focused_by.clone();
                move |_| {
                    let id = BufferId::new(api::Buffer::current());

                    if id.is_of_text_buffer() {
                        let focused_by = next_buffer_focused_by
                            .with_mut(Option::take)
                            .unwrap_or(ActorId::unknown());
                        emitter.send(FocusBuffer { focused_by, id });
                    }

                    false
                }
            })
            .build();

        api::create_autocmd(["BufEnter"], &opts)
            .expect("all arguments are valid")
    }

    fn unsubscribe(&mut self, id: u32, _: &Context<Neovim>) {
        let _ = api::del_autocmd(id);
    }
}
