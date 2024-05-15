use crate::{SceneFragment, View};

/// TODO: docs
pub(crate) struct Scene {}

impl Scene {
    /// Turns the entire `Scene` into a `SceneFragment` which can be used in
    /// the [`paint`](crate::Render::paint) method of a
    /// [`Render`](crate::Render) implementation.
    #[inline]
    pub(crate) fn as_fragment(&mut self) -> SceneFragment<'_> {
        todo!()
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn diff(&self) -> SceneDiff {
        todo!();
    }
}

/// TODO: docs
pub(crate) struct SceneDiff {}

impl SceneDiff {
    /// TODO: docs
    #[inline]
    pub(crate) fn apply(self, _view: &mut View) {
        todo!()
    }
}
