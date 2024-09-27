use collab_server::SessionId;
use futures_util::{select, FutureExt};
use nomad2::neovim::{Neovim, NeovimFunction, NeovimModuleApi};
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
    ctx: Context<E>,
    config: Config,
    join_sub: Subscription<JoinSession, E>,
    start_sub: Subscription<StartSession, E>,
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

    fn init(ctx: &Context<Neovim>) -> NeovimModuleApi<Self> {
        // let join_cmd_sub = ctx.with_editor(|nvim| {
        //     nvim.create_command(JoinSession::NAME, JoinSession)
        // });
        //
        // let start_cmd_sub = ctx.with_editor(|nvim| {
        //     nvim.create_command(StartSession::NAME, StartSession)
        // });

        // let start_cmd_sub = NeovimCommand::builder()
        //     .name(StartSession::NAME)
        //     .on_execute(StartSession)
        //     .build(ctx.clone());

        let (join_fn, join_fn_sub) = NeovimFunction::builder()
            .name(JoinSession::NAME)
            .on_execute(JoinSession)
            .build(ctx.clone());

        let (start_fn, start_fn_sub) = NeovimFunction::builder()
            .name(StartSession::NAME)
            .on_execute(StartSession)
            .build(ctx.clone());

        let collab = Self {
            ctx: ctx.clone(),
            config: Config::default(),
            join_sub: join_fn_sub,
            start_sub: start_fn_sub,
        };

        NeovimModuleApi::new(collab)
            .with_function(join_fn)
            .with_function(start_fn)
    }

    async fn run(&mut self, _: &Context<Neovim>) {
        self.run().await;
    }
}
