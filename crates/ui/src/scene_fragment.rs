use crate::Cells;

/// TODO: docs.
pub struct SceneFragment<'a> {
    _a: &'a (),
}

impl<'a> SceneFragment<'a> {
    /// TODO: docs
    #[inline]
    pub fn cutout<C: Cutout>(self, cutout: C) -> (Self, C::Cutout<'a>) {
        cutout.cutout(self)
    }

    /// TODO: docs
    #[inline]
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// TODO: docs
    #[inline]
    pub fn height(&self) -> Cells {
        todo!()
    }

    /// TODO: docs
    #[inline]
    pub fn split_x(self, _split_at: Cells) -> (Self, Self) {
        todo!()
    }

    /// TODO: docs
    #[inline]
    pub fn split_y(self, _split_at: Cells) -> (Self, Self) {
        todo!()
    }

    /// TODO: docs
    #[inline]
    pub fn width(&self) -> Cells {
        todo!()
    }
}

/// TODO: docs.
pub trait Cutout {
    /// TODO: docs.
    type Cutout<'a>;

    /// TODO: docs.
    fn cutout(
        self,
        fragment: SceneFragment,
    ) -> (SceneFragment, Self::Cutout<'_>);
}
