use futures_util::{select, FutureExt, Stream, StreamExt};
use nomad::{
    module_name,
    Context,
    Editor,
    Event,
    JoinHandle,
    Module,
    ModuleName,
    Spawner,
    Subscription,
};

use crate::{CollabEditor, Config, Session, SessionId};

/// TODO: docs.
pub(crate) struct Collab<E: CollabEditor> {
    pub(crate) ctx: Context<E>,
    pub(crate) config: Config,
    pub(crate) config_stream: E::ConfigStream,
    pub(crate) join_stream: E::JoinStream,
    pub(crate) start_stream: E::StartStream,
}

impl<E: CollabEditor> Collab<E> {
    pub(crate) async fn run(&mut self) {
        loop {
            select! {
                _ = self.start_stream.next().fuse() => self.start_session(),
                session_id = self.join_stream.next().fuse() => {
                    let session_id = session_id.expect("never ends");
                    self.join_session(session_id)
                },
            }
        }
    }

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
