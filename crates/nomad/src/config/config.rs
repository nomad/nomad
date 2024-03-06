use core::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use serde::de::{self, Deserialize};

use crate::ctx::{Ctx, Set};
use crate::module::{Module, ModuleId, ModuleName};
use crate::nvim::{serde::Deserializer, Function, Object};
use crate::warning::{ChunkExt, Warning, WarningMsg};

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
            Warning::new().msg(invalid_config_msg(err)).print();
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
pub(crate) fn with_module<M>(set_config: Set<M::Config>, ctx: Ctx)
where
    M: Module,
{
    DESERIALIZERS.with(|deserializers| {
        let deserializer = ConfigDeserializer::new::<M>(set_config, ctx);
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

type DeserializationError =
    serde_path_to_error::Error<nvim_oxi::serde::DeserializeError>;

/// TODO: docs
struct ConfigDeserializer {
    deserializer: Box<dyn Fn(Object) -> Result<(), DeserializationError>>,
    module_name: ModuleName,
}

impl ConfigDeserializer {
    /// TODO: docs
    #[inline]
    fn deserialize(&self, config: Object) -> Result<(), DeserializationError> {
        (self.deserializer)(config)
    }

    /// TODO: docs
    #[inline]
    fn module_name(&self) -> ModuleName {
        self.module_name
    }

    /// TODO: docs
    #[inline]
    fn new<M: Module>(set_config: Set<M::Config>, ctx: Ctx) -> Self {
        let deserializer = move |config: Object| {
            let deserializer = Deserializer::new(config);
            let config = serde_path_to_error::deserialize(deserializer)?;
            ctx.with_set(|set_ctx| set_config.set(config, set_ctx));
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
                    Warning::new().msg(invalid_key_msg::<A>(err)).print();
                    return Ok(UpdateConfigs);
                },
            };

            let module_config = match map.next_value::<Object>() {
                Ok(obj) => obj,
                Err(err) => {
                    Warning::new().msg(invalid_object_msg::<A>(err)).print();
                    return Ok(UpdateConfigs);
                },
            };

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
    DeserializeModule(DeserializationError),
}

impl Error {
    #[inline]
    fn to_warning(&self) -> Warning {
        let mut msg = WarningMsg::new();

        match self {
            Self::InvalidModule(module) => {
                msg.add("couldn't deserialize config: ");

                msg::invalid_str(module, valid_modules(), "module", &mut msg);
            },

            Self::DeserializeModule(err) => {
                msg.add("couldn't deserialize ")
                    .add(err.path().to_string().highlight())
                    .add(": ");

                use nvim_oxi::serde::DeserializeError::*;

                match err.inner() {
                    Custom { msg: err_msg } => {
                        msg.add(err_msg.as_str());
                    },

                    DuplicateField { field } => {
                        msg.add("duplicate field ").add(field.highlight());
                    },

                    MissingField { field } => {
                        msg.add("missing field ").add(field.highlight());
                    },

                    UnknownField { variant: field, expected } => {
                        msg::invalid_str(field, expected, "field", &mut msg);
                    },

                    UnknownVariant { field: variant, expected } => {
                        msg::invalid_str(
                            variant, expected, "variant", &mut msg,
                        );
                    },
                }
            },
        };

        Warning::new().msg(msg)
    }
}

/// TODO: docs
#[inline]
fn invalid_config_msg<E: core::fmt::Display>(err: E) -> WarningMsg {
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

mod msg {
    use super::*;

    /// TODO: docs
    pub(super) fn invalid_str(
        invalid: &str,
        valid: &[impl AsRef<str> + Copy],
        what: &str,
        msg: &mut WarningMsg,
    ) {
        match InvalidStrMsgKind::new(invalid, valid) {
            InvalidStrMsgKind::ListAll => list_all(invalid, what, valid, msg),

            InvalidStrMsgKind::SuggestClosest { idx } => {
                suggest_closest(invalid, what, valid[idx], msg)
            },
        }
    }

    /// TODO: docs
    enum InvalidStrMsgKind {
        /// TODO: docs
        ListAll,

        /// TODO: docs
        SuggestClosest { idx: usize },
    }

    impl InvalidStrMsgKind {
        #[inline]
        fn new<T, V>(invalid: &str, valid: V) -> Self
        where
            V: IntoIterator<Item = T>,
            V::IntoIter: ExactSizeIterator,
            T: AsRef<str>,
        {
            let valid = valid.into_iter();

            if valid.len() == 0 {
                return Self::ListAll;
            }

            let mut min_distance = usize::MAX;
            let mut idx_closest = 0;

            for (idx, valid) in valid.enumerate() {
                let distance =
                    strsim::damerau_levenshtein(invalid, valid.as_ref());

                if distance < min_distance {
                    min_distance = distance;
                    idx_closest = idx;
                }
            }

            let should_suggest_closest = match invalid.len() {
                // These ranges and cutoffs are arbitrary.
                3 => min_distance <= 1,
                4..=6 => min_distance <= 2,
                7..=10 => min_distance <= 3,
                _ => false,
            };

            if should_suggest_closest {
                Self::SuggestClosest { idx: idx_closest }
            } else {
                Self::ListAll
            }
        }
    }

    /// TODO: docs
    #[inline]
    fn list_all(
        invalid: &str,
        invalid_what: &str,
        valid: &[impl AsRef<str>],
        msg: &mut WarningMsg,
    ) {
        msg.add("invalid ")
            .add(invalid_what)
            .add(" ")
            .add(invalid.highlight());

        match valid {
            [] => {},

            [one] => {
                msg.add(", the only valid ")
                    .add(invalid_what)
                    .add(" is ")
                    .add(one.as_ref().highlight());
            },

            modules => {
                msg.add(", the valid ").add(invalid_what).add("s are ");

                for (idx, valid) in valid.iter().enumerate() {
                    msg.add(valid.as_ref().highlight());

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
    }

    /// TODO: docs
    #[inline]
    fn suggest_closest(
        invalid: &str,
        invalid_what: &str,
        closest: impl AsRef<str>,
        msg: &mut WarningMsg,
    ) {
        msg.add("invalid ")
            .add(invalid_what)
            .add(" ")
            .add(invalid.highlight())
            .add(", did you mean ")
            .add(closest.as_ref().highlight())
            .add("?");
    }
}
