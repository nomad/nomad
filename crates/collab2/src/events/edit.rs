use nomad::neovim::Neovim;
use nomad::{Context, Emitter, Event};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EditEvent {}

impl Event<Neovim> for EditEvent {
    type Payload = Edit;
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
pub(crate) struct Edit {}
