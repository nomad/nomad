use crate::{Cells, Render, RequestedBound, SceneFragment};

impl Render for () {
    #[inline]
    fn layout(&self) -> RequestedBound<Cells> {
        RequestedBound::empty()
    }

    #[inline]
    fn paint(&self, _: SceneFragment) {}
}
