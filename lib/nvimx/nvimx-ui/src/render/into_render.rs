use crate::Render;

/// TODO: docs
pub trait IntoRender {
    /// TODO: docs
    type Render: Render;

    /// TODO: docs
    fn into_render(self) -> Self::Render;
}

impl<T> IntoRender for T
where
    T: Render,
{
    type Render = T;
    #[inline]
    fn into_render(self) -> Self::Render {
        self
    }
}
