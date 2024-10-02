use collab_server::SessionId;
use futures_util::{select, FutureExt, StreamExt};
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
pub(crate) struct Collab<E, JoinStream, StartStream> {
    pub(crate) ctx: Context<E>,
    pub(crate) config: Config,
    pub(crate) join_stream: JoinStream,
    pub(crate) start_stream: StartStream,
}

impl<E: Editor> Collab<E>
where
    JoinSession: Event<E, Payload = SessionId>,
    StartSession: Event<E>,
{
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
                _ = self.start_stream.next().fuse() => self.start_session(),
                &id = self.join_stream.next().fuse() => self.join_session(id),
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
