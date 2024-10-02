use collab_server::SessionId;
use futures_util::{select, FutureExt, StreamExt};
use nomad2::neovim::{command, function, ModuleApi, Neovim};
use nomad2::{
    module_name,
    Api,
    Context,
    Editor,
    Event,
    JoinHandle,
    Module,
    ModuleName,
    Spawner,
    Subscription,
};

use crate::events::{JoinSession, StartSession};
use crate::{Config, Session};

/// TODO: docs.
pub struct Collab<E: Editor> {
    pub(crate) ctx: Context<E>,
    pub(crate) config: Config,
    pub(crate) join_sub: Subscription<JoinSession, E>,
    pub(crate) start_sub: Subscription<StartSession, E>,
}

impl<E: Editor> Collab<E>
where
    JoinSession: Event<E, Payload = SessionId>,
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
                println!("{err:?}");
            }
        };

        self.ctx.spawner().spawn(fut).detach();
    }

    async fn run(&mut self) {
        loop {
            select! {
                _ = self.start_sub.recv().fuse() => self.start_session(),
                &id = self.join_sub.recv().fuse() => self.join_session(id),
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
                println!("{err:?}");
            }
        };

        self.ctx.spawner().spawn(fut).detach();
    }
}

impl Module<Neovim> for Collab<Neovim> {
    const NAME: ModuleName = Self::NAME;

    type Config = Config;

    fn init(ctx: &Context<Neovim>) -> (Self, ModuleApi) {
        let (join_cmd, join_cmd_sub) = command::<JoinSession>(ctx);
        let (start_cmd, start_cmd_sub) = command::<StartSession>(ctx);

        let (join_fn, join_fn_sub) = function::<JoinSession>(ctx);
        let (start_fn, start_fn_sub) = function::<StartSession>(ctx);

        let api = ModuleApi::new::<Self>()
            // .with_default_command(Auth)
            .with_command(join_cmd)
            .with_command(start_cmd)
            .with_function(join_fn)
            .with_function(start_fn);

        let this = Self(Collab {
            ctx: ctx.clone(),
            config: Config::default(),
            join_sub: join_cmd_sub.zip(join_fn_sub),
            start_sub: start_cmd_sub.zip(start_fn_sub),
        });

        (this, api)
    }

    async fn run(&mut self, _: &Context<Neovim>) {
        self.run().await;
    }
}
