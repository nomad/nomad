use std::collections::HashMap;
use std::convert::Infallible;

use common::nvim::{self, Object};
use common::{
    runtime::{self, Runtime},
    *,
};
use serde::de;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("invalid value for {path:?}: {err}", path = .0.path(), err = .0.inner())]
    Path(#[from] serde_path_to_error::Error<nvim::serde::Error>),
}

/// TODO: docs
pub(crate) fn config(config: Object) -> Result<(), Infallible> {
    serde_path_to_error::deserialize::<_, Option<Enable<Config>>>(
        nvim::serde::Deserializer::new(config),
    )
    .map(Option::unwrap_or_default)
    .map(|config| {
        let mad_enabled = config.enable();

        runtime::with(|rt| {
            for (plugin_name, plugin_config) in config.into_inner().0 {
                rt.update_config(
                    plugin_name.as_str(),
                    mad_enabled,
                    plugin_config,
                );
            }
        });
    })
    .or_else(|err| {
        display_error(ConfigError::from(err), None);
        Ok(())
    })
}

#[derive(Default)]
struct Config(HashMap<String, Object>);

impl<'de> de::Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ConfigVisitor;

        impl<'de> de::Visitor<'de> for ConfigVisitor {
            type Value = Config;

            fn expecting(
                &self,
                f: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                f.write_str("a dictionary")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut plugins = HashMap::new();

                while let Some(name) = map.next_key::<String>()? {
                    if runtime::with(|rt| !rt.is_registered(name.as_str())) {
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
