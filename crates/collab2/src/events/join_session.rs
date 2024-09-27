use collab_server::SessionId;
use nomad2::neovim::Neovim;
use nomad2::{Context, Emitter, Event};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct JoinSession;

impl Event<Neovim> for JoinSession {
    type Payload = SessionId;
    type SubscribeCtx = ();

    fn subscribe(
        &self,
        _emitter: Emitter<Self::Payload>,
        _ctx: &Context<Neovim>,
    ) {
        todo!()
    }
}
