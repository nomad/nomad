use core::ops::Range;

use cola::Replica;
use crop::Rope;
use flume::Sender;

use super::{BufferId, EditorId};
use crate::streams::{Deletion, Edit, Edits, Insertion};

/// TODO: docs
pub struct Buffer {
    id: BufferId,
    replica: Replica,
    text: Rope,
}

impl Buffer {
    /// TODO: docs
    #[inline]
    pub fn edits(&self) -> Edits {
        self.edits_inner(None)
    }

    /// TODO: docs
    #[inline]
    pub fn edits_filtered(&self, filter_out: EditorId) -> Edits {
        self.edits_inner(Some(filter_out))
    }

    /// TODO: docs
    #[inline]
    pub fn edits_inner(&self, filter_out: Option<EditorId>) -> Edits {
        todo!();
    }

    /// TODO: docs
    #[inline]
    pub async fn new(id: BufferId) -> Self {
        todo!();
    }

    #[inline]
    fn on_bytes(
        &self,
        sender: Sender<Edit>,
    ) -> impl Fn(ByteEdit) -> bool + 'static {
        move |edit| {
            let byte_range = edit.byte_range();

            let text = rope();

            text.replace(byte_range.clone(), &edit.replacement);

            let replica = replica();

            if !byte_range.is_empty() {
                let del = replica.deleted(byte_range.clone());
                let del = Deletion::new(del);
                let _ = sender.send(Edit::Deletion(del));
            }

            let text_len = edit.replacement.len();

            if text_len > 0 {
                let ins = replica.inserted(byte_range.start, text_len);
                let ins = Insertion::new(ins, edit.replacement);
                let _ = sender.send(Edit::Insertion(ins));
            }

            false
        }
    }
}

type ByteOffset = usize;

/// TODO: docs
struct ByteEdit {
    start: ByteOffset,
    end: ByteOffset,
    replacement: String,
}

impl ByteEdit {
    #[inline]
    fn byte_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

fn rope() -> &'static mut Rope {
    todo!()
}

fn replica() -> &'static mut Replica {
    todo!()
}
