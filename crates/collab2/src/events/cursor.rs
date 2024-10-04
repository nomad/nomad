use nomad::neovim::Neovim;
use nomad::{Context, Emitter, Event};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CursorEvent {}

impl Event<Neovim> for CursorEvent {
    type Payload = Cursor;
    type SubscribeCtx = ();

    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        _ctx: &Context<Neovim>,
    ) {
        todo!();
    }
}

#[derive(Clone)]
pub(crate) struct Cursor {}
