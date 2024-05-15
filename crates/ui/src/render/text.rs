use compact_str::CompactString;
use str_indices::chars;

use crate::{Bound, Cells, IntoRender, Render, RequestedBound, SceneFragment};

/// TODO: docs
pub struct Text {
    inner: CompactString,
}

impl Text {
    #[inline]
    pub(crate) fn new(inner: CompactString) -> Self {
        Self { inner }
    }
}

impl Render for Text {
    #[inline]
    fn layout(&self) -> RequestedBound<Cells> {
        // TODO: is it worth counting graphemes instead of characters?
        // TODO: support soft wrapping.
        let bound = Bound::new(1u32, chars::count(&self.inner) as u32);
        RequestedBound::Explicit(bound)
    }

    #[inline]
    fn paint(&self, _scene_fragment: SceneFragment) {
        // TODO: support soft wrapping.
        todo!()
    }
}

impl<S: AsRef<str>> From<S> for Text {
    #[inline]
    fn from(value: S) -> Self {
        Self::new(value.as_ref().into())
    }
}

impl IntoRender for &str {
    type Render = Text;

    #[inline]
    fn into_render(self) -> Self::Render {
        self.into()
    }
}

impl IntoRender for String {
    type Render = Text;

    #[inline]
    fn into_render(self) -> Self::Render {
        self.into()
    }
}
