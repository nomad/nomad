//! TODO: docs.

use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use fxhash::FxHashMap;
use nvim_oxi::{
    Object as NvimObject,
    ObjectKind as NvimObjectKind,
    String as NvimString,
};
use smol_str::SmolStr;

use crate::diagnostics::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};
use crate::Module;

/// The output of calling [`as_str`](crate::ModuleName::as_str) on a
/// [`ModuleName`](crate::ModuleName).
type ModuleNameStr = &'static str;

/// TODO: docs.
pub struct ConfigReceiver<M> {
    stream: ConfigStream<M>,
}

/// TODO: docs.
#[derive(Default)]
pub(crate) struct Setup {
    /// Map from [`ModuleName`] to the [`ConfigReceiver`] for that module.]
    config_senders: FxHashMap<ModuleNameStr, ConfigSender>,
}

/// TODO: docs.
struct ConfigStream<T> {
    inner: futures_util::stream::Pending<NvimObject>,
    ty: PhantomData<T>,
}

/// TODO: docs.
struct ConfigSender {}

impl<M: Module> ConfigReceiver<M> {
    /// TODO: docs.
    pub async fn recv(&mut self) -> M::Config {
        use futures_util::StreamExt;
        self.stream
            .next()
            .await
            .expect("sender never dropped, stream never ends")
    }
}

impl Setup {
    pub(crate) const NAME: &'static str = "setup";

    /// Adds a module to the setup function.
    ///
    /// # Panics
    ///
    /// Panics if the module's name is `"setup"` or equal to the name of a
    /// previously added module.
    #[track_caller]
    pub(crate) fn add_module<M: Module>(&mut self) -> ConfigReceiver<M> {
        todo!();
    }

    pub(crate) fn into_fn(self) -> impl Fn(NvimObject) + 'static {
        move |obj| {
            if let Err(errors) = self.call(obj) {
                for error in errors {
                    error.emit()
                }
            }
        }
    }

    fn call(&self, config: NvimObject) -> Result<(), Vec<SetupError>> {
        let config = match config.kind() {
            NvimObjectKind::Dictionary => {
                // SAFETY: the object's kind is a dictionary.
                unsafe { config.into_dict_unchecked() }
            },
            other => return Err(vec![SetupError::ConfigNotDict(other)]),
        };

        let mut errors = Vec::new();

        for (module_name, module_config) in config {
            let module_name = match module_name.to_str() {
                Ok(module_name) => module_name,
                Err(_) => {
                    errors.push(SetupError::NonUnicodeKey(module_name));
                    continue;
                },
            };

            let Some(config_sender) = self.config_senders.get(module_name)
            else {
                errors.push(SetupError::UnknownModule {
                    name: module_name.into(),
                    valid: self.config_senders.keys().copied().collect(),
                });
                continue;
            };

            config_sender.send(module_config);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl ConfigSender {
    fn send(&self, _config: NvimObject) {
        todo!();
    }
}

/// The type of error that can occur when [`call`](Setup::call)ing the
/// [`Setup`] function.
enum SetupError {
    /// The configuration object is not a dictionary.
    ConfigNotDict(NvimObjectKind),

    /// The configuration dictionary contains a key that is not a valid UTF-8
    /// string.
    NonUnicodeKey(NvimString),

    /// The configuration dictionary contains a module name that doesn't match
    /// any of the modules that were added to [`Setup`].
    UnknownModule { name: SmolStr, valid: Vec<ModuleNameStr> },
}

impl SetupError {
    fn emit(&self) {
        self.message().emit(Level::Error, self.source());
    }

    fn message(&self) -> DiagnosticMessage {
        let mut message = DiagnosticMessage::new();
        match self {
            SetupError::ConfigNotDict(kind) => {
                message
                    .push_str("expected a dictionary, got a ")
                    .push_str_highlighted(
                        kind.as_static(),
                        HighlightGroup::special(),
                    )
                    .push_str(" instead");
            },
            SetupError::NonUnicodeKey(key) => {
                message
                    .push_str("module name '")
                    .push_str_highlighted(
                        key.to_string_lossy(),
                        HighlightGroup::special(),
                    )
                    .push_str("' is not a valid Unicode string");
            },
            SetupError::UnknownModule { name, valid } => {
                message
                    .push_str("unknown module '")
                    .push_str_highlighted(name, HighlightGroup::special())
                    .push_str("', the valid modules are ")
                    .push_comma_separated(valid, HighlightGroup::special());
            },
        }
        message
    }

    fn source(&self) -> DiagnosticSource {
        let mut source = DiagnosticSource::new();
        source.push_segment(Setup::NAME);
        source
    }
}

impl<M: Module> Unpin for ConfigStream<M> {}

impl<M> Stream for ConfigStream<M>
where
    M: Module,
{
    type Item = M::Config;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let obj = match self.get_mut().inner.poll_next_unpin(ctx) {
            Poll::Ready(Some(obj)) => obj,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => unreachable!(),
        };

        match crate::serde::deserialize::<M::Config>(obj) {
            Ok(config) => Poll::Ready(Some(config)),
            Err(err) => {
                let mut source = DiagnosticSource::new();
                source
                    .push_segment(Setup::NAME)
                    .push_segment(M::NAME.as_str());
                err.into_msg().emit(Level::Warning, source);
                Poll::Pending
            },
        }
    }
}
