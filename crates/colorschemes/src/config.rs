use common::Enable;
use serde::de::{Deserialize, Error, Visitor};

use crate::schemes;

#[derive(Debug)]
pub struct Config {
    enabled: Option<String>,
}

impl Config {
    /// TODO: docs
    pub fn enabled_colorscheme(self) -> Option<String> {
        self.enabled
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct ConfigVisitor;

        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("a map of colorschemes configurations")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut last_enabled = None;

                while let Some(colorscheme_name) = map.next_key::<String>()? {
                    if !schemes::colorscheme_api_names()
                        .contains(&colorscheme_name.as_str())
                    {
                        return Err(V::Error::unknown_field(
                            &colorscheme_name,
                            schemes::colorscheme_api_names(),
                        ));
                    }

                    let enable = map.next_value::<Enable<()>>()?;

                    if enable.enable() {
                        last_enabled = Some(
                            schemes::api_name_to_colorscheme_name(
                                &colorscheme_name,
                            )
                            .expect("colorscheme name is valid")
                            .to_owned(),
                        );
                    }
                }

                Ok(Config { enabled: last_enabled })
            }
        }

        deserializer.deserialize_map(ConfigVisitor)
    }
}
