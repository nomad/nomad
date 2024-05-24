use core::marker::PhantomData;

use crate::{Bound, Cells, Render, View};

/// TODO: docs
pub struct Popover {
    /// TODO: docs
    _anchor: PopoverAnchor,

    /// TODO: docs
    _view: View,
}

impl Popover {
    /// TODO: docs
    #[inline]
    pub fn builder() -> PopoverBuilder<RootRender> {
        PopoverBuilder {
            anchor: PopoverAnchor::Editor,
            root: Box::new(()),
            _state: PhantomData,
        }
    }

    /// TODO: docs
    pub fn lines(&self) -> Vec<String> {
        self._view.lines()
    }
}

// the first time a popover is opened, we:
//
// - get the available size we have;
// - ask the root render its layout. together with the total size this will
// determine the size of the popover;
// - set the size of the scene to that size;
// - paint the root render into the scene;
// - paint the entire scene into the view;
//
// once that's done, we'll only re-render if:
//
// - the anchor changes;
// - the position of the anchor changes;
// - the size of the terminal changes;
//
// or if:
//
// - a reactive used when rendering changes.
//
// If we change for one of the first 3 reasons, may be able to just reposition
// the view without re-rendering it. to determine this:
//
// - get the new available size;
// - if it's the same, we're done;
// - if the last time we rendered the requested bound was:
//   * Available -> we re-render from scratch;
//   * Explicit with bound > current size -> we re-render from scratch;
//   * Explicit with bound <= current size -> we just reposition the view;
//
// If we change because a reactive changed, we do re-layout and re-paint,
// incrementally. The `Scene` will then tell us how to change the view while
// doing the least amount of work possible. This means we shouldn't re-render
// everything if the size or layout changes.

/// TODO: docs
pub enum PopoverAnchor {
    /// TODO: docs
    Cursor,

    /// TODO: docs
    Editor,
}

impl PopoverAnchor {
    /// Returns the maximum size a popover can have when anchored to this
    /// anchor.
    #[inline]
    fn max_size(&self) -> Bound<Cells> {
        Bound::new(20, 5)
    }
}

/// TODO: docs
pub struct PopoverBuilder<State> {
    anchor: PopoverAnchor,
    root: Box<dyn Render + 'static>,
    _state: PhantomData<State>,
}

impl<State> PopoverBuilder<State> {
    #[inline]
    fn change_state<NewState>(self) -> PopoverBuilder<NewState> {
        PopoverBuilder {
            anchor: self.anchor,
            root: self.root,
            _state: PhantomData,
        }
    }
}

impl PopoverBuilder<RootRender> {
    /// TODO: docs
    #[inline]
    pub fn render<R>(mut self, root: R) -> PopoverBuilder<Anchor>
    where
        R: Render + 'static,
    {
        self.root = Box::new(root);
        self.change_state()
    }
}

impl PopoverBuilder<Anchor> {
    /// TODO: docs
    #[inline]
    pub fn anchor<A>(mut self, anchor: A) -> PopoverBuilder<Done>
    where
        A: Into<PopoverAnchor>,
    {
        self.anchor = anchor.into();
        self.change_state()
    }
}

impl PopoverBuilder<Done> {
    /// TODO: docs
    #[inline]
    pub fn open(self) -> Popover {
        let available_size = self.anchor.max_size();
        let view = View::open(self.root, available_size);
        Popover { _anchor: self.anchor, _view: view }
    }
}

use typestate::*;

mod typestate {
    /// TODO: docs.
    pub struct Anchor;

    /// TODO: docs.
    pub struct RootRender;

    /// TODO: docs.
    pub struct Done;
}
