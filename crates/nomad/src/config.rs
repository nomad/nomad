use core::fmt;
use std::collections::HashMap;

use nvim::serde::Deserializer;
use nvim::{Function, Object};
use serde::de::{self, DeserializeSeed};

// use crate::serde::{deserialize, DeserializeError};
use crate::{
    ActionName,
    DeserializeError,
    Module,
    ModuleId,
    ModuleName,
    Set,
    Warning,
    WarningMsg,
};

#[derive(Default)]
pub(crate) struct Config {
    /// TODO: docs
    deserializers: HashMap<ModuleId, ConfigDeserializer>,

    /// TODO: docs
    module_names: &'static [ModuleName],
}

impl Config {
    pub(crate) const NAME: ActionName = ActionName::from_str("config");

    /// TODO: docs
    #[inline]
    pub(crate) fn add_module<M: Module>(
        &mut self,
        set_config: Set<M::Config>,
    ) {
        let deserializer = ConfigDeserializer::new::<M>(set_config);
        self.deserializers.insert(M::NAME.id(), deserializer);
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn into_function(mut self) -> Function<Object, ()> {
        let mut names = self
            .deserializers
            .values()
            .map(ConfigDeserializer::module_name)
            .collect::<Vec<_>>();

        // Sort the module names alphabetically. This produces a nicer
        // message if we need to print the list of valid modules in a
        // warning.
        names.sort_unstable();

        // This isn't a memory leak because we're only leaking the
        // vector once when this function is called for the first time.
        self.module_names = &*(names.leak());

        Function::from_fn(move |object| {
            let deserializer = Deserializer::new(object);

            if let Err(err) = self.deserialize(deserializer) {
                self.warning(invalid_config_msg(err)).print();
            }

            Ok::<_, core::convert::Infallible>(())
        })
    }

    /// TODO: docs
    #[inline]
    fn update_config(
        &self,
        module_name: String,
        module_config: Object,
    ) -> Result<(), Error> {
        let module_id = ModuleId::from_module_name(&module_name);

        self.deserializers
            .get(&module_id)
            .ok_or(Error::InvalidModule(module_name, self.module_names))
            .and_then(|des| {
                des.deserialize(module_config)
                    .map_err(Error::DeserializeModule)
            })
    }

    /// TODO: docs
    #[inline]
    fn warning(&self, msg: WarningMsg) -> Warning {
        Warning::new().action(Self::NAME).msg(msg)
    }
}

impl<'de> DeserializeSeed<'de> for &Config {
    type Value = ();

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { config: self })
    }
}

struct ConfigVisitor<'a> {
    config: &'a Config,
}

impl<'de> de::Visitor<'de> for ConfigVisitor<'_> {
    type Value = ();

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a dictionary")
    }

    #[inline]
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // We store the module names and their configs because we want to make
        // sure that the map deserializes correctly before deserializing the
        // individual configs for each module.
        //
        // Not doing this could cause only some of the configs to be updated.
        // For example:
        //
        // ```lua
        // nomad.config({
        //   foo = { .. },
        //   "hello",
        //   bar = { .. },
        // })
        // ```
        //
        // In this case, the `foo` module would be updated, but the `bar`
        // module wouldn't because the `config` function would return an error
        // when it gets to `"hello"`.
        let mut buffer = Vec::new();

        loop {
            let module_name = match map.next_key::<String>() {
                Ok(Some(name)) => name,
                Ok(None) => break,
                Err(err) => {
                    self.config.warning(invalid_key_msg::<A>(err)).print();
                    return Ok(());
                },
            };

            let module_config = match map.next_value::<Object>() {
                Ok(obj) => obj,
                Err(err) => {
                    self.config.warning(invalid_object_msg::<A>(err)).print();
                    return Ok(());
                },
            };

            buffer.push((module_name, module_config));
        }

        for (module_name, module_config) in buffer {
            if let Err(err) =
                self.config.update_config(module_name, module_config)
            {
                self.config.warning(err.into()).print();
            }
        }

        Ok(())
    }
}

/// TODO: docs
struct ConfigDeserializer {
    deserializer: Box<dyn Fn(Object) -> Result<(), DeserializeError>>,
    module_name: ModuleName,
}

impl ConfigDeserializer {
    /// TODO: docs
    #[inline]
    fn deserialize(&self, config: Object) -> Result<(), DeserializeError> {
        (self.deserializer)(config)
    }

    /// TODO: docs
    #[inline]
    fn module_name(&self) -> ModuleName {
        self.module_name
    }

    /// TODO: docs
    #[inline]
    fn new<M: Module>(set_config: Set<M::Config>) -> Self {
        let deserializer = move |config: Object| {
            let config =
                crate::deserialize(config, "config").map_err(|mut err| {
                    err.set_module_name(M::NAME);
                    err
                })?;
            set_config.set(config);
            Ok(())
        };

        Self { deserializer: Box::new(deserializer), module_name: M::NAME }
    }
}

/// TODO: docs
enum Error {
    /// TODO: docs
    InvalidModule(String, &'static [ModuleName]),

    /// TODO: docs
    DeserializeModule(DeserializeError),
}

impl From<Error> for WarningMsg {
    #[inline]
    fn from(err: Error) -> WarningMsg {
        let mut msg = WarningMsg::new();

        match err {
            Error::InvalidModule(module, valid_modules) => {
                msg.add("couldn't deserialize config: ");
                msg.add_invalid(module, valid_modules.iter(), "module");
            },

            Error::DeserializeModule(err) => {
                msg = err.into();
            },
        };

        msg
    }
}

/// TODO: docs
#[inline]
fn invalid_config_msg<E: fmt::Display>(err: E) -> WarningMsg {
    let mut msg = WarningMsg::new();
    msg.add("couldn't deserialize config: ").add(err.to_string());
    msg
}

/// TODO: docs
#[inline]
fn invalid_key_msg<'de, A>(err: A::Error) -> WarningMsg
where
    A: de::MapAccess<'de>,
{
    let mut msg = WarningMsg::new();
    msg.add("couldn't deserialize config key: ").add(err.to_string());
    msg
}

/// TODO: docs
#[inline]
fn invalid_object_msg<'de, A>(err: A::Error) -> WarningMsg
where
    A: de::MapAccess<'de>,
{
    let mut msg = WarningMsg::new();
    msg.add("couldn't deserialize object: ").add(err.to_string());
    msg
}
