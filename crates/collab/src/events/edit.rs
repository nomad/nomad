use core::ops::Range;

use nomad::ByteOffset;
use smallvec::{smallvec, SmallVec};
use smol_str::SmolStr;

use crate::CollabEditor;

pub(crate) type Edits = neovim::Edits;

pub(crate) struct Edit<E: CollabEditor> {
    pub(crate) file_id: E::FileId,
    pub(crate) hunks: SmallVec<[Hunk; 1]>,
}

#[derive(Clone)]
pub(crate) struct Hunk {
    pub(crate) start: ByteOffset,
    pub(crate) end: ByteOffset,
    pub(crate) text: SmolStr,
}

impl Hunk {
    pub(crate) fn deleted_byte_range(&self) -> Range<usize> {
        self.start.into()..self.end.into()
    }
}

mod neovim {
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use futures_util::Stream;
    use nomad::neovim::{self, Neovim};
    use nomad::Subscription;

    use super::*;

    pin_project_lite::pin_project! {
        pub(crate) struct Edits {
            buffer_id: neovim::BufferId,
            #[pin]
            inner: Subscription<neovim::events::EditEvent<ByteOffset>, Neovim>,
        }
    }

    impl Stream for Edits {
        type Item = super::Edit<Neovim>;

        fn poll_next(
            self: Pin<&mut Self>,
            ctx: &mut Context,
        ) -> Poll<Option<Self::Item>> {
            let this = self.project();
            this.inner.poll_next(ctx).map(|maybe_edit| {
                maybe_edit.map(|edit| Edit {
                    file_id: this.buffer_id.clone(),
                    hunks: smallvec![edit.into()],
                })
            })
        }
    }

    impl From<neovim::events::Edit<ByteOffset>> for super::Hunk {
        fn from(edit: neovim::events::Edit<ByteOffset>) -> Self {
            Self {
                start: edit.deleted_range().start,
                end: edit.deleted_range().end,
                text: edit.inserted_text().as_str().into(),
            }
        }
    }
}
