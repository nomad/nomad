use collab_server::SessionId;
use futures_util::{select, FutureExt, StreamExt};
use nomad2::neovim::Neovim;
use nomad2::{
    module_name,
    Api,
    Context,
    Editor,
    Event,
    Module,
    ModuleName,
    Subscription,
};

use crate::events::{JoinSession, StartSession};
use crate::{Config, Session};

/// TODO: docs.
pub struct Collab<E: Editor> {
    ctx: Context<E>,
    config: Config,
    join_sub: Subscription<JoinSession, E>,
    start_sub: Subscription<StartSession, E>,
}

impl<E: Editor> Collab<E>
where
    JoinSession: Event<E>,
    StartSession: Event<E>,
{
    const NAME: ModuleName = module_name!("collab");

    fn join_session(&self, id: SessionId) {
        let ctx = self.ctx.clone();
        let config = self.config.clone();

        let fut = async move {
            let session = match Session::join(id, config, ctx).await {
                Ok(session) => session,
                Err(err) => {
                    println!("{err:?}");
                    return;
                },
            };

            if let Err(err) = session.run().await {
                println!("{err}");
            }
        };

        self.ctx.spawner().spawn(fut).detach();
    }

    async fn run(&mut self) {
        loop {
            select! {
                _ = self.start_sub.next().fuse() => self.start_session(),
                id = self.join_sub.next().fuse() => self.join_session(id),
            }
        }
    }

    fn start_session(&self) {
        let ctx = self.ctx.clone();
        let config = self.config.clone();

        let fut = async move {
            let session = match Session::start(config, ctx).await {
                Ok(session) => session,
                Err(err) => {
                    println!("{err:?}");
                    return;
                },
            };

            if let Err(err) = session.run().await {
                println!("{err}");
            }
        };

        self.ctx.spawner().spawn(fut).detach();
    }
}

impl Module<Neovim> for Collab<Neovim> {
    const NAME: ModuleName = Self::NAME;

    type Config = Config;

    fn init(_ctx: &Context<Neovim>) -> Neovim::ModuleApi<Self> {
        todo!();
    }

    async fn run(&mut self, _: &Context<Neovim>) {
        self.run().await;
    }
}
