use core::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use serde::de::{self, Deserialize};

use super::EnableConfig;
use crate::ctx::{Ctx, Set};
use crate::module::{Module, ModuleId, ModuleName};
use crate::nvim::{serde::Deserializer, Function, Object};
use crate::warning::{Chunk, Warning, WarningMsg};

thread_local! {
    /// TODO: docs
    static DESERIALIZERS: ConfigDeserializers
        = const { ConfigDeserializers::new() };
}

/// TODO: docs
pub(crate) fn config() -> Function<Object, ()> {
    Function::from_fn(|object| {
        let deserializer = Deserializer::new(object);
        UpdateConfigs::deserialize(deserializer).unwrap();
        Ok::<_, core::convert::Infallible>(())
    })
}

/// TODO: docs
#[inline]
pub(crate) fn with_module<M>(set_config: Set<EnableConfig<M>>, ctx: Ctx)
where
    M: Module,
{
    DESERIALIZERS.with(|deserializers| {
        let deserializer = ConfigDeserializer::new(set_config, ctx);
        deserializers.insert(M::NAME.id(), deserializer)
    });
}

/// TODO: docs
#[inline]
fn valid_modules() -> &'static [ModuleName] {
    &[]
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
    deserializer: Box<dyn Fn(Object) + 'static>,
}

impl ConfigDeserializer {
    /// TODO: docs
    #[inline]
    fn deserialize(&self, config: Object) {
        (self.deserializer)(config);
    }

    /// TODO: docs
    #[inline]
    fn new<M: Module>(set_config: Set<EnableConfig<M>>, ctx: Ctx) -> Self {
        let deserializer = move |config: Object| {
            let deserializer = Deserializer::new(config);
            let config = match EnableConfig::<M>::deserialize(deserializer) {
                Ok(config) => config,
                Err(_err) => return,
            };
            ctx.with_set(|set_ctx| set_config.set(config, set_ctx));
        };

        Self { deserializer: Box::new(deserializer) }
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

        while let Some(module_name) = map.next_key::<String>()? {
            let module_config = map.next_value::<Object>()?;
            buffer.push((module_name, module_config));
        }

        for (module_name, module_config) in buffer {
            if let Err(err) = update_config(module_name, module_config) {
                err.to_warning().print();
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
                .map(|des| des.deserialize(module_config))
        })
    })
}

/// TODO: docs
enum Error {
    InvalidModule(String),
}

impl Error {
    #[inline]
    fn to_warning(&self) -> Warning {
        let msg = match self {
            Self::InvalidModule(module) => {
                match InvalidModuleMsgKind::from_str(module) {
                    InvalidModuleMsgKind::ListAllModules => {
                        list_all_modules_msg(module)
                    },

                    InvalidModuleMsgKind::SuggestClosest(closest) => {
                        suggest_closest_msg(module, closest)
                    },
                }
            },
        };

        Warning::new().msg(msg)
    }
}

enum InvalidModuleMsgKind {
    ListAllModules,
    SuggestClosest(ModuleName),
}

impl InvalidModuleMsgKind {
    #[inline]
    fn from_str(module: &str) -> Self {
        let mut min_distance = usize::MAX;

        let mut idx_closest = 0;

        for (idx, valid) in valid_modules().iter().enumerate() {
            let distance = strsim::damerau_levenshtein(module, valid.as_str());

            if distance < min_distance {
                min_distance = distance;
                idx_closest = idx;
            }
        }

        let should_suggest_closest = match module.len() {
            3 => min_distance <= 1,
            4..=6 => min_distance <= 2,
            7..=10 => min_distance <= 3,
            _ => false,
        };

        if should_suggest_closest {
            let closest = valid_modules()[idx_closest];
            Self::SuggestClosest(closest)
        } else {
            Self::ListAllModules
        }
    }
}

/// TODO: docs
#[inline]
fn list_all_modules_msg(module: &str) -> WarningMsg {
    let mut msg = WarningMsg::new();

    msg.add("invalid module ").add(module.highlight());

    match valid_modules() {
        [] => return msg,

        [one] => {
            msg.add(", the only valid module is ").add(one.highlight());
            return msg;
        },

        modules => {
            msg.add(", the valid modules are ");

            for (idx, module) in modules.iter().enumerate() {
                msg.add(module.highlight());

                let is_last = idx + 1 == modules.len();

                if is_last {
                    break;
                }

                let is_second_to_last = idx + 2 == modules.len();

                if is_second_to_last {
                    msg.add(" and ");
                } else {
                    msg.add(", ");
                }
            }
        },
    }

    msg
}

/// TODO: docs
#[inline]
fn suggest_closest_msg(module: &str, closest: ModuleName) -> WarningMsg {
    let mut msg = WarningMsg::new();

    msg.add("invalid module ")
        .add(module.highlight())
        .add(", did you mean ")
        .add(closest.highlight())
        .add("?");

    msg
}
