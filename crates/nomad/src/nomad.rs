use core::future::Future;
use core::pin::Pin;

use collab_fs::AbsUtf8PathBuf;
use nvim_oxi::{lua, Dictionary as NvimDictionary, Function as NvimFunction};

use crate::config::Setup;
use crate::ctx::NeovimCtx;
use crate::diagnostics::{DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::nomad_command::NomadCommand;
use crate::Module;

/// TODO: docs.
pub struct Nomad {
    api: NvimDictionary,
    command: NomadCommand,
    neovim_ctx: NeovimCtx<'static>,
    run: Vec<Pin<Box<dyn Future<Output = ()>>>>,
    setup: Setup,
}

impl Nomad {
    /// TODO: docs.
    pub(crate) const AUGROUP_NAME: &'static str = "nomad";

    /// TODO: docs.
    pub(crate) const COMMAND_NAME: &'static str = "Mad";

    /// TODO: docs.
    pub(crate) const DIAGNOSTICS_SEGMENT_NAME: &'static str = "nomad";

    /// TODO: docs.
    pub fn new() -> Self {
        Self {
            api: NvimDictionary::default(),
            command: NomadCommand::default(),
            neovim_ctx: NeovimCtx::default(),
            run: Vec::default(),
            setup: Setup::default(),
        }
    }

    /// TODO: docs.
    #[track_caller]
    pub fn with_module<M: Module>(mut self) -> Self {
        let config_rx = self.setup.add_module::<M>();
        let module = M::from(config_rx);
        let module_api = module.init(self.neovim_ctx.reborrow());
        self.api.insert(M::NAME.as_str(), module_api.dictionary);
        self.command.add_module(module_api.commands);
        self.run.push({
            let neovim_ctx = self.neovim_ctx.clone();
            Box::pin(async move {
                if let Err(err) = module.run(neovim_ctx).await.into_result() {
                    let mut source = DiagnosticSource::new();
                    source.push_segment(M::NAME.as_str());
                    err.into().emit(Level::Error, source);
                }
            })
        });
        self
    }

    pub(crate) fn log_dir(&self) -> AbsUtf8PathBuf {
        #[cfg(target_family = "unix")]
        {
            let mut home = match home::home_dir() {
                Some(home) if !home.as_os_str().is_empty() => {
                    AbsUtf8PathBuf::from_path_buf(home)
                        .expect("home is absolute")
                },
                _ => panic!("failed to get the home directory"),
            };
            home.push(".local");
            home.push("share");
            home.push("nvim");
            home.push("nomad");
            home.push("logs");
            home
        }
        #[cfg(not(target_family = "unix"))]
        {
            unimplemented!()
        }
    }
}

impl lua::Pushable for Nomad {
    unsafe fn push(
        mut self,
        state: *mut lua::ffi::State,
    ) -> Result<i32, lua::Error> {
        crate::log::init(&self.log_dir());

        // Start each module's event loop.
        for fut in self.run.drain(..) {
            crate::executor::spawn(fut).detach();
        }

        self.command.create();

        let setup = NvimFunction::from_fn(self.setup.into_fn());
        self.api.insert(Setup::NAME, setup);
        self.api.push(state)
    }
}
