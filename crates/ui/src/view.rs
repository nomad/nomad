use api::types::*;
use nvim::api;

use crate::{Render, Scene};

/// TODO: docs.
pub(crate) struct View {
    /// TODO: docs.
    buffer: api::Buffer,

    /// TODO: docs.
    root: Box<dyn Render + 'static>,

    /// TODO: docs.
    scene: Scene,

    /// TODO: docs.
    window: api::Window,
}

impl View {
    /// Creates a new `View` that isn't displayed on the screen.
    #[inline]
    pub(crate) fn new_hidden() -> Self {
        let buffer = api::create_buf(false, true).expect("never fails(?)");

        let config = WindowConfig::builder()
            .relative(WindowRelativeTo::Editor)
            .height(1)
            .width(1)
            .row(0)
            .col(0)
            .hide(true)
            .build();

        let _window = api::open_win(&buffer, false, &config)
            .expect("the config is valid");

        // Self { buffer, window }

        todo!();
    }
}
