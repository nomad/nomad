use core::ops::Range;

use async_broadcast::{InactiveReceiver, Receiver, Sender};
use cola::{Anchor, Replica, ReplicaId};
use crop::Rope;
use nvim::api;

use crate::runtime::spawn;
use crate::streams::Edits;
use crate::{
    utils,
    Apply,
    BufferSnapshot,
    ByteOffset,
    CrdtReplacement,
    Edit,
    EditorId,
    IntoCtx,
    NvimBuffer,
    Replacement,
    Shared,
};

/// TODO: docs
pub struct Buffer {
    /// TODO: docs
    broadcast_status: Shared<BroadcastStatus>,

    /// TODO: docs
    broadcaster: EditBroadcaster,

    /// TODO: docs
    inner: Shared<BufferInner>,

    /// TODO: docs
    nvim: NvimBuffer,
}

impl Buffer {
    /// TODO: docs
    #[inline]
    fn attach(&self) {
        self.nvim.on_edit(self.on_edit());
    }

    /// TODO: docs
    #[inline]
    pub fn create(text: &str, replica: Replica) -> Self {
        let inner = BufferInner::new(text, replica);

        let mut buf = NvimBuffer::create();

        let Ok(()) = buf.inner_mut().set_lines(.., true, text.lines()) else {
            unreachable!()
        };

        let Ok(()) = api::Window::current().set_buf(buf.inner()) else {
            unreachable!()
        };

        Self::new(inner, buf)
    }

    /// TODO: docs
    #[inline]
    pub fn edit<E>(&mut self, edit: E, editor_id: EditorId)
    where
        Self: Apply<E, Diff = Edit>,
    {
        let edit = self.apply(edit);
        self.broadcaster.broadcast(edit.with_editor(editor_id));
    }

    /// TODO: docs
    #[inline]
    pub fn edits(&self) -> Edits {
        Edits::new(self.broadcaster.receiver())
    }

    /// TODO: docs
    ///
    /// # Panics
    ///
    /// todo.
    #[inline]
    pub fn from_id(replica_id: ReplicaId, buffer: NvimBuffer) -> Self {
        let text = Rope::try_from(&buffer).expect("");
        let replica = Replica::new(replica_id, text.byte_len());
        Self::new(BufferInner::new(text, replica), buffer)
    }

    #[inline]
    fn new(inner: BufferInner, bound_to: NvimBuffer) -> Self {
        let this = Self {
            broadcast_status: Shared::new(BroadcastStatus::Broadcast),
            inner: Shared::new(inner),
            nvim: bound_to,
            broadcaster: EditBroadcaster::new(),
        };

        this.attach();

        this
    }

    #[inline]
    fn on_edit(&self) -> impl Fn(&Replacement<ByteOffset>) + 'static {
        let inner = self.inner.clone();
        let broadcaster = self.broadcaster.clone();
        let should_broadcast = self.broadcast_status.clone();

        move |replacement| {
            if should_broadcast.get().should_broadcast() {
                let edit = inner
                    .with_mut(|inner| inner.apply(replacement.clone()))
                    .with_editor(EditorId::unknown());

                broadcaster.broadcast(edit);
            }
        }
    }

    /// TODO: docs
    #[inline]
    pub fn snapshot(&self) -> BufferSnapshot {
        self.inner.with(BufferInner::snapshot)
    }
}

/// TODO: docs
#[derive(Clone)]
struct EditBroadcaster {
    receiver: InactiveReceiver<Edit>,
    sender: Sender<Edit>,
}

impl EditBroadcaster {
    #[inline]
    fn broadcast(&self, edit: Edit) {
        if self.receiver.receiver_count() > 0 {
            let sender = self.sender.clone();

            spawn(async move {
                if sender.receiver_count() > 0 {
                    let _ = sender.broadcast_direct(edit).await;
                }
            });
        }
    }

    #[inline]
    fn new() -> Self {
        let (sender, receiver) = async_broadcast::broadcast(32);
        Self { sender, receiver: receiver.deactivate() }
    }

    #[inline]
    fn receiver(&self) -> Receiver<Edit> {
        self.receiver.activate_cloned()
    }
}

/// TODO: docs
#[derive(Copy, Clone, Debug)]
enum BroadcastStatus {
    /// The [`NvimBuffer`] is not being edited on our side, so replacements
    /// should be broadcasted.
    Broadcast,

    /// An edit is currently being applied to the [`Buffer`], so replacements
    /// should not be re-broadcasted.
    DontBroadcast,
}

impl BroadcastStatus {
    #[inline]
    fn should_broadcast(&self) -> bool {
        matches!(self, Self::Broadcast)
    }
}

impl Apply<Replacement<ByteOffset>> for Buffer {
    type Diff = Edit;

    #[inline]
    fn apply(&mut self, replacement: Replacement<ByteOffset>) -> Self::Diff {
        let point_range =
            self.inner.with(|inner| replacement.range().into_ctx(&inner.text));

        self.broadcast_status.set(BroadcastStatus::DontBroadcast);
        self.nvim.edit(replacement.clone().map_range(|_| point_range));
        self.broadcast_status.set(BroadcastStatus::Broadcast);

        self.inner.with_mut(|inner| inner.apply(replacement))
    }
}

impl Apply<Replacement<Anchor>> for Buffer {
    type Diff = Edit;

    #[inline]
    fn apply(&mut self, repl: Replacement<Anchor>) -> Self::Diff {
        let anchor_range = repl.range();

        let (start, end) = self.inner.with(|inner| {
            let start = inner.resolve_anchor(&anchor_range.start);
            let end = inner.resolve_anchor(&anchor_range.end);
            (start, end)
        });

        if let (Some(start), Some(end)) = (start, end) {
            self.apply(repl.map_range(|_| start..end))
        } else {
            Edit::no_op()
        }
    }
}

impl<T: AsRef<str>> Apply<(cola::Insertion, T)> for Buffer {
    type Diff = Edit;

    #[inline]
    fn apply(
        &mut self,
        (insertion, text): (cola::Insertion, T),
    ) -> Self::Diff {
        let text = text.as_ref();

        let Some(offset) = self.inner.with_mut(|inner| {
            let off = inner.replica.integrate_insertion(&insertion)?;
            inner.text.insert(off, text);
            Some(ByteOffset::new(off))
        }) else {
            return Edit::no_op();
        };

        let point = self.inner.with(|inner| offset.into_ctx(&inner.text));

        self.broadcast_status.set(BroadcastStatus::DontBroadcast);
        self.nvim.edit(Replacement::insertion(point, text));
        self.broadcast_status.set(BroadcastStatus::Broadcast);

        Edit::remote_insertion(offset, text, insertion)
    }
}

impl Apply<cola::Deletion> for Buffer {
    type Diff = Edit;

    #[inline]
    fn apply(&mut self, deletion: cola::Deletion) -> Self::Diff {
        let byte_ranges = self
            .inner
            .with_mut(|inner| inner.replica.integrate_deletion(&deletion));

        let point_ranges =
            byte_ranges.iter().cloned().map(utils::into_byte_range).map(
                |range| self.inner.with(|inner| range.into_ctx(&inner.text)),
            );

        self.broadcast_status.set(BroadcastStatus::DontBroadcast);

        for point_range in point_ranges {
            self.nvim.edit(Replacement::deletion(point_range));
        }

        self.broadcast_status.set(BroadcastStatus::Broadcast);

        for byte_range in byte_ranges.iter().cloned().rev() {
            self.inner.with_mut(|inner| inner.text.delete(byte_range));
        }

        Edit::remote_deletion(
            byte_ranges.into_iter().map(utils::into_byte_range),
            deletion,
        )
    }
}

/// TODO: docs
#[derive(Clone)]
struct BufferInner {
    /// TODO: docs
    replica: Replica,

    /// TODO: docs
    text: Rope,
}

impl BufferInner {
    #[inline]
    fn delete(&mut self, range: Range<ByteOffset>) -> cola::Deletion {
        let range: Range<usize> = range.start.into()..range.end.into();
        self.text.delete(range.clone());
        self.replica.deleted(range)
    }

    #[inline]
    fn insert(&mut self, offset: ByteOffset, text: &str) -> cola::Insertion {
        self.text.insert(offset.into(), text);
        self.replica.inserted(offset.into(), text.len())
    }

    #[inline]
    fn new(text: impl Into<Rope>, replica: Replica) -> Self {
        let text = text.into();

        assert_eq!(
            text.byte_len(),
            replica.len(),
            "text and replica out of sync"
        );

        Self { replica, text }
    }

    #[inline]
    fn resolve_anchor(&self, anchor: &Anchor) -> Option<ByteOffset> {
        self.replica.resolve_anchor(*anchor).map(ByteOffset::new)
    }

    #[inline]
    fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot::new(self.replica.clone(), self.text.clone())
    }
}

impl Apply<Replacement<ByteOffset>> for BufferInner {
    type Diff = Edit;

    #[inline]
    fn apply(&mut self, repl: Replacement<ByteOffset>) -> Self::Diff {
        let mut crdt = CrdtReplacement::new_no_op();

        if !repl.range().is_empty() {
            crdt.with_deletion(self.delete(repl.range()));
        }

        if !repl.text().is_empty() {
            crdt.with_insertion(self.insert(repl.range().start, repl.text()));
        }

        Edit::local(repl, crdt)
    }
}
