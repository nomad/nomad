use core::cell::{OnceCell, RefCell};
use core::fmt;
use std::collections::HashMap;

use nvim::{serde::Deserializer, Function, Object};
use serde::de::{self, Deserialize};

use crate::prelude::*;
use crate::serde::{deserialize, DeserializeError};

/// TODO: docs
pub(crate) const CONFIG_NAME: ActionName = ActionName::from_str("config");

thread_local! {
    /// TODO: docs
    static DESERIALIZERS: ConfigDeserializers
        = const { ConfigDeserializers::new() };

    /// TODO: docs
    static MODULE_NAMES: OnceCell<&'static [ModuleName]>
        = const { OnceCell::new() };
}

/// TODO: docs
pub(crate) fn config() -> Function<Object, ()> {
    Function::from_fn(|object| {
        let deserializer = Deserializer::new(object);

        if let Err(err) = UpdateConfigs::deserialize(deserializer) {
            warning(invalid_config_msg(err)).print();
        }

        Ok::<_, core::convert::Infallible>(())
    })
}

/// TODO: docs
#[inline]
fn valid_modules() -> &'static [ModuleName] {
    let init_module_names = || {
        DESERIALIZERS.with(|d| {
            d.with_map(|map| {
                let mut vec = map
                    .values()
                    .map(ConfigDeserializer::module_name)
                    .collect::<Vec<_>>();

                // Sort the module names alphabetically. This produces a nicer
                // message if we need to print the list of valid modules in a
                // warning.
                vec.sort_unstable();

                // This isn't a memory leak because we're only leaking the
                // vector once when this function is called for the first time.
                &*(vec.leak())
            })
        })
    };

    MODULE_NAMES.with(|names| *names.get_or_init(init_module_names))
}

/// TODO: docs
#[inline]
fn warning(msg: WarningMsg) -> Warning {
    Warning::new().action(CONFIG_NAME).msg(msg)
}

/// TODO: docs
#[inline]
pub(crate) fn with_module<M>(set_config: Set<M::Config>)
where
    M: Module,
{
    DESERIALIZERS.with(|deserializers| {
        let deserializer = ConfigDeserializer::new::<M>(set_config);
        deserializers.insert(M::NAME.id(), deserializer)
    });
}

/// TODO: docs
struct ConfigDeserializers {
    deserializers: OnceCell<RefCell<HashMap<ModuleId, ConfigDeserializer>>>,
}

impl ConfigDeserializers {
    /// TODO: docs
    #[inline]
    fn insert(&self, id: ModuleId, deserializer: ConfigDeserializer) {
        self.with_map(|map| map.insert(id, deserializer));
    }

    /// TODO: docs
    const fn new() -> Self {
        Self { deserializers: OnceCell::new() }
    }

    /// TODO: docs
    #[inline]
    fn with_map<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut HashMap<ModuleId, ConfigDeserializer>) -> R,
    {
        let inner = self.deserializers.get_or_init(RefCell::default);
        let map = &mut *inner.borrow_mut();
        f(map)
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
            let config = deserialize(config).map_err(|mut err| {
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
struct UpdateConfigs;

impl<'de> Deserialize<'de> for UpdateConfigs {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_map(UpdateConfigsVisitor)
    }
}

struct UpdateConfigsVisitor;

impl<'de> de::Visitor<'de> for UpdateConfigsVisitor {
    type Value = UpdateConfigs;

    #[inline]
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
                    warning(invalid_key_msg::<A>(err)).print();
                    return Ok(UpdateConfigs);
                },
            };

            let module_config = match map.next_value::<Object>() {
                Ok(obj) => obj,
                Err(err) => {
                    warning(invalid_object_msg::<A>(err)).print();
                    return Ok(UpdateConfigs);
                },
            };

            buffer.push((module_name, module_config));
        }

        for (module_name, module_config) in buffer {
            if let Err(err) = update_config(module_name, module_config) {
                warning(err.into()).print();
            }
        }

        Ok(UpdateConfigs)
    }
}

/// TODO: docs
#[inline]
fn update_config(
    module_name: String,
    module_config: Object,
) -> Result<(), Error> {
    let module_id = ModuleId::from_module_name(&module_name);

    DESERIALIZERS.with(move |deserializers| {
        deserializers.with_map(|map| {
            map.get(&module_id)
                .ok_or(Error::InvalidModule(module_name))
                .and_then(|des| {
                    des.deserialize(module_config)
                        .map_err(Error::DeserializeModule)
                })
        })
    })
}

/// TODO: docs
enum Error {
    /// TODO: docs
    InvalidModule(String),

    /// TODO: docs
    DeserializeModule(DeserializeError),
}

impl From<Error> for WarningMsg {
    #[inline]
    fn from(err: Error) -> WarningMsg {
        let mut msg = WarningMsg::new();

        match err {
            Error::InvalidModule(module) => {
                msg.add("couldn't deserialize config: ");
                msg.add_invalid(module, valid_modules().iter(), "module");
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
