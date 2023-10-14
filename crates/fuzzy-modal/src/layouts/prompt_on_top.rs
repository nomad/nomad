use common::*;
use nvim::api::{
    types::{WindowConfig, *},
    Buffer,
    Window,
};

use crate::*;

const UPPER_RIGHT_STRAIGHT_CORNER: char = '┌';

const UPPER_LEFT_STRAIGHT_CORNER: char = '┐';

const LOWER_RIGHT_STRAIGHT_CORNER: char = '┘';

const LOWER_LEFT_STRAIGHT_CORNER: char = '└';

const HORIZONTAL_EDGE: char = '─';

const VERTICAL_EDGE: char = '│';

const VERTICAL_RIGHT_CONNECTOR: char = '├';

const VERTICAL_LEFT_CONNECTOR: char = '┤';

const OUTER_PROMPT_Z_INDEX: u32 = 2 * OUTER_RESULTS_Z_INDEX;

const OUTER_RESULTS_Z_INDEX: u32 = 100;

/// TODO: docs
#[derive(Default)]
pub struct PromptOnTop {
    outer_prompt: OuterPrompt,
    outer_results: OuterResults,
    prompt: Prompt,
    results: Results,
}

impl Layout for PromptOnTop {
    fn open(
        &mut self,
        prompt_buffer: &Buffer,
        results_buffer: &Buffer,
        bounding_rectangle: Rectangle,
    ) -> nvim::Result<()> {
        let (outer_prompt_rect, outer_results_rect) = bounding_rectangle
            .into_screen(lines(), columns())
            .split_vertically(1);

        let prompt_rect = self.outer_prompt.open(outer_prompt_rect)?;

        let results_rect =
            self.outer_results.open(outer_results_rect, &self.outer_prompt)?;

        self.results.open(
            results_buffer,
            results_rect,
            &self.outer_results,
        )?;

        self.prompt.open(prompt_buffer, prompt_rect, &self.outer_prompt)?;

        Ok(())
    }

    fn resize(&mut self, _inside: Rectangle) -> nvim::Result<()> {
        let _lines = lines();
        let _columns = columns();

        Ok(())
    }

    fn close(&mut self) -> nvim::Result<Option<usize>> {
        self.outer_prompt.close();
        self.outer_results.close();
        self.prompt.close();
        self.results.close()
    }

    fn select_next(&mut self) -> Option<usize> {
        self.results.select_next()
    }

    fn select_prev(&mut self) -> Option<usize> {
        self.results.select_prev()
    }
}

/// TODO: docs
fn lines() -> u16 {
    nvim::api::get_option::<u16>("lines").expect("the 'columns' option exists")
}

/// TODO: docs
fn columns() -> u16 {
    nvim::api::get_option::<u16>("columns")
        .expect("the 'columns' option exists")
}

/// TODO: docs
struct OuterPrompt {
    buffer: Buffer,
    config: WindowConfigBuilder,
    window: Option<Window>,
}

impl Default for OuterPrompt {
    fn default() -> Self {
        let buffer = nvim::api::create_buf(false, true).unwrap();

        let border = WindowBorder::Anal(
            UPPER_RIGHT_STRAIGHT_CORNER.into(),
            HORIZONTAL_EDGE.into(),
            UPPER_LEFT_STRAIGHT_CORNER.into(),
            VERTICAL_EDGE.into(),
            VERTICAL_LEFT_CONNECTOR.into(),
            HORIZONTAL_EDGE.into(),
            VERTICAL_RIGHT_CONNECTOR.into(),
            VERTICAL_EDGE.into(),
        );

        let mut config = WindowConfig::builder();

        config
            .anchor(WindowAnchor::NorthWest)
            .border(border)
            .focusable(false)
            .relative(WindowRelativeTo::Editor)
            .style(WindowStyle::Minimal)
            .zindex(OUTER_PROMPT_Z_INDEX);

        Self { buffer, config, window: None }
    }
}

impl OuterPrompt {
    fn open(
        &mut self,
        rectangle: ScreenRectangle,
    ) -> nvim::Result<ScreenRectangle> {
        let config = self
            .config
            .clone()
            .width(rectangle.width() as _)
            .height(rectangle.height() as _)
            .col(rectangle.x())
            .row(rectangle.y())
            .build();

        let window = nvim::api::open_win(&self.buffer, false, &config)?;

        self.window = Some(window);

        Ok(rectangle.shrink_horizontally(2))
    }

    fn close(&mut self) {
        if let Some(window) = self.window.take() {
            let _ = window.close(true);
        }
    }
}

/// TODO: docs
struct Prompt {
    config: WindowConfigBuilder,
    window: Option<Window>,
}

impl Default for Prompt {
    fn default() -> Self {
        let mut config = WindowConfig::builder();

        config
            .anchor(WindowAnchor::NorthWest)
            .col(1)
            .row(0)
            .focusable(true)
            .style(WindowStyle::Minimal)
            .zindex(OUTER_PROMPT_Z_INDEX);

        Self { config, window: None }
    }
}

impl Prompt {
    fn open(
        &mut self,
        buffer: &Buffer,
        rectangle: ScreenRectangle,
        outer_prompt: &OuterPrompt,
    ) -> nvim::Result<()> {
        let outer_prompt_win = outer_prompt.window.clone().unwrap();

        let config = self
            .config
            .clone()
            .relative(WindowRelativeTo::Window(outer_prompt_win))
            .width(rectangle.width() as _)
            .height(rectangle.height() as _)
            .build();

        let window = nvim::api::open_win(buffer, true, &config)?;

        self.window = Some(window);

        Ok(())
    }

    fn close(&mut self) {
        if let Some(window) = self.window.take() {
            let _ = window.close(true);
        }
    }
}

/// TODO: docs
struct OuterResults {
    buffer: Buffer,
    config: WindowConfigBuilder,
    window: Option<Window>,
}

impl Default for OuterResults {
    fn default() -> Self {
        let buffer = nvim::api::create_buf(false, true).unwrap();

        let border = WindowBorder::Anal(
            None.into(),
            None.into(),
            None.into(),
            VERTICAL_EDGE.into(),
            LOWER_RIGHT_STRAIGHT_CORNER.into(),
            HORIZONTAL_EDGE.into(),
            LOWER_LEFT_STRAIGHT_CORNER.into(),
            VERTICAL_EDGE.into(),
        );

        let mut config = WindowConfig::builder();

        config
            .anchor(WindowAnchor::NorthWest)
            .col(-1)
            .row(2)
            .border(border)
            .focusable(false)
            .style(WindowStyle::Minimal)
            .zindex(OUTER_RESULTS_Z_INDEX);

        Self { buffer, config, window: None }
    }
}

impl OuterResults {
    fn open(
        &mut self,
        rectangle: ScreenRectangle,
        outer_prompt: &OuterPrompt,
    ) -> nvim::Result<ScreenRectangle> {
        let outer_prompt_win = outer_prompt.window.clone().unwrap();

        let config = self
            .config
            .clone()
            .relative(WindowRelativeTo::Window(outer_prompt_win))
            .width(rectangle.width() as _)
            .height(rectangle.height() as _)
            .build();

        let window = nvim::api::open_win(&self.buffer, false, &config)?;

        self.window = Some(window);

        Ok(rectangle.shrink_horizontally(2))
    }

    fn close(&mut self) {
        if let Some(window) = self.window.take() {
            let _ = window.close(true);
        }
    }
}

/// TODO: docs
struct Results {
    /// TODO: docs
    config: WindowConfigBuilder,

    /// TODO: docs
    window: Option<Window>,

    /// TODO: docs
    rollover: bool,

    /// TODO: docs
    selected_result: Option<usize>,

    /// TODO: docs
    total_results: usize,
}

impl Default for Results {
    fn default() -> Self {
        let mut config = WindowConfig::builder();

        config
            .anchor(WindowAnchor::NorthWest)
            .col(1)
            .row(0)
            .focusable(false)
            .style(WindowStyle::Minimal)
            .zindex(OUTER_RESULTS_Z_INDEX);

        Self {
            config,
            window: None,
            rollover: false,
            selected_result: None,
            total_results: 0,
        }
    }
}

impl Results {
    fn open(
        &mut self,
        buffer: &Buffer,
        rectangle: ScreenRectangle,
        outer_results: &OuterResults,
    ) -> nvim::Result<()> {
        let outer_results_win = outer_results.window.clone().unwrap();

        let config = self
            .config
            .clone()
            .relative(WindowRelativeTo::Window(outer_results_win))
            .width(rectangle.width() as _)
            .height(rectangle.height() as _)
            .build();

        let window = nvim::api::open_win(buffer, false, &config)?;

        self.window = Some(window);

        Ok(())
    }

    fn close(&mut self) -> nvim::Result<Option<usize>> {
        if let Some(window) = self.window.take() {
            let _ = window.close(true);
        }

        Ok(self.selected_result)
    }

    fn is_first(&self, idx: usize) -> bool {
        idx == 0
    }

    fn is_last(&self, idx: usize) -> bool {
        idx == self.total_results - 1
    }

    fn select_idx(&mut self, idx: usize) -> Option<usize> {
        let old_selected = self.selected_result;

        self.selected_result = Some(idx);

        if old_selected != self.selected_result {
            if let Some(window) = &mut self.window {
                if old_selected.is_some() {
                    window.set_option("cursorline", true).unwrap();
                }
                window.set_cursor(idx + 1, 0).unwrap();
            }

            Some(idx)
        } else {
            None
        }
    }

    fn select_next(&mut self) -> Option<usize> {
        if self.total_results == 0 {
            return None;
        }

        let select_idx = if let Some(selected_idx) = self.selected_result {
            if !self.is_last(selected_idx) {
                selected_idx + 1
            } else if self.rollover {
                0
            } else {
                return None;
            }
        } else {
            0
        };

        self.select_idx(select_idx)
    }

    fn select_prev(&mut self) -> Option<usize> {
        if self.total_results == 0 {
            return None;
        }

        let select_idx = if let Some(selected_idx) = self.selected_result {
            if !self.is_first(selected_idx) {
                selected_idx - 1
            } else if self.rollover {
                self.total_results - 1
            } else {
                return None;
            }
        } else {
            self.total_results - 1
        };

        self.select_idx(select_idx)
    }
}
