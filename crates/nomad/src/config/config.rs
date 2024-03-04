use core::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use nvim::{serde::Deserializer, Object};
use serde::de::Deserialize;

use super::EnableConfig;
use crate::ctx::{Ctx, Set};
use crate::prelude::{nvim, Module, ModuleName};

thread_local! {
    /// TODO: docs
    static DESERIALIZERS: ConfigDeserializers = ConfigDeserializers::new();
}

/// TODO: docs
pub(crate) fn config() -> nvim::Function<Object, ()> {
    todo!();
}

/// TODO: docs
#[inline]
pub(crate) fn with_module<M>(set_config: Set<EnableConfig<M>>, ctx: &Ctx)
where
    M: Module,
{
    let deserializer = ConfigDeserializer::new(set_config, ctx.clone());

    DESERIALIZERS.with(move |deserializers| {
        deserializers.insert(M::NAME, deserializer)
    });
}

/// TODO: docs
struct ConfigDeserializers {
    deserializers: OnceCell<RefCell<HashMap<ModuleName, ConfigDeserializer>>>,
}

impl ConfigDeserializers {
    /// TODO: docs
    #[inline]
    fn insert(&self, name: ModuleName, deserializer: ConfigDeserializer) {
        self.with_map(|map| map.insert(name, deserializer));
    }

    /// TODO: docs
    const fn new() -> Self {
        Self { deserializers: OnceCell::new() }
    }

    /// TODO: docs
    #[inline]
    fn with_map<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut HashMap<ModuleName, ConfigDeserializer>) -> R,
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
