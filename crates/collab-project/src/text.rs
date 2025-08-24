//! TODO: docs.

use core::cmp::Ordering;
use core::ops::{Deref, DerefMut, Range};

use collab_types::annotation::AnnotationId;
use collab_types::text::{
    Cursor,
    CursorCreation,
    CursorMove,
    CursorRemoval,
    Selection,
    SelectionCreation,
    SelectionMove,
    SelectionRemoval,
    TextEdit,
};
use collab_types::{Counter, PeerId, cola, crop, puff};
use crop::Rope;
use fxhash::FxHashMap;
use nohash::IntMap as NoHashMap;
use puff::file::{GlobalFileId, LocalFileId};
use puff::node::{Backlogged, Deleted, Editable, IsVisible, Visible};
use smallvec::SmallVec;
use smol_str::{SmolStr, SmolStrBuilder};

use crate::Project;
use crate::abs_path::AbsPathBuf;
use crate::annotation::{
    self,
    Annotation,
    AnnotationMut,
    AnnotationRef,
    Annotations,
    AnnotationsIter,
};
use crate::fs::{
    FileContents,
    FileMut,
    PuffFile,
    PuffFileMut,
    PuffFileState,
    PuffFileStateMut,
};
use crate::project::{State, StateMut};

/// TODO: docs.
pub type ByteOffset = usize;

/// TODO: docs.
pub struct TextFile<'a, S = Visible> {
    inner: PuffFile<'a, S>,
    state: State<'a>,
}

/// TODO: docs.
pub struct TextFileMut<'a, S = Editable> {
    inner: PuffFileMut<'a, S>,
    state: StateMut<'a>,
}

/// TODO: docs.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CursorId {
    inner: AnnotationId,
}

/// TODO: docs.
pub struct CursorRef<'a> {
    file: PuffFileState<'a>,
    id: CursorId,
    offset: ByteOffset,
    state: State<'a>,
}

/// TODO: docs.
pub struct CursorMut<'a> {
    id: CursorId,
    proj: &'a mut Project,
}

/// TODO: docs.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SelectionId {
    inner: AnnotationId,
}

/// TODO: docs.
pub struct SelectionRef<'a> {
    file: PuffFileState<'a>,
    id: SelectionId,
    offset_range: Range<ByteOffset>,
    state: State<'a>,
}

/// TODO: docs.
pub struct SelectionMut<'a> {
    id: SelectionId,
    proj: &'a mut Project,
}

/// TODO: docs.
pub struct TextReplacement {
    /// TODO: docs.
    pub deleted_range: Range<ByteOffset>,

    /// TODO: docs.
    pub inserted_text: SmolStr,
}

/// TODO: docs.
pub struct TextReplacements {
    inner: smallvec::IntoIter<[TextReplacement; 1]>,
}

/// TODO: docs.
pub struct Cursors<'a> {
    inner: AnnotationsIter<'a, Cursor>,
    proj: &'a Project,
}

/// TODO: docs.
pub struct TextFileCursors<'a, S = Visible> {
    inner: AnnotationsIter<'a, Cursor>,
    file: TextFile<'a, S>,
}

/// TODO: docs.
pub struct Selections<'a> {
    inner: AnnotationsIter<'a, Selection>,
    proj: &'a Project,
}

/// TODO: docs.
pub struct TextFileSelections<'a, S = Visible> {
    inner: AnnotationsIter<'a, Selection>,
    file: TextFile<'a, S>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TextCtx {
    pub(crate) cursors: Annotations<Cursor>,
    pub(crate) selections: Annotations<Selection>,
}

/// TODO: docs.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TextContents {
    crdt: TextCrdt,
    text: crop::Rope,
    text_backlog: TextBacklog,
}

/// TODO: docs.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub(crate) struct TextEditBacklog {
    /// Map from a backlogged file's global ID to the list of text edits
    /// received for that file in receival order.
    edits: FxHashMap<GlobalFileId, Vec<TextEdit>>,
}

/// The state of a text file.
pub(crate) enum TextStateMut<'a> {
    Visible(TextFileMut<'a, Editable>),
    Backlogged(TextFileMut<'a, Backlogged>),
    Deleted(TextFileMut<'a, Deleted>),
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
enum TextCrdt {
    #[cfg(feature = "serde")]
    Encoded(cola::EncodedReplica<'static>),
    Ready(cola::Replica),
}

/// TODO: docs.
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct TextBacklog {
    /// Map from peer ID to a list of `(temporal_offset, insertion)` pairs for
    /// that peer, ordered by increasing temporal offset.
    insertions: NoHashMap<cola::ReplicaId, Vec<(usize, SmolStr)>>,
}

impl<'a, S> TextFile<'a, S> {
    /// Returns a `Rope` containing the text file's contents.
    #[inline]
    pub fn contents(&self) -> &'a Rope {
        &self.text_contents().text
    }

    /// Returns an iterator over the cursors in this text file.
    #[inline]
    pub fn cursors(&self) -> TextFileCursors<'a, S> {
        TextFileCursors {
            inner: self.state.text_ctx().cursors.iter(),
            file: *self,
        }
    }

    /// Returns this text file's global ID.
    #[inline]
    pub fn global_id(&self) -> GlobalFileId {
        self.inner.global_id()
    }

    /// Returns this text file's ID.
    #[inline]
    pub fn id(&self) -> LocalFileId {
        self.inner.local_id()
    }

    /// Returns an iterator over the selections in this text file.
    #[inline]
    pub fn selections(&self) -> TextFileSelections<'a, S> {
        TextFileSelections {
            inner: self.state.text_ctx().selections.iter(),
            file: *self,
        }
    }

    #[inline]
    pub(crate) fn inner(&self) -> PuffFile<'a, S> {
        self.inner
    }

    #[track_caller]
    #[inline]
    pub(crate) fn new(inner: PuffFile<'a, S>, state: State<'a>) -> Self {
        debug_assert!(inner.metadata().is_text());
        Self { inner, state }
    }

    #[inline]
    pub(crate) fn state(&self) -> State<'a> {
        self.state
    }

    #[inline]
    fn text_contents(&self) -> &'a TextContents {
        match self.inner.metadata() {
            FileContents::Text(text_contents) => text_contents,
            _ => unreachable!(),
        }
    }
}

impl<'a, S: IsVisible> TextFile<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn path(&self) -> AbsPathBuf {
        self.inner.path()
    }
}

impl<'a, S> TextFileMut<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn as_file(&self) -> TextFile<'_, S> {
        TextFile { inner: self.inner.as_file(), state: self.state.as_ref() }
    }

    #[inline]
    pub(crate) fn inner_mut(&mut self) -> &mut PuffFileMut<'a, S> {
        &mut self.inner
    }

    #[inline]
    pub(crate) fn integrate_edit(
        &mut self,
        edit: TextEdit,
    ) -> TextReplacements {
        debug_assert_eq!(edit.file_id, self.inner.global_id());
        self.contents_mut().integrate_edit(edit)
    }

    #[inline]
    pub(crate) fn into_inner(self) -> PuffFileMut<'a, S> {
        self.inner
    }

    #[track_caller]
    #[inline]
    pub(crate) fn new(inner: PuffFileMut<'a, S>, state: StateMut<'a>) -> Self {
        debug_assert!(inner.metadata().is_text());
        Self { inner, state }
    }

    #[inline]
    fn contents_mut(&mut self) -> &mut TextContents {
        match self.inner.metadata_mut() {
            FileContents::Text(text_contents) => text_contents,
            _ => unreachable!(),
        }
    }
}

impl<'a, S: IsVisible> TextFileMut<'a, S> {
    /// TODO: docs.
    #[inline]
    pub fn create_cursor(
        &mut self,
        offset: ByteOffset,
    ) -> (CursorId, CursorCreation) {
        let local_id = self.state.local_id();
        let cursor = Cursor {
            anchor: self.contents_mut().create_cursor(offset),
            sequence_num: Counter::new(0),
        };
        let (annotation, creation) = self.state.text_ctx_mut().cursors.create(
            local_id,
            self.inner.as_file(),
            cursor,
        );
        (annotation.id().into(), creation)
    }

    /// TODO: docs.
    #[inline]
    pub fn create_selection(
        &mut self,
        offset_range: Range<ByteOffset>,
    ) -> (SelectionId, SelectionCreation) {
        let local_id = self.state.local_id();
        let anchor_range = self.contents_mut().create_selection(offset_range);
        let selection = Selection {
            start: anchor_range.start,
            end: anchor_range.end,
            sequence_num: Counter::new(0),
        };
        let (annotation, creation) = self
            .state
            .text_ctx_mut()
            .selections
            .create(local_id, self.inner.as_file(), selection);
        (SelectionId { inner: annotation.id() }, creation)
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn edit<R>(&mut self, replacements: R) -> TextEdit
    where
        R: IntoIterator<Item = TextReplacement>,
    {
        let file_id = self.inner.global_id();
        self.contents_mut().edit(replacements, file_id)
    }
}

impl CursorId {
    /// TODO: docs.
    #[inline]
    pub fn owner(&self) -> PeerId {
        self.inner.created_by
    }
}

impl<'a> CursorRef<'a> {
    /// TODO: docs.
    #[inline]
    pub fn file(&self) -> Option<TextFile<'a>> {
        match self.file {
            PuffFileState::Visible(file) => {
                Some(TextFile::new(file, self.state))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn id(&self) -> CursorId {
        self.id
    }

    /// TODO: docs.
    #[inline]
    pub fn offset(&self) -> ByteOffset {
        self.offset
    }

    /// TODO: docs.
    #[inline]
    pub fn owner(&self) -> PeerId {
        self.id.owner()
    }

    #[inline]
    pub(crate) fn from_id(id: CursorId, proj: &'a Project) -> Option<Self> {
        let cursor = proj.state().text_ctx().cursors.get(id.inner)?;

        let file = proj.fs().file(cursor.file_id());

        let FileContents::Text(contents) = file.metadata() else {
            unreachable!("cursors can only be created on TextFiles");
        };

        Some(Self {
            id: cursor.id().into(),
            offset: contents.resolve_cursor(cursor.data())?,
            file,
            state: proj.state(),
        })
    }
}

impl<'a> CursorMut<'a> {
    /// TODO: docs.
    #[inline]
    pub fn delete(mut self) -> CursorRemoval {
        self.annotation_mut().delete()
    }

    /// TODO: docs.
    #[inline]
    pub fn file_mut(&mut self) -> Option<TextFileMut<'_>> {
        let file_id = self.annotation().file_id();
        let (state, fs) = self.proj.state_mut();
        match fs.file_mut(file_id) {
            PuffFileStateMut::Visible(file) => {
                Some(TextFileMut::new(file, state))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn r#move(&mut self, new_offset: ByteOffset) -> CursorMove {
        let file_id = self.annotation().file_id();
        let file_state = self.proj.fs_mut().file(file_id);

        let FileContents::Text(contents) = file_state.metadata() else {
            unreachable!("cursors can only be created on TextFiles");
        };

        let new_anchor = contents.create_cursor(new_offset);

        self.annotation_mut().update(|cursor| {
            cursor.anchor = new_anchor;
            cursor.sequence_num.post_increment();
            *cursor
        })
    }

    #[inline]
    pub(crate) fn from_id(
        id: CursorId,
        proj: &'a mut Project,
    ) -> Option<Self> {
        CursorRef::from_id(id, proj).is_some().then_some(Self { id, proj })
    }

    #[inline]
    fn annotation(&self) -> AnnotationRef<'_, Cursor> {
        self.proj
            .text_ctx()
            .cursors
            .get(self.id.inner)
            .expect("CursorId is valid")
    }

    #[inline]
    fn annotation_mut(&mut self) -> AnnotationMut<'_, Cursor> {
        self.proj
            .text_ctx_mut()
            .cursors
            .get_mut(self.id.inner)
            .expect("CursorId is valid")
    }
}

impl SelectionId {
    /// TODO: docs.
    #[inline]
    pub fn owner(&self) -> PeerId {
        self.inner.created_by
    }
}

impl<'a> SelectionRef<'a> {
    /// TODO: docs.
    #[inline]
    pub fn file(&self) -> Option<TextFile<'a>> {
        match self.file {
            PuffFileState::Visible(file) => {
                Some(TextFile::new(file, self.state))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn id(&self) -> SelectionId {
        self.id
    }

    /// TODO: docs.
    #[inline]
    pub fn offset_range(&self) -> Range<ByteOffset> {
        self.offset_range.clone()
    }

    /// TODO: docs.
    #[inline]
    pub fn owner(&self) -> PeerId {
        self.id.owner()
    }

    #[inline]
    pub(crate) fn from_id(id: SelectionId, proj: &'a Project) -> Option<Self> {
        let selection = proj.state().text_ctx().selections.get(id.inner)?;

        let file = proj.fs().file(selection.file_id());

        let FileContents::Text(contents) = file.metadata() else {
            unreachable!("selections can only be created on TextFiles");
        };

        Some(Self {
            id: selection.id().into(),
            offset_range: contents.resolve_selection(selection.data())?,
            state: proj.state(),
            file,
        })
    }
}

impl<'a> SelectionMut<'a> {
    /// TODO: docs.
    #[inline]
    pub fn delete(mut self) -> SelectionRemoval {
        self.annotation_mut().delete()
    }

    /// TODO: docs.
    #[inline]
    pub fn file_mut(&mut self) -> Option<TextFileMut<'_>> {
        let file_id = self.annotation().file_id();
        let (state, fs) = self.proj.state_mut();
        match fs.file_mut(file_id) {
            PuffFileStateMut::Visible(file) => {
                Some(TextFileMut::new(file, state))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn r#move(&mut self, new_range: Range<ByteOffset>) -> SelectionMove {
        let file_state = self.proj.fs().file(self.annotation().file_id());

        let FileContents::Text(contents) = file_state.metadata() else {
            unreachable!("selections can only be created on TextFiles");
        };

        let new_anchor_range = contents.create_selection(new_range);

        self.annotation_mut().update(|selection| {
            selection.start = new_anchor_range.start;
            selection.end = new_anchor_range.end;
            selection.sequence_num.post_increment();
            *selection
        })
    }

    #[inline]
    pub(crate) fn from_id(
        id: SelectionId,
        proj: &'a mut Project,
    ) -> Option<Self> {
        SelectionRef::from_id(id, proj).is_some().then_some(Self { id, proj })
    }

    #[inline]
    fn annotation_mut(&mut self) -> AnnotationMut<'_, Selection> {
        self.proj
            .text_ctx_mut()
            .selections
            .get_mut(self.id.inner)
            .expect("SelectionId is valid")
    }

    #[inline]
    fn annotation(&self) -> AnnotationRef<'_, Selection> {
        self.proj
            .text_ctx()
            .selections
            .get(self.id.inner)
            .expect("SelectionId is valid")
    }
}

impl<'a> Cursors<'a> {
    #[inline]
    pub(crate) fn new(project: &'a Project) -> Self {
        Self { inner: project.text_ctx().cursors.iter(), proj: project }
    }
}

impl<'a> Selections<'a> {
    #[inline]
    pub(crate) fn new(project: &'a Project) -> Self {
        Self { inner: project.text_ctx().selections.iter(), proj: project }
    }
}

impl TextContents {
    #[inline]
    pub(crate) fn decode(&mut self, local_id: PeerId) {
        let local_id = local_id.into_u64();

        match &mut self.crdt {
            #[cfg(feature = "serde")]
            TextCrdt::Encoded(encoded) => {
                match cola::Replica::decode(local_id, encoded) {
                    Ok(decoded) => self.crdt = TextCrdt::Ready(decoded),
                    Err(err) => panic!("decoding failed: {err}"),
                }
            },
            TextCrdt::Ready(replica) => {
                if replica.id() != local_id {
                    *replica = replica.fork(local_id);
                }
            },
        }
    }

    #[inline]
    pub(crate) fn integrate_edit(
        &mut self,
        edit: TextEdit,
    ) -> TextReplacements {
        let mut replacements = SmallVec::new();

        for (insertion, text) in edit.insertions {
            let Some(byte_offset) = self.crdt.integrate_insertion(&insertion)
            else {
                self.text_backlog.insert(insertion.text().clone(), text);
                continue;
            };
            self.text.insert(byte_offset, &*text);
            replacements.push(TextReplacement {
                deleted_range: byte_offset..byte_offset,
                inserted_text: text,
            });
        }

        for (text, byte_offset) in self.crdt.backlogged_insertions() {
            let text = self.text_backlog.take(text);
            self.text.insert(byte_offset, &*text);
            replacements.push(TextReplacement {
                deleted_range: byte_offset..byte_offset,
                inserted_text: text,
            });
        }

        for byte_ranges in self.crdt.backlogged_deletions() {
            for deleted_range in byte_ranges.into_iter().rev() {
                self.text.delete(deleted_range.clone());
                replacements.push(TextReplacement {
                    deleted_range,
                    inserted_text: SmolStr::default(),
                });
            }
        }

        for deletion in edit.deletions {
            for deleted_range in
                self.crdt.integrate_deletion(&deletion).into_iter().rev()
            {
                self.text.delete(deleted_range.clone());
                replacements.push(TextReplacement {
                    deleted_range,
                    inserted_text: SmolStr::default(),
                });
            }
        }

        TextReplacements { inner: replacements.into_iter() }
    }

    #[inline]
    pub(crate) fn new(local_id: PeerId, text: crop::Rope) -> Self {
        Self {
            crdt: TextCrdt::Ready(cola::Replica::new(
                local_id.into_u64(),
                text.byte_len(),
            )),
            text,
            text_backlog: TextBacklog::default(),
        }
    }

    #[inline]
    fn create_cursor(&self, offset: ByteOffset) -> cola::Anchor {
        self.crdt.create_anchor(offset, cola::AnchorBias::Left)
    }

    #[inline]
    fn create_selection(
        &self,
        offset_range: Range<ByteOffset>,
    ) -> Range<cola::Anchor> {
        let byte_range = match offset_range.start.cmp(&offset_range.end) {
            Ordering::Less | Ordering::Equal => offset_range,
            Ordering::Greater => offset_range.start..offset_range.end,
        };

        let anchor_start =
            self.crdt.create_anchor(byte_range.start, cola::AnchorBias::Right);

        let anchor_end =
            self.crdt.create_anchor(byte_range.end, cola::AnchorBias::Left);

        anchor_start..anchor_end
    }

    #[track_caller]
    #[inline]
    fn edit<R>(&mut self, replacements: R, file_id: GlobalFileId) -> TextEdit
    where
        R: IntoIterator<Item = TextReplacement>,
    {
        let mut deletions = SmallVec::new();
        let mut insertions = SmallVec::new();

        for TextReplacement { deleted_range, inserted_text } in replacements {
            let start = deleted_range.start;
            let end = deleted_range.end;
            match start.cmp(&end) {
                Ordering::Less => {
                    self.text.delete(start..end);
                    let deletion = self.crdt.deleted(start..end);
                    deletions.push(deletion);
                },
                Ordering::Equal => {},
                Ordering::Greater => panic!(),
            }

            if !inserted_text.is_empty() {
                self.text.insert(start, &*inserted_text);
                let insertion = self.crdt.inserted(start, inserted_text.len());
                insertions.push((insertion, inserted_text));
            }
        }

        TextEdit { file_id, deletions, insertions }
    }

    #[inline]
    fn resolve_cursor(&self, cursor: &Cursor) -> Option<ByteOffset> {
        self.crdt.resolve_anchor(cursor.anchor)
    }

    #[inline]
    fn resolve_selection(
        &self,
        selection: &Selection,
    ) -> Option<Range<ByteOffset>> {
        let start = self.crdt.resolve_anchor(selection.start)?;
        let end = self.crdt.resolve_anchor(selection.end)?;
        Some(start..end)
    }
}

impl TextEditBacklog {
    #[inline]
    pub(crate) fn insert(&mut self, edit: TextEdit) {
        self.edits.entry(edit.file_id).or_default().push(edit);
    }

    #[inline]
    pub(crate) fn take(&mut self, file_id: GlobalFileId) -> Vec<TextEdit> {
        self.edits.remove(&file_id).unwrap_or_default()
    }
}

impl<'a> TextStateMut<'a> {
    #[inline]
    pub(crate) fn integrate_edit(
        &mut self,
        edit: TextEdit,
    ) -> TextReplacements {
        match self {
            Self::Visible(file) => file.integrate_edit(edit),
            Self::Backlogged(file) => file.integrate_edit(edit),
            Self::Deleted(file) => file.integrate_edit(edit),
        }
    }

    #[inline]
    pub(crate) fn new(
        file_state: PuffFileStateMut<'a>,
        state: StateMut<'a>,
    ) -> Option<Self> {
        match file_state {
            PuffFileStateMut::Visible(file) => {
                match FileMut::new(file, state) {
                    FileMut::Text(file) => Some(Self::Visible(file)),
                    _ => None,
                }
            },
            PuffFileStateMut::Backlogged(file) => {
                match FileMut::new(file, state) {
                    FileMut::Text(file) => Some(Self::Backlogged(file)),
                    _ => None,
                }
            },
            PuffFileStateMut::Deleted(file) => {
                match FileMut::new(file, state) {
                    FileMut::Text(file) => Some(Self::Deleted(file)),
                    _ => None,
                }
            },
        }
    }
}

impl TextBacklog {
    #[track_caller]
    #[inline]
    pub(crate) fn insert(&mut self, text: cola::Text, insertion: SmolStr) {
        let text_range = text.temporal_range();
        debug_assert_eq!(text_range.len(), insertion.len());
        let temporal_offset = text_range.start;

        let ranges = self.insertions.entry(text.inserted_by()).or_default();

        let Err(insert_idx) = ranges
            .binary_search_by_key(&temporal_offset, |&(offset, _)| offset)
        else {
            unreachable!(
                "TextInsertion at offset {} already exists",
                temporal_offset
            );
        };

        // Check that there's no overlap with the previous item.
        debug_assert!(
            insert_idx
                .checked_sub(1)
                .and_then(|idx| ranges.get(idx))
                .map(|(prev_offset, prev_insertion)| {
                    prev_offset + prev_insertion.len() <= temporal_offset
                })
                .unwrap_or(true),
        );

        // Check that there's no overlap with the next item.
        debug_assert!(
            ranges
                .get(insert_idx + 1)
                .map(|(next_offset, _)| {
                    temporal_offset + insertion.len() <= *next_offset
                })
                .unwrap_or(true),
        );

        ranges.insert(insert_idx, (temporal_offset, insertion));
    }

    #[track_caller]
    #[inline]
    pub(crate) fn take(&mut self, text: cola::Text) -> SmolStr {
        let Some(ranges) = self.insertions.get_mut(&text.inserted_by()) else {
            unreachable!(
                "there's no backlogged insertion for peer {}",
                text.inserted_by()
            );
        };

        let Ok(drain_from) = ranges.binary_search_by_key(
            &text.temporal_range().start,
            |&(offset, _)| offset,
        ) else {
            unreachable!(
                "there's no backlogged insertion from peer {} starting at \
                 offset {}",
                text.inserted_by(),
                text.temporal_range().start
            )
        };

        let drain_to = match ranges[drain_from..].binary_search_by_key(
            &text.temporal_range().end,
            |(offset, insertion)| offset + insertion.len(),
        ) {
            Ok(idx) => drain_from + idx + 1,
            Err(_) => unreachable!(
                "there's no backlogged insertion from peer {} ending at \
                 offset {}",
                text.inserted_by(),
                text.temporal_range().end
            ),
        };

        let mut insertion = SmolStrBuilder::new();

        for (_, text) in ranges.drain(drain_from..drain_to) {
            insertion.push_str(&text);
        }

        let insertion = insertion.finish();

        if insertion.len() < text.temporal_range().len() {
            unreachable!(
                "there are missing insertions from peer {} in the {:?} \
                 offset range",
                text.inserted_by(),
                text.temporal_range()
            );
        }

        insertion
    }
}

impl<'a, S> Copy for TextFile<'a, S> {}

impl<'a, S> Clone for TextFile<'a, S> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl From<AnnotationId> for CursorId {
    #[inline]
    fn from(id: AnnotationId) -> Self {
        Self { inner: id }
    }
}

impl From<CursorId> for AnnotationId {
    #[inline]
    fn from(id: CursorId) -> Self {
        id.inner
    }
}

impl From<AnnotationId> for SelectionId {
    #[inline]
    fn from(id: AnnotationId) -> Self {
        Self { inner: id }
    }
}

impl From<SelectionId> for AnnotationId {
    #[inline]
    fn from(id: SelectionId) -> Self {
        id.inner
    }
}

impl Iterator for TextReplacements {
    type Item = TextReplacement;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> Iterator for Cursors<'a> {
    type Item = CursorRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let cursor = self.inner.next()?;

        match CursorRef::from_id(cursor.id().into(), self.proj) {
            Some(cursor) => Some(cursor),
            None => self.next(),
        }
    }
}

impl<'a> Iterator for TextFileCursors<'a> {
    type Item = CursorRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let annotation = self.inner.next()?;

        if annotation.file_id() == self.file.id() {
            let Some(offset) =
                self.file.text_contents().resolve_cursor(annotation.data())
            else {
                return self.next();
            };
            Some(CursorRef {
                id: annotation.id().into(),
                file: PuffFileState::Visible(self.file.inner),
                offset,
                state: self.file.state,
            })
        } else {
            self.next()
        }
    }
}

impl<'a> Iterator for Selections<'a> {
    type Item = SelectionRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let selection = self.inner.next()?;

        match SelectionRef::from_id(selection.id().into(), self.proj) {
            Some(selection) => Some(selection),
            None => self.next(),
        }
    }
}

impl<'a> Iterator for TextFileSelections<'a> {
    type Item = SelectionRef<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let annotation = self.inner.next()?;

        if annotation.file_id() == self.file.id() {
            let Some(offset_range) =
                self.file.text_contents().resolve_selection(annotation.data())
            else {
                return self.next();
            };
            Some(SelectionRef {
                id: annotation.id().into(),
                file: PuffFileState::Visible(self.file.inner),
                offset_range,
                state: self.file.state,
            })
        } else {
            self.next()
        }
    }
}

impl Annotation for Cursor {
    type Op = Self;
    type Backlog = Self;
    type IntegrateResult = bool;

    #[inline]
    fn integrate_op(&mut self, other: Self) -> Self::IntegrateResult {
        if self.sequence_num < other.sequence_num {
            *self = other;
            true
        } else {
            false
        }
    }

    #[inline]
    fn integrate_backlog(&mut self, other: Self) -> Self::IntegrateResult {
        self.integrate_op(other)
    }
}

impl annotation::Backlog for Cursor {
    type Annotation = Self;

    #[inline]
    fn insert(&mut self, other: Self) {
        self.integrate_op(other);
    }

    #[inline]
    fn new(other: Self) -> Self {
        other
    }
}

impl Annotation for Selection {
    type Op = Self;
    type Backlog = Self;
    type IntegrateResult = bool;

    #[inline]
    fn integrate_op(&mut self, other: Self) -> Self::IntegrateResult {
        if self.sequence_num < other.sequence_num {
            *self = other;
            true
        } else {
            false
        }
    }

    #[inline]
    fn integrate_backlog(&mut self, other: Self) -> Self::IntegrateResult {
        self.integrate_op(other)
    }
}

impl annotation::Backlog for Selection {
    type Annotation = Self;

    #[inline]
    fn insert(&mut self, other: Self) {
        self.integrate_op(other);
    }

    #[inline]
    fn new(other: Self) -> Self {
        other
    }
}

impl Deref for TextCrdt {
    type Target = cola::Replica;

    #[track_caller]
    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ready(replica) => replica,
            #[cfg(feature = "serde")]
            Self::Encoded(_) => panic!("TextCrdt not yet decoded"),
        }
    }
}

impl DerefMut for TextCrdt {
    #[track_caller]
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Ready(replica) => replica,
            #[cfg(feature = "serde")]
            Self::Encoded(_) => panic!("TextCrdt not yet decoded"),
        }
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Serialize};

    use super::*;

    impl Serialize for TextCrdt {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                Self::Ready(replica) => replica.encode().serialize(serializer),
                Self::Encoded(encoded) => encoded.serialize(serializer),
            }
        }
    }

    impl<'de> Deserialize<'de> for TextCrdt {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            cola::EncodedReplica::deserialize(deserializer).map(Self::Encoded)
        }
    }
}
