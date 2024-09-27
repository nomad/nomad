use nomad2::{Context, Emitter, Event, Neovim};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StartSession;

impl StartSession {
    pub(crate) const NAME: &str = "start";
}

impl Event<Neovim> for StartSession {
    type Payload = ();
    type SubscribeCtx = ();

    fn subscribe(
        &self,
        _emitter: Emitter<Self::Payload>,
        _ctx: &Context<Neovim>,
    ) {
        todo!()
    }
}
