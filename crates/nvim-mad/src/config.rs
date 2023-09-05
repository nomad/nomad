use std::collections::HashMap;
use std::convert::Infallible;

use common::oxi::{self, Object};
use common::*;
use serde::de;

use crate::runtime;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("invalid value for {path:?}: {err}", path = .0.path(), err = .0.inner())]
    Path(#[from] serde_path_to_error::Error<oxi::serde::Error>),
}

/// TODO: docs
#[inline]
pub(crate) fn config(config: Object) -> Result<(), Infallible> {
    serde_path_to_error::deserialize::<_, Enable<Config>>(
        oxi::serde::Deserializer::new(config),
    )
    .map(|config| {
        let mad_enabled = config.enable();
        runtime::with(|rt| {
            for (plugin_name, plugin_config) in config.into_inner().0 {
                rt.get_plugin_mut(plugin_name.as_str())
                    .expect("key checked during deserialization")
                    .config(mad_enabled, plugin_config);
            }
        });
    })
    .or_else(|err| {
        display_error(ConfigError::from(err), None);
        Ok(())
    })
}

struct Config(HashMap<String, Object>);

impl<'de> de::Deserialize<'de> for Config {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ConfigVisitor;

        impl<'de> de::Visitor<'de> for ConfigVisitor {
            type Value = Config;

            #[inline]
            fn expecting(
                &self,
                f: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                f.write_str("a dictionary")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut plugins = HashMap::new();

                while let Some(name) = map.next_key::<String>()? {
                    if runtime::with(|rt| rt.get_plugin_mut(&name).is_none()) {
                        return Err(de::Error::unknown_field(
                            &name,
                            runtime::plugin_names(),
                        ));
                    }
                    let value = map.next_value()?;
                    plugins.insert(name, value);
                }

                Ok(Config(plugins))
            }
        }

        deserializer.deserialize_map(ConfigVisitor)
    }
}
