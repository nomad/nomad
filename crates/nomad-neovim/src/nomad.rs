use core::error::Error;

use abs_path::{AbsPathBuf, NodeName, node};
use ed::module::{ApiCtx, Empty, Module};
use ed::notify::Name;
use ed::plugin::Plugin;
use ed::{Borrowed, Context};
use either::Either;
use neovim::Neovim;
use tracing_subscriber::layer::SubscriberExt;

use crate::file_appender::FileAppender;

pub(crate) struct Nomad;

impl Nomad {
    /// The prefix for the log file names.
    pub(crate) const LOG_FILENAME_PREFIX: &NodeName = node!("nomad.log");

    /// Returns the directory under which the log files should be stored.
    fn log_dir(&self) -> Result<AbsPathBuf, impl Error> {
        neovim::oxi::api::call_function::<_, String>("stdpath", ("data",))
            .map_err(Either::Left)
            .and_then(|path| path.parse::<AbsPathBuf>().map_err(Either::Right))
            .map(|neovim_data_dir| neovim_data_dir.join(node!("nomad")))
    }
}

impl Plugin<Neovim> for Nomad {
    const COMMAND_NAME: Name = "Mad";
}

impl Module<Neovim> for Nomad {
    const NAME: Name = "nomad";

    type Config = Empty;

    fn api(&self, ctx: &mut ApiCtx<Neovim>) {
        let auth = auth::Auth::default();
        let collab = collab::Collab::from(&auth);

        ctx.with_command(auth.login())
            .with_command(auth.logout())
            .with_command(collab.start())
            .with_command(version::EmitVersion::new())
            .with_constant(version::VERSION)
            .with_module(auth)
            .with_module(collab);
    }

    fn on_init(&self, ctx: &mut Context<Neovim, Borrowed>) {
        ctx.set_notifier(neovim::notify::detect());

        let subscriber = tracing_subscriber::Registry::default()
            .with(ctx.tracing_layer())
            .with(self.log_dir().map(|dir| FileAppender::new(dir, ctx)).ok());

        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            panic!("failed to set global tracing subscriber: {err}");
        }
    }

    fn on_new_config(
        &self,
        _: Self::Config,
        _: &mut Context<Neovim, Borrowed>,
    ) {
    }
}
