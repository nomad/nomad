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
}

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
        todo!();
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
