use core::ops::Range;

use async_broadcast::{InactiveReceiver, Sender};
use cola::{Anchor, Replica, ReplicaId};
use crop::Rope;
use nvim::api;

use crate::runtime::spawn;
use crate::streams::{AppliedDeletion, AppliedEdit, AppliedInsertion, Edits};
use crate::{
    Apply,
    BufferSnapshot,
    ByteOffset,
    EditorId,
    IntoCtx,
    NvimBuffer,
    Replacement,
    Shared,
};

/// TODO: docs
pub struct Buffer {
    /// TODO: docs
    broadcast: Shared<BroadcastNvimReplacement>,

    /// TODO: docs
    inner: Shared<BufferInner>,

    /// TODO: docs
    nvim: NvimBuffer,

    /// TODO: docs
    receiver: InactiveReceiver<AppliedEdit>,

    /// TODO: docs
    sender: Sender<AppliedEdit>,
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
    pub fn edit<E>(
        &mut self,
        edit: E,
        _editor_id: EditorId,
    ) -> <Self as Apply<E>>::Diff
    where
        Self: Apply<E>,
    {
        self.apply(edit)
    }

    /// TODO: docs
    #[inline]
    pub fn edits(&self) -> Edits {
        Edits::new(self.receiver.activate_cloned())
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
        let (sender, receiver) = async_broadcast::broadcast(32);

        let this = Self {
            broadcast: Shared::new(BroadcastNvimReplacement::Broadcast),
            inner: Shared::new(inner),
            nvim: bound_to,
            receiver: receiver.deactivate(),
            sender,
        };

        this.attach();

        this
    }

    #[inline]
    fn on_edit(&self) -> impl Fn(&Replacement<ByteOffset>) + 'static {
        let broadcast = self.broadcast.clone();
        let inner = self.inner.clone();
        let sender = self.sender.clone();

        move |replacement| {
            if let BroadcastNvimReplacement::Broadcast = broadcast.get() {
                let (del, ins) =
                    inner.with_mut(|inner| inner.edit(replacement));

                let id = EditorId::unknown();

                if let Some(deletion) = del {
                    let edit = AppliedEdit::deletion(deletion, id);
                    broadcast_edit(&sender, edit);
                }

                if let Some(insertion) = ins {
                    let edit = AppliedEdit::insertion(insertion, id);
                    broadcast_edit(&sender, edit);
                }
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
#[derive(Copy, Clone, Debug)]
enum BroadcastNvimReplacement {
    /// The [`NvimBuffer`] is not being edited on our side, so replacements
    /// should be broadcasted.
    Broadcast,

    /// An edit is currently being applied to the [`Buffer`], so replacements
    /// should not be re-broadcasted.
    DontBroadcast,
}

/// TODO: docs
#[inline]
fn broadcast_edit(sender: &Sender<AppliedEdit>, edit: AppliedEdit) {
    if sender.receiver_count() > 0 {
        let sender = sender.clone();

        spawn(async move {
            if sender.receiver_count() > 0 {
                let _ = sender.broadcast_direct(edit).await;
            }
        });
    }
}

impl Apply<Replacement<ByteOffset>> for Buffer {
    type Diff = ();

    #[inline]
    fn apply(&mut self, repl: Replacement<ByteOffset>) -> Self::Diff {
        let point_range =
            self.inner.with(|inner| repl.range().into_ctx(inner.rope()));

        self.inner.with_mut(|inner| {
            let range = repl.range().start.into()..repl.range().end.into();
            inner.rope_mut().replace(range.clone(), repl.text());
            let _ = inner.replica_mut().deleted(range.clone());
            let _ =
                inner.replica_mut().inserted(range.start, repl.text().len());
        });

        self.broadcast.set(BroadcastNvimReplacement::DontBroadcast);
        self.nvim.edit(repl.map_range(|_| point_range));
        self.broadcast.set(BroadcastNvimReplacement::Broadcast);
    }
}

impl Apply<Replacement<Anchor>> for Buffer {
    type Diff = ();

    #[inline]
    fn apply(&mut self, repl: Replacement<Anchor>) -> Self::Diff {
        let anchor_range = repl.range();

        let (start, end) = self.inner.with(|inner| {
            let start = inner.resolve_anchor(&anchor_range.start);
            let end = inner.resolve_anchor(&anchor_range.end);
            (start, end)
        });

        if let (Some(start), Some(end)) = (start, end) {
            self.apply(repl.map_range(|_| start..end));
        }
    }
}

impl<T: AsRef<str>> Apply<(&cola::Insertion, T)> for Buffer {
    type Diff = ();

    #[inline]
    fn apply(
        &mut self,
        (insertion, text): (&cola::Insertion, T),
    ) -> Self::Diff {
        let maybe_point = self.inner.with_mut(|inner| {
            let offset = inner.replica_mut().integrate_insertion(insertion)?;
            inner.rope_mut().insert(offset, text.as_ref());
            Some(ByteOffset::new(offset).into_ctx(inner.rope()))
        });

        if let Some(point) = maybe_point {
            self.broadcast.set(BroadcastNvimReplacement::DontBroadcast);
            self.nvim.edit(Replacement::insertion(point, text.as_ref()));
            self.broadcast.set(BroadcastNvimReplacement::Broadcast);
        }
    }
}

impl Apply<&cola::Deletion> for Buffer {
    type Diff = ();

    #[inline]
    fn apply(&mut self, deletion: &cola::Deletion) -> Self::Diff {
        let byte_ranges = self.inner.with_mut(|inner| {
            inner.replica_mut().integrate_deletion(deletion)
        });

        let point_ranges = byte_ranges
            .iter()
            .cloned()
            .map(|range| {
                ByteOffset::from(range.start)..ByteOffset::from(range.end)
            })
            .map(|range| {
                self.inner.with(|inner| range.into_ctx(inner.rope()))
            });

        self.broadcast.set(BroadcastNvimReplacement::DontBroadcast);

        for point_range in point_ranges {
            self.nvim.edit(Replacement::deletion(point_range));
        }

        self.broadcast.set(BroadcastNvimReplacement::Broadcast);

        for byte_range in byte_ranges.into_iter().rev() {
            self.inner.with_mut(|inner| inner.rope_mut().delete(byte_range));
        }
    }
}

/// TODO: docs
#[derive(Clone)]
pub(super) struct BufferInner {
    /// TODO: docs
    replica: Replica,

    /// TODO: docs
    text: Rope,
}

impl BufferInner {
    /// TODO: docs
    #[inline]
    pub fn delete(&mut self, range: Range<ByteOffset>) -> cola::Deletion {
        let range: Range<usize> = range.start.into()..range.end.into();
        self.text.delete(range.clone());
        self.replica.deleted(range)
    }

    /// TODO: docs
    #[inline]
    pub fn edit<E>(&mut self, edit: E) -> <Self as Apply<E>>::Diff
    where
        Self: Apply<E>,
    {
        self.apply(edit)
    }

    /// TODO: docs
    #[inline]
    pub fn insert(
        &mut self,
        offset: ByteOffset,
        text: &str,
    ) -> cola::Insertion {
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

    /// Returns an exclusive reference to the buffer's [`Replica`].
    #[inline]
    pub(crate) fn replica_mut(&mut self) -> &mut Replica {
        &mut self.replica
    }

    /// TODO: docs
    #[inline]
    pub fn resolve_anchor(&self, anchor: &Anchor) -> Option<ByteOffset> {
        self.replica.resolve_anchor(*anchor).map(ByteOffset::new)
    }

    /// Returns a shared reference to the buffer's [`Rope`].
    #[inline]
    pub(crate) fn rope(&self) -> &Rope {
        &self.text
    }

    /// Returns an exclusive reference to the buffer's [`Rope`].
    #[inline]
    pub(crate) fn rope_mut(&mut self) -> &mut Rope {
        &mut self.text
    }

    /// TODO: docs
    #[inline]
    pub fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot::new(self.replica.clone(), self.text.clone())
    }
}

impl Apply<&Replacement<ByteOffset>> for BufferInner {
    type Diff = (Option<AppliedDeletion>, Option<AppliedInsertion>);

    #[inline]
    fn apply(&mut self, repl: &Replacement<ByteOffset>) -> Self::Diff {
        let mut applied_del = None;
        let mut applied_ins = None;

        if !repl.range().is_empty() {
            let del = self.delete(repl.range());
            applied_del = Some(AppliedDeletion::new(del));
        }

        if !repl.text().is_empty() {
            let ins = self.insert(repl.range().start, repl.text());
            applied_ins =
                Some(AppliedInsertion::new(ins, repl.text().to_owned()));
        }

        (applied_del, applied_ins)
    }
}
