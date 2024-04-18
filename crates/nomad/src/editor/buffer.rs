use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::iter;
use core::ops::Range;

use async_broadcast::{InactiveReceiver, Sender};
use cola::{Anchor, Replica, ReplicaId};
use crop::{Rope, RopeBuilder};
use nvim::api::{self, opts, Buffer as NvimBuffer};

use super::{
    BufferId,
    BufferSnapshot,
    BufferState,
    EditorId,
    LocalDeletion,
    LocalInsertion,
};
use crate::runtime::spawn;
use crate::streams::{AppliedDeletion, AppliedEdit, AppliedInsertion, Edits};

/// TODO: docs
pub struct Buffer {
    /// TODO: docs
    applied_queue: AppliedEditQueue,

    /// TODO: docs
    nvim: NvimBuffer,

    /// TODO: docs
    receiver: InactiveReceiver<AppliedEdit>,

    /// TODO: docs
    sender: Sender<AppliedEdit>,

    /// TODO: docs
    state: BufferState,
}

impl Buffer {
    /// TODO: docs
    #[inline]
    pub fn apply_local_deletion(
        &mut self,
        delete_range: Range<Anchor>,
        id: EditorId,
    ) {
        let deletion = LocalDeletion::new(delete_range);
        let maybe_deletion = self.state.edit(&deletion);
        if let Some((deletion, range)) = maybe_deletion {
            self.applied_queue.push_back(AppliedEdit::deletion(deletion, id));
            nvim_delete(&mut self.nvim, range);
        }
    }

    /// TODO: docs
    #[inline]
    pub fn apply_local_insertion(
        &mut self,
        insert_at: Anchor,
        text: String,
        id: EditorId,
    ) {
        let also_text = text.clone();
        let insertion = LocalInsertion::new(insert_at, text);
        let (insertion, point) = self.state.edit(insertion);
        self.applied_queue.push_back(AppliedEdit::insertion(insertion, id));
        nvim_insert(&mut self.nvim, point, &also_text);
    }

    /// TODO: docs
    #[inline]
    pub fn apply_remote_deletion(
        &mut self,
        deletion: RemoteDeletion,
        id: EditorId,
    ) {
        let point_ranges = self.state.edit(&deletion);
        self.applied_queue.push_back(AppliedEdit::deletion(deletion, id));
        for range in point_ranges.into_iter().rev() {
            nvim_delete(&mut self.nvim, range);
        }
    }

    /// TODO: docs
    #[inline]
    pub fn apply_remote_insertion(
        &mut self,
        insertion: RemoteInsertion,
        id: EditorId,
    ) {
        let Some(point) = self.state.edit(&insertion) else { return };
        let text = insertion.text().to_owned();
        self.applied_queue.push_back(AppliedEdit::insertion(insertion, id));
        nvim_insert(&mut self.nvim, point, &text);
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
        let _ = self.nvim.attach(false, &opts);
    }

    /// TODO: docs
    #[inline]
    pub fn create(text: &str, replica: Replica) -> Self {
        let state = BufferState::new(text, replica);

        let Ok(mut buf) = api::create_buf(true, false) else { unreachable!() };

        let Ok(()) = buf.set_lines(.., true, text.lines()) else {
            unreachable!()
        };

        let Ok(()) = api::Window::current().set_buf(&buf) else {
            unreachable!()
        };

        Self::new(state, buf)
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
    pub fn from_id(replica_id: ReplicaId, buffer_id: BufferId) -> Self {
        let buf = NvimBuffer::from(buffer_id);
        let text = rope_from_buf(&buf).expect("buffer must exist");
        let replica = Replica::new(replica_id, text.byte_len());
        Self::new(BufferState::new(text, replica), buffer_id.into())
    }

    #[inline]
    fn new(state: BufferState, bound_to: NvimBuffer) -> Self {
        let (sender, receiver) = async_broadcast::broadcast(32);

        let this = Self {
            applied_queue: AppliedEditQueue::new(),
            nvim: bound_to,
            state,
            receiver: receiver.deactivate(),
            sender,
        };

        this.attach();

        this
    }

    #[inline]
    fn on_bytes(&self) -> impl Fn(ByteChange) + 'static {
        let applied_queue = self.applied_queue.clone();
        let buffer = self.state.clone();
        let sender = self.sender.clone();

        move |change| {
            // If the change was caused by an edit we already applied we
            // mustn't apply it again.
            if let Some(edit) = applied_queue.pop_front() {
                broadcast_edit(&sender, edit);
            }
            // The change was either caused by the user or by another plugin,
            // so we apply it to our buffer to keep it in sync.
            else {
                let (del, ins) = buffer.edit(change);

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
        self.state.snapshot()
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
#[inline]
fn nvim_delete(buf: &mut NvimBuffer, range: Range<Point>) {
    // This can fail if the buffer has been unloaded.
    let _ = buf.set_text(
        range.start.row..range.end.row,
        range.start.col,
        range.end.col,
        iter::empty::<nvim::String>(),
    );
}

/// TODO: docs
#[inline]
fn nvim_insert(buf: &mut NvimBuffer, insert_at: Point, text: &str) {
    // If the text has a trailing newline, the iterator we feed to `set_text`
    // has to yield a final empty line for it to work like we want it to.
    let lines = text.lines().chain(text.ends_with('\n').then_some(""));

    // This can fail if the buffer has been unloaded.
    let _ = buf.set_text(
        insert_at.row..insert_at.row,
        insert_at.col,
        insert_at.col,
        lines,
    );
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

/// TODO: docs
#[derive(Debug, PartialEq, Eq)]
pub struct Point {
    /// TODO: docs
    pub row: usize,

    /// TODO: docs
    pub col: ByteOffset,
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
}

/// TODO: docs
pub struct RemoteInsertion {
    inner: cola::Insertion,
    text: String,
}

impl RemoteInsertion {
    /// TODO: docs
    #[inline]
    pub fn inner(&self) -> &cola::Insertion {
        &self.inner
    }

    /// TODO: docs
    #[inline]
    pub fn new(inner: cola::Insertion, text: String) -> Self {
        Self { inner, text }
    }

    /// TODO: docs
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
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
    pub fn inner(&self) -> &cola::Deletion {
        &self.inner
    }

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

pub type ByteOffset = usize;

/// TODO: docs
pub struct ByteChange {
    pub start: ByteOffset,
    pub end: ByteOffset,
    pub replacement: String,
}

impl ByteChange {
    #[inline]
    pub fn byte_range(&self) -> Range<usize> {
        self.start..self.end
    }

    #[inline]
    pub fn into_text(self) -> String {
        self.replacement
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.replacement
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
