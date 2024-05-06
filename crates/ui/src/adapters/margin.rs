use crate::{Cells, ExpandRect, Render, RequestedBound, SceneFragment};

/// TODO: docs
pub struct Margin<R> {
    inner: R,
    _expand: ExpandRect<Cells>,
}

impl<R> Margin<R> {
    #[inline]
    pub(crate) fn new(inner: R, expand: ExpandRect<Cells>) -> Self {
        Self { inner, _expand: expand }
    }
}

impl<R: Render> Render for Margin<R> {
    #[inline]
    fn layout(&self) -> RequestedBound<Cells> {
        self.inner.layout()
    }

    #[inline]
    fn paint(&self, scene_fragment: &mut SceneFragment) {
        self.inner.paint(scene_fragment)
    }
}
