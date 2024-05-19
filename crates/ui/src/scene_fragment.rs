use alloc::rc::Rc;
use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::{Bound, Cells, Point, Scene};

/// TODO: docs.
pub struct SceneFragment<'scene> {
    /// TODO: docs.
    ptr: NonNull<Scene>,

    /// The origin of the fragment.
    ///
    /// The sub-area of the scene represented by this fragment is the rectangle
    /// given by [`Self::size`] with its top-left corner at this point.
    origin: Point<Cells>,

    /// The size of the fragment.
    size: Bound<Cells>,

    /// Whether the fragment is currently borrowed. This is needed to make
    /// accessing the scene via a raw pointer safe.
    borrow: Rc<Cell<Borrow>>,

    _lifetime: PhantomData<&'scene mut Scene>,
}

enum Borrow {
    Borrowed,
    NotBorrowed,
}

impl<'scene> SceneFragment<'scene> {
    /// TODO: docs
    #[inline]
    pub fn cutout<C: Cutout>(self, cutout: C) -> (Self, C::Cutout<'scene>) {
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
    pub(crate) fn new(scene: &'scene mut Scene) -> Self {
        Self {
            size: scene.size(),
            ptr: NonNull::from(scene),
            origin: Point::origin(),
            borrow: Rc::new(Cell::new(Borrow::NotBorrowed)),
            _lifetime: PhantomData,
        }
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
