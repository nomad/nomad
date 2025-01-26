use core::cell::Cell;

use auth::AuthInfos;
use flume::{Receiver, Sender};
use nvimx2::module::{ApiCtx, Module};
use nvimx2::notify::Name;
use nvimx2::{NeovimCtx, Shared};

use crate::CollabBackend;
use crate::config::Config;
use crate::leave::LeaveChannels;
use crate::session::Session;
use crate::sessions::Sessions;
use crate::start::Start;
use crate::yank::Yank;

/// TODO: docs.
pub struct Collab<B: CollabBackend> {
    pub(crate) auth_infos: Shared<Option<AuthInfos>>,
    pub(crate) config: Shared<Config>,
    pub(crate) leave_channels: LeaveChannels,
    pub(crate) session_tx: Sender<Session<B>>,
    pub(crate) sessions: Sessions,
    session_rx: Cell<Option<Receiver<Session<B>>>>,
}

impl<B: CollabBackend> Collab<B> {
    /// Returns a new instance of the [`Start`] action.
    pub fn start(&self) -> Start<B> {
        self.into()
    }

    /// Returns a new instance of the [`Yank`] action.
    pub fn yank(&self) -> Yank {
        self.into()
    }
}

impl<B: CollabBackend> Module<B> for Collab<B> {
    const NAME: Name = "collab";

    type Config = Config;

    fn api(&self, ctx: &mut ApiCtx<B>) {
        ctx.with_command(self.start())
            .with_command(self.yank())
            .with_function(self.start())
            .with_function(self.yank());
    }

    fn on_init(&self, ctx: &mut NeovimCtx<B>) {
        let session_rx = self
            .session_rx
            .replace(None)
            .expect("`Module::on_init()` is only called once");

        ctx.spawn_local(async move |ctx| {
            while let Ok(session) = session_rx.recv_async().await {
                ctx.spawn_local(async move |ctx| {
                    if let Err(err) = session.run(ctx).await {
                        ctx.emit_err(err);
                    }
                });
            }
        });
    }

    fn on_new_config(&self, new_config: Self::Config, _: &mut NeovimCtx<B>) {
        self.config.set(new_config);
    }
}

impl<B: CollabBackend> From<&auth::Auth> for Collab<B> {
    fn from(auth: &auth::Auth) -> Self {
        let (session_tx, session_rx) = flume::bounded(1);
        Self {
            auth_infos: auth.infos().clone(),
            config: Default::default(),
            leave_channels: Default::default(),
            session_tx,
            sessions: Default::default(),
            session_rx: Cell::new(Some(session_rx)),
        }
    }
}
