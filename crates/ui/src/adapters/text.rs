use compact_str::CompactString;

use crate::{Cells, IntoRender, Render, RequestedBound, SceneFragment};

/// TODO: docs
pub struct Text {
    _inner: CompactString,
}

impl Text {
    #[inline]
    pub(crate) fn new(inner: CompactString) -> Self {
        Self { _inner: inner }
    }
}

impl Render for Text {
    #[inline]
    fn layout(&self) -> RequestedBound<Cells> {
        todo!()
    }

    #[inline]
    fn paint(&self, _scene_fragment: &mut SceneFragment) {
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
    #[inline]
    fn into_render(self) -> Text {
        self.into()
    }
}

impl IntoRender for String {
    #[inline]
    fn into_render(self) -> Text {
        self.into()
    }
}
