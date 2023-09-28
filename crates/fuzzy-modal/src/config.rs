use common::WindowConfig;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub window: WindowConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: WindowConfig::new()
                .at_x(0.325)
                .at_y(0.15)
                .with_width(0.35)
                .with_height(0.45),
        }
    }
}
