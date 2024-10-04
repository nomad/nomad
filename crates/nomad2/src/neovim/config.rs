//! Contains things related to the configuration of modules.

use core::cmp::Ordering;
use core::marker::PhantomData;
use std::collections::HashMap;

use nvim_oxi::{
    Object as NvimObject,
    ObjectKind as NvimObjectKind,
    String as NvimString,
};
use serde::de::DeserializeOwned;

use super::diagnostic::{
    DiagnosticMessage,
    DiagnosticSource,
    HighlightGroup,
    Level,
};
use super::serde::deserialize;
use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared};

pub(super) type OnConfigChange =
    Box<dyn Fn(NvimObject) -> Result<(), DeserializeConfigError>>;

/// TODO: docs.
pub struct ConfigEvent<T> {
    pub(super) module_name: &'static str,
    pub(super) buf: Shared<Option<OnConfigChange>>,
    pub(super) ty: PhantomData<T>,
}

impl<T: Module<Neovim>> ConfigEvent<T> {
    pub(super) fn new(buf: Shared<Option<OnConfigChange>>) -> Self {
        Self { module_name: T::NAME.as_str(), buf, ty: PhantomData }
    }
}

impl<T> PartialEq for ConfigEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T> Eq for ConfigEvent<T> {}

impl<T> PartialOrd for ConfigEvent<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for ConfigEvent<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.module_name.cmp(other.module_name)
    }
}

impl<T: Module<Neovim>> Event<Neovim> for ConfigEvent<T> {
    type Payload = T::Config;
    type SubscribeCtx = ();

    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        _: &Context<Neovim>,
    ) {
        let on_config_change = Box::new(move |obj| {
            let config = deserialize::<T::Config>(obj).map_err(|err| {
                let mut source = DiagnosticSource::new();
                source
                    .push_segment(Setup::NAME)
                    .push_segment(T::NAME.as_str());
                DeserializeConfigError::new(source, err.into_msg())
            })?;
            emitter.send(config);
            Ok(())
        });

        self.buf.with_mut(|buf| {
            *buf = Some(on_config_change);
        });
    }
}

pub(super) struct Setup {
    on_config_change: HashMap<&'static str, OnConfigChange>,
    module_names: Vec<&'static str>,
}

impl Setup {
    pub(super) const NAME: &'static str = "setup";

    pub(super) fn into_fn(self) -> impl Fn(NvimObject) {
        move |obj| {
            if let Err(err) = self.on_config_change(obj) {
                err.emit()
            }
        }
    }

    pub(super) fn new(
        on_config_change: HashMap<&'static str, OnConfigChange>,
    ) -> Self {
        let mut module_names =
            on_config_change.keys().copied().collect::<Vec<_>>();

        // Sort the module names alphabetically to have a nicer message when
        // including the list of valid modules in a warning.
        module_names.sort_unstable();

        Self { on_config_change, module_names }
    }

    fn on_config_change(
        &self,
        obj: NvimObject,
    ) -> Result<(), DeserializeConfigError> {
        let dict = match obj.kind() {
            NvimObjectKind::Dictionary => {
                // SAFETY: the object's kind is a dictionary.
                unsafe { obj.into_dict_unchecked() }
            },
            other => {
                return Err(DeserializeConfigError::config_not_dict(other))
            },
        };

        let handle = |module_name: NvimString, module_config: NvimObject| {
            let module_name = module_name.to_str().map_err(|_| {
                DeserializeConfigError::non_unicode_module_name(&module_name)
            })?;

            let on_config_change =
                self.on_config_change.get(&module_name).ok_or_else(|| {
                    DeserializeConfigError::unknown_module(
                        module_name,
                        &self.module_names,
                    )
                })?;

            on_config_change(module_config)
        };

        for (module_name, module_config) in dict {
            if let Err(err) = handle(module_name, module_config) {
                err.emit()
            }
        }

        Ok(())
    }
}

/// Error returned when a subset of the Lua object given to the `setup()`
/// function cannot be deserialized into the expected type.
pub(super) struct DeserializeConfigError {
    source: DiagnosticSource,
    msg: DiagnosticMessage,
}

impl DeserializeConfigError {
    fn emit(self) {
        self.msg.emit(Level::Error, self.source);
    }

    fn new(source: DiagnosticSource, msg: DiagnosticMessage) -> Self {
        Self { source, msg }
    }

    fn non_unicode_module_name(module_name: &NvimString) -> Self {
        let mut source = DiagnosticSource::new();
        source.push_segment(Setup::NAME);

        let mut msg = DiagnosticMessage::new();
        let module_name = module_name.to_string_lossy();
        msg.push_str("module name '")
            .push_str_highlighted(module_name, HighlightGroup::special())
            .push_str("' is not a valid Unicode string");

        Self { source, msg }
    }

    fn config_not_dict(kind: NvimObjectKind) -> Self {
        let mut source = DiagnosticSource::new();
        source.push_segment(Setup::NAME);

        let mut msg = DiagnosticMessage::new();
        msg.push_str("expected a dictionary, got a ")
            .push_str_highlighted(kind.as_static(), HighlightGroup::special())
            .push_str(" instead");

        Self { source, msg }
    }

    fn unknown_module(module_name: &str, valid_module_names: &[&str]) -> Self {
        debug_assert!(!valid_module_names.is_empty());

        let mut source = DiagnosticSource::new();
        source.push_segment(Setup::NAME);

        let mut msg = DiagnosticMessage::new();
        msg.push_str("unknown module '")
            .push_str_highlighted(module_name, HighlightGroup::special())
            .push_str("', the valid modules are ")
            .push_comma_separated(
                valid_module_names,
                HighlightGroup::special(),
            );

        Self { source, msg }
    }
}
