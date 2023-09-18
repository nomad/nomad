use common::nvim;
use nvim::api::types::{WindowConfig as NvimWindowConfig, *};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub(crate) window: WindowConfig,
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    /// TODO: docs
    pub(crate) width: ScreenUnit,

    /// TODO: docs
    pub(crate) height: ScreenUnit,

    /// TODO: docs
    pub(crate) x: ScreenUnit,

    /// TODO: docs
    pub(crate) y: ScreenUnit,
}

impl WindowConfig {
    pub fn bisect(&self, axis: Axis, at: ScreenUnit) -> (Self, Self) {
        match axis {
            Axis::Horizontal => self.bisect_horizontal(at),
            Axis::Vertical => self.bisect_vertical(at),
        }
    }

    fn bisect_horizontal(&self, at: ScreenUnit) -> (Self, Self) {
        let left = at.to_cells(Axis::Horizontal);
        let right = self.width.to_cells(Axis::Horizontal) - left;
        (
            Self { width: ScreenUnit::Cells(left), ..*self },
            Self {
                width: ScreenUnit::Cells(right),
                x: ScreenUnit::Cells(left),
                ..*self
            },
        )
    }

    fn bisect_vertical(&self, at: ScreenUnit) -> (Self, Self) {
        let top = at.to_cells(Axis::Vertical);
        let bottom = self.height.to_cells(Axis::Vertical) - top;
        let y = self.y.to_cells(Axis::Vertical);
        (
            Self { height: ScreenUnit::Cells(top), ..*self },
            Self {
                height: ScreenUnit::Cells(bottom),
                y: ScreenUnit::Cells(y + top),
                ..*self
            },
        )
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: ScreenUnit::Percent(0.7),
            height: ScreenUnit::Percent(0.7),
            x: ScreenUnit::Percent(0.15),
            y: ScreenUnit::Percent(0.15),
        }
    }
}

impl From<&WindowConfig> for NvimWindowConfig {
    fn from(config: &WindowConfig) -> Self {
        let WindowConfig { width, height, x, y } = config;

        let width = width.to_cells(Axis::Horizontal);
        let height = height.to_cells(Axis::Vertical);
        let x = x.to_cells(Axis::Horizontal);
        let y = y.to_cells(Axis::Vertical);

        Self::builder()
            .width(width.into())
            .height(height.into())
            .col(x)
            .row(y)
            .anchor(WindowAnchor::NorthWest)
            .relative(WindowRelativeTo::Editor)
            .focusable(false)
            .style(WindowStyle::Minimal)
            .build()
    }
}

/// TODO: docs
#[derive(Copy, Clone, Debug)]
pub enum ScreenUnit {
    /// TODO: docs
    Percent(f32),

    /// TODO: docs
    Cells(u16),
}

impl ScreenUnit {
    fn to_cells(self, axis: Axis) -> u16 {
        match self {
            Self::Percent(percent) => {
                let total = match axis {
                    Axis::Horizontal => columns(),
                    Axis::Vertical => lines(),
                };

                (total as f32 * percent) as u16
            },
            Self::Cells(cells) => cells,
        }
    }
}

fn columns() -> u16 {
    nvim::api::get_option::<u16>("columns")
        .expect("the 'columns' option exists")
}

fn lines() -> u16 {
    nvim::api::get_option::<u16>("lines").expect("the 'columns' option exists")
}

/// TODO: docs
#[derive(Copy, Clone, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

mod custom_deserialize {
    use serde::de::{Deserialize, Error, Visitor};

    use super::*;

    impl<'de> Deserialize<'de> for ScreenUnit {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::de::Deserializer<'de>,
        {
            struct ScreenUnitVisitor;

            impl<'de> Visitor<'de> for ScreenUnitVisitor {
                type Value = ScreenUnit;

                fn expecting(
                    &self,
                    formatter: &mut std::fmt::Formatter,
                ) -> std::fmt::Result {
                    formatter.write_str("a screen unit")
                }

                fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    if !(0.0..=1.0).contains(&value) {
                        return Err(Error::custom(
                            "fractional screen unit must be between 0.0 and \
                             1.0",
                        ));
                    }
                    Ok(ScreenUnit::Percent(value as f32))
                }

                fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(ScreenUnit::Cells(value as u16))
                }
            }

            deserializer.deserialize_str(ScreenUnitVisitor)
        }
    }
}
