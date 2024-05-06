/// TODO: docs
#[derive(Debug, Default, Copy, Clone)]
pub struct ExpandRect<T> {
    top: T,
    bottom: T,
    left: T,
    right: T,
}

impl<T> ExpandRect<T> {
    /// Creates a new [`ExpandRect`] with the given top, bottom, left, and right values.
    #[inline]
    pub fn new(top: T, bottom: T, left: T, right: T) -> Self {
        Self { top, bottom, left, right }
    }
}

impl<T: Copy> ExpandRect<T> {
    /// Sets the left and right edges of the [`ExpandRect`] to the given value.
    #[inline]
    pub fn x(mut self, expand_x_by: T) -> Self {
        self.left = expand_x_by;
        self.right = expand_x_by;
        self
    }

    /// Sets the top and bottom edges of the [`ExpandRect`] to the given value.
    #[inline]
    pub fn y(mut self, expand_y_by: T) -> Self {
        self.top = expand_y_by;
        self.bottom = expand_y_by;
        self
    }
}
