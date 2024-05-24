use compact_str::CompactString;

use crate::{Bound, Cells, IntoRender, Render, RequestedBound, SceneFragment};

/// TODO: docs
pub struct Text {
    inner: CompactString,
    width: Cells,
}

impl Text {
    #[inline]
    pub(crate) fn new(inner: CompactString) -> Self {
        Self { width: Cells::measure(&inner), inner }
    }
}

impl Render for Text {
    #[inline]
    fn layout(&self) -> RequestedBound<Cells> {
        // TODO: is it worth counting graphemes instead of characters?
        // TODO: support soft wrapping.
        let bound = Bound::new(1u32, self.width);
        RequestedBound::Explicit(bound)
    }

    #[inline]
    fn paint(&self, mut fragment: SceneFragment) {
        let Some(line) = fragment.lines().next() else {
            return;
        };

        let mut text = &*self.inner;

        if line.width() < self.width {
            (text, _) = line.width().split(text);
        }

        line.into_run().set_text(text);
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
