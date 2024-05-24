use crate::{Cells, Component, RequestedBound, SceneFragment};

/// TODO: docs
pub trait Render {
    /// TODO: docs
    fn layout(&self) -> RequestedBound<Cells>;

    /// TODO: docs
    fn paint(&self, scene_fragment: SceneFragment);
}

impl<T: Render> Render for &T {
    #[inline]
    fn layout(&self) -> RequestedBound<Cells> {
        (*self).layout()
    }

    #[inline]
    fn paint(&self, scene_fragment: SceneFragment) {
        (*self).paint(scene_fragment)
    }
}

impl<T: Render> Component for T {
    #[inline]
    fn compose(&self) -> impl Render {
        self
    }
}
