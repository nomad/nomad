use core::cmp::Ordering;

use nvim_oxi::api;

use crate::neovim::{BufferId, Neovim};
use crate::{ActorId, Context, Emitter, Event, Shared};

/// TODO: docs.
pub struct CloseBuffer {
    closed_by: ActorId,
    id: BufferId,
}

/// TODO: docs.
pub struct CloseBufferEvent {
    next_buffer_closed_by: Shared<Option<ActorId>>,
}

impl CloseBuffer {
    /// TODO: docs.
    pub fn closed_by(&self) -> ActorId {
        self.closed_by
    }

    /// TODO: docs.
    pub fn id(&self) -> BufferId {
        self.id.clone()
    }
}

impl PartialEq for CloseBufferEvent {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for CloseBufferEvent {}

impl PartialOrd for CloseBufferEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CloseBufferEvent {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl Event<Neovim> for CloseBufferEvent {
    type Payload = CloseBuffer;
    type SubscribeCtx = u32;

    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        _: &Context<Neovim>,
    ) -> Self::SubscribeCtx {
        let opts = api::opts::CreateAutocmdOpts::builder()
            .callback({
                let next_buffer_closed_by = self.next_buffer_closed_by.clone();
                move |args: api::types::AutocmdCallbackArgs| {
                    let id = BufferId::new(args.buffer);

                    if id.is_of_text_buffer() {
                        let closed_by = next_buffer_closed_by
                            .with_mut(Option::take)
                            .unwrap_or(ActorId::unknown());
                        emitter.send(CloseBuffer { closed_by, id });
                    }

                    false
                }
            })
            .build();

        api::create_autocmd(["BufUnload"], &opts)
            .expect("all arguments are valid")
    }

    fn unsubscribe(&mut self, id: u32, _: &Context<Neovim>) {
        let _ = api::del_autocmd(id);
    }
}
