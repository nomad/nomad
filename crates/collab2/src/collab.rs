use core::cell::Cell;

use async_channel::{Receiver, Sender};
use auth::AuthInfos;
use nvimx2::module::{ApiCtx, Module};
use nvimx2::notify::Name;
use nvimx2::{NeovimCtx, Shared};

use crate::CollabBackend;
use crate::config::Config;
use crate::start::Start;

/// TODO: docs.
pub struct Collab {
    pub(crate) auth_infos: Shared<Option<AuthInfos>>,
    pub(crate) config: Shared<Config>,
    pub(crate) session_tx: Sender<()>,
    session_rx: Cell<Option<Receiver<()>>>,
}

impl Collab {
    /// Returns a new instance of the [`Start`] action.
    pub fn start(&self) -> Start {
        self.into()
    }
}

impl<B: CollabBackend> Module<B> for Collab {
    const NAME: Name = "collab";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<B>) {
        ctx.with_function(self.start());
    }

    fn on_init(&self, ctx: &mut NeovimCtx<B>) {
        let session_rx = self
            .session_rx
            .replace(None)
            .expect("`Module::on_init()` is only called once");

        ctx.spawn_local(
            async move |_ctx| {
                while let Ok(()) = session_rx.recv().await {}
            },
        );
    }

    fn on_new_config(&self, new_config: Self::Config, _: &mut NeovimCtx<B>) {
        self.config.set(new_config);
    }
}

impl From<&auth::Auth> for Collab {
    fn from(auth: &auth::Auth) -> Self {
        let (session_tx, session_rx) = async_channel::bounded(1);
        Self {
            auth_infos: auth.infos().clone(),
            config: Default::default(),
            session_tx,
            session_rx: Cell::new(Some(session_rx)),
        }
    }
}
