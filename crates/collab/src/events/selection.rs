use nomad::neovim::Neovim;
use nomad::{Context, Emitter, Event};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectionEvent {}

impl Event<Neovim> for SelectionEvent {
    type Payload = Selection;
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
pub(crate) struct Selection {}
