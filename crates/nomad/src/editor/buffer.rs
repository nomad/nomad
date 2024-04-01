use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::iter;
use core::ops::Range;

use async_broadcast::{InactiveReceiver, Sender};
use cola::{Anchor, Replica};
use crop::{Rope, RopeBuilder};
use nvim::api::{self, opts, Buffer as NvimBuffer};

use super::{BufferId, BufferSnapshot, EditorId};
use crate::runtime::spawn;
use crate::streams::{AppliedDeletion, AppliedEdit, AppliedInsertion, Edits};

/// TODO: docs
pub struct Buffer {
    /// TODO: docs
    applied_queue: AppliedEditQueue,

    /// TODO: docs
    id: BufferId,

    /// TODO: docs
    inner: Rc<RefCell<BufferInner>>,

    /// TODO: docs
    receiver: InactiveReceiver<AppliedEdit>,

    /// TODO: docs
    sender: Sender<AppliedEdit>,
}

impl Buffer {
    /// TODO: docs
    #[track_caller]
    #[inline]
    pub fn apply_local_deletion(&mut self, delete_range: Range<Anchor>) {
        let mut buffer = self.inner.borrow_mut();
        let maybe_deletion = buffer.apply_local_deletion(delete_range);
        if let Some(deletion) = maybe_deletion {
            self.applied_queue.push_back(AppliedEdit::Deletion(deletion));
        }
    }

    /// TODO: docs
    #[track_caller]
    #[inline]
    pub fn apply_local_insertion(&mut self, insert_at: Anchor, text: String) {
        let mut buffer = self.inner.borrow_mut();
        let insertion = buffer.apply_local_insertion(insert_at, text);
        self.applied_queue.push_back(AppliedEdit::Insertion(insertion));
    }

    /// TODO: docs
    #[track_caller]
    #[inline]
    pub fn apply_remote_deletion(&mut self, deletion: RemoteDeletion) {
        let mut buffer = self.inner.borrow_mut();
        buffer.apply_remote_deletion(&deletion);
        self.applied_queue.push_back(AppliedEdit::Deletion(deletion.into()));
    }

    /// TODO: docs
    #[track_caller]
    #[inline]
    pub fn apply_remote_insertion(&mut self, insertion: RemoteInsertion) {
        let mut buffer = self.inner.borrow_mut();
        buffer.apply_remote_insertion(&insertion);
        self.applied_queue.push_back(AppliedEdit::Insertion(insertion.into()));
    }

    /// TODO: docs
    #[inline]
    fn attach(&self) {
        let on_bytes = self.on_bytes();

        let opts = opts::BufAttachOpts::builder()
            .on_bytes(move |args| {
                on_bytes(ByteChange::from(args));
                Ok(false)
            })
            .build();

        // This can fail if the buffer has been unloaded.
        let _ = NvimBuffer::from(self.id).attach(false, &opts);
    }

    /// TODO: docs
    #[inline]
    pub fn create(text: &str) -> Self {
        let Ok(mut buf) = api::create_buf(true, false) else { unreachable!() };

        let Ok(()) = buf.set_lines(.., true, text.lines()) else {
            unreachable!()
        };

        let mut win = api::Window::current();

        let Ok(()) = win.set_buf(&buf) else { unreachable!() };

        Self::new(buf.into())
    }

    /// TODO: docs
    #[inline]
    pub fn edits(&self) -> Edits {
        Edits::new(self.receiver.activate_cloned())
    }

    /// TODO: docs
    #[inline]
    pub fn new(id: BufferId) -> Self {
        let (sender, receiver) = async_broadcast::broadcast(32);

        let this = Self {
            applied_queue: AppliedEditQueue::new(),
            id,
            inner: Rc::new(RefCell::new(BufferInner::new(id))),
            receiver: receiver.deactivate(),
            sender,
        };

        this.attach();

        this
    }

    #[inline]
    fn on_bytes(&self) -> impl Fn(ByteChange) + 'static {
        let applied_queue = self.applied_queue.clone();
        let buffer = self.inner.clone();
        let sender = self.sender.clone();

        move |change| {
            // This should never happen, but check just in case.
            if change.is_no_op() {
                return;
            }

            let mut buffer = buffer.borrow_mut();

            let caused_by_applied = applied_queue.with_first(|applied| {
                let Some(applied) = applied else { return false };
                buffer.applied_caused_change(applied, &change)
            });

            // If the change was caused by an edit we already applied we
            // mustn't apply it again. We instead pop the applied edit from
            // the queue.
            if caused_by_applied {
                let applied = applied_queue.pop_front().expect("just checked");
                broadcast_edit(&sender, applied);
            }
            // The change was either caused by the user or by another plugin,
            // so we apply it to our buffer to keep it in sync.
            else {
                let (del, ins) = buffer.apply_byte_change(change);

                if let Some(deletion) = del {
                    broadcast_edit(&sender, AppliedEdit::Deletion(deletion));
                }

                if let Some(insertion) = ins {
                    broadcast_edit(&sender, AppliedEdit::Insertion(insertion));
                }
            }
        }
    }

    /// TODO: docs
    #[inline]
    pub fn snapshot(&self) -> BufferSnapshot {
        todo!();
    }
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

/// TODO: docs
struct BufferInner {
    /// TODO: docs
    crdt: Replica,

    /// TODO: docs
    nvim: NvimBuffer,

    /// TODO: docs
    text: Rope,
}

impl BufferInner {
    /// TODO: docs
    #[inline]
    fn apply_byte_change(
        &mut self,
        change: ByteChange,
    ) -> (Option<AppliedDeletion>, Option<AppliedInsertion>) {
        debug_assert!(!change.is_no_op());

        let byte_range = change.byte_range();

        self.text.replace(byte_range.clone(), &change.replacement);

        let mut deletion = None;

        let mut insertion = None;

        if !byte_range.is_empty() {
            let del = self.crdt.deleted(byte_range.clone());
            deletion = Some(AppliedDeletion::new(del));
        }

        let text_len = change.replacement.len();

        if text_len > 0 {
            let ins = self.crdt.inserted(byte_range.start, text_len);
            insertion = Some(AppliedInsertion::new(ins, change.replacement));
        }

        (deletion, insertion)
    }

    /// TODO: docs
    ///
    /// # Panics
    ///
    /// Panics if either the start or end anchor cannot be resolved to a byte
    /// offset in the buffer.
    #[track_caller]
    #[inline]
    fn apply_local_deletion(
        &mut self,
        delete_range: Range<Anchor>,
    ) -> Option<AppliedDeletion> {
        let Some(start_offset) = self.crdt.resolve_anchor(delete_range.start)
        else {
            panic_couldnt_resolve_anchor(&delete_range.start);
        };

        let Some(end_offset) = self.crdt.resolve_anchor(delete_range.end)
        else {
            panic_couldnt_resolve_anchor(&delete_range.end);
        };

        if start_offset == end_offset {
            return None;
        }

        assert!(start_offset < end_offset);

        let delete_range = start_offset..end_offset;

        self.nvim_delete(delete_range.clone());

        self.text.delete(delete_range.clone());

        let deletion = self.crdt.deleted(delete_range);

        Some(AppliedDeletion::new(deletion))
    }

    /// TODO: docs
    ///
    /// # Panics
    ///
    /// Panics if the anchor cannot be resolved to a byte offset in the buffer.
    #[track_caller]
    #[inline]
    fn apply_local_insertion(
        &mut self,
        insert_at: Anchor,
        text: String,
    ) -> AppliedInsertion {
        let Some(byte_offset) = self.crdt.resolve_anchor(insert_at) else {
            panic_couldnt_resolve_anchor(&insert_at);
        };

        self.nvim_insert(byte_offset, &text);

        self.text.insert(byte_offset, &text);

        let insertion = self.crdt.inserted(byte_offset, text.len());

        AppliedInsertion::new(insertion, text)
    }

    /// TODO: docs
    #[inline]
    fn apply_remote_deletion(&mut self, deletion: &RemoteDeletion) {
        let delete_ranges = self.crdt.integrate_deletion(&deletion.inner);

        for range in delete_ranges.into_iter().rev() {
            self.nvim_delete(range.clone());
            self.text.delete(range);
        }
    }

    /// TODO: docs
    #[inline]
    fn apply_remote_insertion(&mut self, insertion: &RemoteInsertion) {
        let Some(byte_offset) =
            self.crdt.integrate_insertion(&insertion.inner)
        else {
            return;
        };

        self.nvim_insert(byte_offset, &insertion.text);
        self.text.insert(byte_offset, &insertion.text);
    }

    /// TODO: docs
    #[inline]
    fn applied_caused_change(
        &self,
        applied: &AppliedEdit,
        change: &ByteChange,
    ) -> bool {
        let byte_range = change.byte_range();

        match (byte_range.is_empty(), change.replacement.is_empty()) {
            // Insertion
            (true, false) => {
                let AppliedEdit::Insertion(insertion) = &applied else {
                    return false;
                };

                self.applied_insertion_caused_change(
                    insertion,
                    byte_range.start,
                    &change.replacement,
                )
            },

            // Deletion
            (false, true) => {
                let AppliedEdit::Deletion(deletion) = &applied else {
                    return false;
                };

                self.applied_deletion_caused_change(deletion, byte_range)
            },

            // Replacement or no-op
            _ => false,
        }
    }

    /// TODO: docs
    #[inline]
    fn applied_deletion_caused_change(
        &self,
        deletion: &AppliedDeletion,
        byte_range: Range<ByteOffset>,
    ) -> bool {
        #[inline(never)]
        fn unreachable_applied() -> ! {
            unreachable!(
                "the deletion was applied, so its start and end anchors can \
                 be resolved"
            );
        }

        let Some(deletion_start) = self.crdt.resolve_anchor(deletion.start())
        else {
            unreachable_applied();
        };

        if deletion_start != byte_range.start {
            return false;
        }

        // TODO: compare deletion's length to byte range's.

        true
    }

    /// TODO: docs
    #[inline]
    fn applied_insertion_caused_change(
        &self,
        insertion: &AppliedInsertion,
        byte_offset: ByteOffset,
        text: &str,
    ) -> bool {
        #[inline(never)]
        fn unreachable_applied() -> ! {
            unreachable!(
                "the insertion was applied, so its anchor can be resolved"
            );
        }

        if insertion.text().len() != text.len() {
            return false;
        }

        let Some(insertion_offset) =
            self.crdt.resolve_anchor(insertion.anchor())
        else {
            unreachable_applied();
        };

        if insertion_offset != byte_offset {
            return false;
        }

        let compare_until_threshold = 16;

        // If we get here we know that both the anchor and the text length
        // match, so it's very likely that the text is the same.
        //
        // Still, for texts up to a length threshold we actually perform the
        // comparison just to be sure.
        if text.len() < compare_until_threshold {
            insertion.text() == text
        }
        // For larger texts we only compare the first `threshold` bytes because
        // comparing the whole texts gets too expensive considering this blocks
        // after every edit.
        else {
            insertion.text().as_bytes()[..compare_until_threshold]
                == text.as_bytes()[..compare_until_threshold]
        }
    }

    #[inline]
    fn new(id: BufferId) -> Self {
        let nvim = id.into();
        let text = rope_from_buf(&nvim).expect("buffer must exist");
        let crdt = Replica::new(1, text.byte_len());
        Self { crdt, nvim, text }
    }

    /// TODO: docs
    #[inline]
    fn nvim_delete(&mut self, delete_range: Range<ByteOffset>) {
        let start_point = self.point_of_offset(delete_range.start);

        let end_point = self.point_of_offset(delete_range.end);

        // This can fail if the buffer has been unloaded.
        let _ = self.nvim.set_text(
            start_point.row..end_point.row,
            start_point.col,
            end_point.col,
            iter::empty::<nvim::String>(),
        );
    }

    /// TODO: docs
    #[inline]
    fn nvim_insert(&mut self, insert_at: ByteOffset, text: &str) {
        let point = self.point_of_offset(insert_at);

        // This can fail if the buffer has been unloaded.
        let _ = self.nvim.set_text(
            point.row..point.row,
            point.col,
            point.col,
            iter::once(text),
        );
    }

    /// Transforms the 1-dimensional byte offset into a 2-dimensional
    /// [`Point`].
    #[inline]
    fn point_of_offset(&self, byte_offset: ByteOffset) -> Point {
        let row = self.text.line_of_byte(byte_offset);
        let row_offset = self.text.byte_of_line(row);
        let col = byte_offset - row_offset;
        Point { row, col }
    }
}

/// TODO: docs
#[inline]
fn rope_from_buf(buf: &NvimBuffer) -> Result<Rope, nvim::api::Error> {
    let num_lines = buf.line_count()?;

    let has_trailine_newline = {
        let mut last_line = buf.get_lines(num_lines - 1..num_lines, true)?;

        let last_line_len =
            buf.get_offset(num_lines)? - buf.get_offset(num_lines - 1)?;

        if let Some(last_line) = last_line.next() {
            last_line_len > last_line.len()
        } else {
            false
        }
    };

    let lines = buf.get_lines(0..num_lines, true)?;

    let mut builder = RopeBuilder::new();

    for (idx, line) in lines.enumerate() {
        builder.append(line.to_string_lossy());
        let is_last = idx + 1 == num_lines;
        let should_append_newline = !is_last | has_trailine_newline;
        if should_append_newline {
            builder.append("\n");
        }
    }

    Ok(builder.build())
}

#[inline(never)]
fn panic_couldnt_resolve_anchor(anchor: &Anchor) -> ! {
    panic!("{anchor:?} couldn't be resolved");
}

/// TODO: docs
#[derive(Debug, PartialEq, Eq)]
struct Point {
    /// TODO: docs
    row: usize,

    /// TODO: docs
    col: ByteOffset,
}

#[derive(Clone)]
struct AppliedEditQueue {
    inner: Rc<RefCell<VecDeque<AppliedEdit>>>,
}

impl AppliedEditQueue {
    #[inline]
    fn new() -> Self {
        Self { inner: Rc::new(RefCell::new(VecDeque::new())) }
    }

    #[inline]
    fn pop_front(&self) -> Option<AppliedEdit> {
        self.inner.borrow_mut().pop_front()
    }

    #[inline]
    fn push_back(&self, edit: AppliedEdit) {
        self.inner.borrow_mut().push_back(edit);
    }

    #[inline]
    fn with_first<F, R>(&self, fun: F) -> R
    where
        F: FnOnce(Option<&AppliedEdit>) -> R,
    {
        let inner = self.inner.borrow();
        let first = inner.front();
        fun(first)
    }
}

/// TODO: docs
pub struct RemoteInsertion {
    inner: cola::Insertion,
    text: String,
}

impl RemoteInsertion {
    /// TODO: docs
    #[inline]
    pub fn new(inner: cola::Insertion, text: String) -> Self {
        Self { inner, text }
    }
}

impl From<RemoteInsertion> for AppliedInsertion {
    #[inline]
    fn from(insertion: RemoteInsertion) -> Self {
        AppliedInsertion::new(insertion.inner, insertion.text)
    }
}

/// TODO: docs
pub struct RemoteDeletion {
    inner: cola::Deletion,
}

impl RemoteDeletion {
    /// TODO: docs
    #[inline]
    pub fn new(inner: cola::Deletion) -> Self {
        Self { inner }
    }
}

impl From<RemoteDeletion> for AppliedDeletion {
    #[inline]
    fn from(deletion: RemoteDeletion) -> Self {
        AppliedDeletion::new(deletion.inner)
    }
}

type ByteOffset = usize;

/// TODO: docs
struct ByteChange {
    start: ByteOffset,
    end: ByteOffset,
    replacement: String,
}

impl ByteChange {
    #[inline]
    fn byte_range(&self) -> Range<usize> {
        self.start..self.end
    }

    #[inline]
    fn is_no_op(&self) -> bool {
        self.start == self.end && self.replacement.is_empty()
    }
}

impl From<opts::OnBytesArgs> for ByteChange {
    #[inline]
    fn from(
        (
            _bytes,
            buf,
            _changedtick,
            start_row,
            start_col,
            start_offset,
            _old_end_row,
            _old_end_col,
            old_end_len,
            new_end_row,
            new_end_col,
            _new_end_len,
        ): opts::OnBytesArgs,
    ) -> Self {
        let replacement_start = Point { row: start_row, col: start_col };

        let replacement_end = Point {
            row: start_row + new_end_row,
            col: start_col * (new_end_row == 0) as usize + new_end_col,
        };

        let replacement = if replacement_start == replacement_end {
            String::new()
        } else {
            nvim_buf_get_text(&buf, replacement_start..replacement_end)
                .expect("buffer must exist")
        };

        Self {
            start: start_offset,
            end: start_offset + old_end_len,
            replacement,
        }
    }
}

/// TODO: docs
#[inline]
fn nvim_buf_get_text(
    buf: &NvimBuffer,
    point_range: Range<Point>,
) -> Result<String, nvim::api::Error> {
    let mut lines = buf.get_text(
        point_range.start.row..point_range.end.row,
        point_range.start.col,
        point_range.end.col,
        &Default::default(),
    )?;

    let mut text = String::new();

    let Some(first_line) = lines.next() else {
        return Ok(text);
    };

    text.push_str(&first_line.to_string_lossy());

    for line in lines {
        text.push('\n');
        text.push_str(&line.to_string_lossy());
    }

    Ok(text)
}
