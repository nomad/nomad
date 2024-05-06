use crate::Render;

/// TODO: docs
pub trait IntoRender {
    /// TODO: docs
    fn into_render(self) -> impl Render;
}

impl<T> IntoRender for T
where
    T: Render,
{
    #[inline]
    fn into_render(self) -> impl Render {
        self
    }
}
