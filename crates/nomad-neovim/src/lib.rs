//! TODO: docs.

use core::error::Error;

use abs_path::{AbsPathBuf, NodeName, node};
use editor::Context;
use editor::context::Borrowed;
use editor::module::{ApiCtx, Empty, Module, PanicInfo, Plugin};
use either::Either;
use neovim::Neovim;
use neovim::notify::ContextExt;
use tracing::Subscriber;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

#[neovim::plugin]
fn nomad() -> Nomad {
    Nomad
}

struct Nomad;

impl Nomad {
    /// The prefix for the log file names.
    const LOG_FILENAME_PREFIX: &NodeName = node!("nomad.log");

    /// Returns the directory path under which files that need to be persisted
    /// over Neovim restarts should be stored.
    fn data_dir(&self) -> Result<AbsPathBuf, impl Error> {
        neovim::oxi::api::call_function::<_, String>("stdpath", ("data",))
            .map_err(Either::Left)
            .and_then(|path| path.parse::<AbsPathBuf>().map_err(Either::Right))
            .map(|neovim_data_dir| neovim_data_dir.join(node!("nomad")))
    }

    fn file_appender<S>(
        &self,
        ctx: &mut Context<Neovim, Borrowed>,
    ) -> impl Layer<S>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        match self.log_dir() {
            Ok(dir) => Some(tracing_layers::FileAppender::new(
                dir,
                Self::LOG_FILENAME_PREFIX.to_owned(),
                ctx,
            )),
            Err(err) => {
                ctx.notify_error(format_args!(
                    "couldn't create tracing file appender: {err}"
                ));
                None
            },
        }
    }

    /// Returns the directory path under which the log files should be stored.
    fn log_dir(&self) -> Result<AbsPathBuf, impl Error> {
        self.data_dir().map(|dir| dir.join(node!("logs")))
    }
}

impl Plugin<Neovim> for Nomad {
    const COMMAND_NAME: &str = "Mad";

    fn handle_panic(
        &self,
        panic_info: PanicInfo,
        ctx: &mut Context<Neovim, Borrowed<'_>>,
    ) {
        tracing::error!(
            title = %ctx.namespace().dot_separated(),
            "panicked{at_location}{with_payload}",
            at_location = panic_info
                .location
                .as_ref()
                .map(|loc| format!(" at {loc}"))
                .unwrap_or_default(),
            with_payload = panic_info
                .payload_as_str()
                .map(|payload| format!(": {payload}"))
                .unwrap_or_default(),
        );
    }
}

impl Module<Neovim> for Nomad {
    const NAME: &str = "nomad";

    type Config = Empty;

    fn api(&self, ctx: &mut ApiCtx<Neovim>) {
        let auth = auth::Auth::default();
        let collab = collab::Collab::from(&auth);

        ctx.with_command(auth::login::Login::from(&auth))
            .with_command(auth::logout::Logout::from(&auth))
            .with_command(collab::start::Start::from(&collab))
            .with_command(collab::join::Join::from(&collab))
            .with_command(version::EmitVersion::new())
            .with_constant(version::VERSION)
            .with_module(auth)
            .with_module(collab);
    }

    fn on_init(&self, ctx: &mut Context<Neovim, Borrowed>) {
        ctx.set_notifier(neovim::notify::detect());

        let subscriber = tracing_subscriber::Registry::default()
            .with(ctx.tracing_layer())
            .with(self.file_appender(ctx).with_filter(LevelFilter::INFO));

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
