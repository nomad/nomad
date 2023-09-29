use nvim::api::types::{WindowConfig as NvimWindowConfig, *};
use serde::Deserialize;

use crate::nvim;

/// TODO: docs
#[derive(Clone, Debug, Deserialize)]
pub struct WindowConfig {
    tot_width: u16,
    tot_height: u16,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    border: WindowBorder,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowConfig {
    pub fn new() -> Self {
        Self {
            tot_width: columns(),
            tot_height: lines(),
            border: WindowBorder::None,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn at_x(mut self, x: impl Into<ScreenUnit>) -> Self {
        self.x = x.into().to_cells(self.tot_width);
        self
    }

    pub fn at_y(mut self, y: impl Into<ScreenUnit>) -> Self {
        self.y = y.into().to_cells(self.tot_height);
        self
    }

    pub fn with_border(mut self, border: WindowBorder) -> Self {
        self.border = border;
        self
    }

    pub fn with_width(mut self, width: impl Into<ScreenUnit>) -> Self {
        self.width = width.into().to_cells(self.tot_width);
        self
    }

    pub fn with_height(mut self, height: impl Into<ScreenUnit>) -> Self {
        self.height = height.into().to_cells(self.tot_height);
        self
    }

    pub fn bisect_horizontal(self, at: u16) -> (Self, Self) {
        let left_width = at;
        let right_width = self.width - left_width;
        let left = Self { width: left_width, ..self.clone() };
        let right = Self { width: right_width, x: left_width, ..self };
        (left, right)
    }

    pub fn bisect_vertical(self, at: u16) -> (Self, Self) {
        let top_height = at;
        let bottom_height = self.height - top_height;
        let top = Self { height: top_height, ..self.clone() };
        let bottom =
            Self { height: bottom_height, y: self.y + top_height, ..self };
        (top, bottom)
    }

    pub fn shift_down(&mut self, amount: u16) {
        self.y += amount;
    }
}

impl From<&WindowConfig> for NvimWindowConfig {
    fn from(config: &WindowConfig) -> Self {
        Self::builder()
            .width(config.width as _)
            .height(config.height as _)
            .col(config.x)
            .row(config.y)
            .border(config.border.clone())
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
    fn to_cells(self, total: u16) -> u16 {
        match self {
            Self::Percent(percent) => (total as f32 * percent) as u16,
            Self::Cells(cells) => cells,
        }
    }
}

impl From<u16> for ScreenUnit {
    fn from(cells: u16) -> Self {
        Self::Cells(cells)
    }
}

impl From<f32> for ScreenUnit {
    fn from(percent: f32) -> Self {
        Self::Percent(percent)
    }
}

fn columns() -> u16 {
    nvim::api::get_option::<u16>("columns")
        .expect("the 'columns' option exists")
}

fn lines() -> u16 {
    nvim::api::get_option::<u16>("lines").expect("the 'columns' option exists")
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
