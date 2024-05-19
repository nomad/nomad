use alloc::borrow::Cow;
use alloc::vec::Drain;
use core::cmp::Ordering;
use core::ops::Range;

use compact_str::CompactString;

use crate::{
    Bound,
    Cells,
    Highlight,
    Memoize,
    Metric,
    Point,
    SceneFragment,
    Surface,
};

/// TODO: docs
#[derive(Debug, Default)]
pub(crate) struct Scene {
    /// TODO: docs.
    surface: SceneSurface,

    /// TODO: docs.
    diff_tracker: DiffTracker,
}

impl Scene {
    /// Applies the [`ResizeOp`] to this scene.
    #[inline]
    fn apply(&mut self, resize_op: ResizeOp) {
        resize_op.apply_to(self);
    }

    /// Turns the entire `Scene` into a `SceneFragment` which can be used in
    /// the [`paint`](crate::Render::paint) method of a
    /// [`Render`](crate::Render) implementation.
    #[inline]
    pub(crate) fn as_fragment(&mut self) -> SceneFragment<'_> {
        todo!()
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn diff(&mut self) -> SceneDiff<'_> {
        SceneDiff {
            surface: &self.surface,
            deleted: self.diff_tracker.deleted.take(),
            inserted: self.diff_tracker.inserted.take(),
            truncated: self.diff_tracker.truncated.drain(..),
            extended: self.diff_tracker.extended.drain(..),
            replaced: self.diff_tracker.replaced.drain(..),
            _paint: self.diff_tracker.paint.drain(..),
        }
    }

    /// Returns the height of the scene in terminal [`Cells`].
    #[inline]
    pub(crate) fn height(&self) -> Cells {
        self.surface.height()
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn resize(&mut self, new_size: Bound<Cells>) {
        self.apply(ResizeOp::new(self.size(), new_size));
    }

    /// Returns the size of the scene in terminal [`Cells`].
    #[inline]
    pub(crate) fn size(&self) -> Bound<Cells> {
        Bound::new(self.height(), self.width())
    }

    /// Returns the width of the scene in terminal [`Cells`].
    #[inline]
    pub(crate) fn width(&self) -> Cells {
        self.surface.width()
    }
}

/// TODO: docs.
#[derive(Debug, Default)]
struct SceneSurface {
    /// TODO: docs.
    lines: Vec<SceneLine>,
}

impl SceneSurface {
    #[inline]
    fn height(&self) -> Cells {
        (self.lines.len() as u32).into()
    }

    #[inline]
    fn lines(&self) -> impl Iterator<Item = &SceneLine> + '_ {
        self.lines.iter()
    }

    #[inline]
    fn lines_mut(&mut self) -> impl Iterator<Item = &mut SceneLine> + '_ {
        self.lines.iter_mut()
    }

    #[inline]
    fn run_at(
        &self,
        line_idx: usize,
        run_offset: Cells,
        bias: Bias,
    ) -> &SceneRun {
        let line = &self.lines[line_idx];
        let (run_idx, _) = line.run_at(run_offset, bias);
        &line.runs[run_idx]
    }

    #[inline]
    fn width(&self) -> Cells {
        self.lines.first().map(SceneLine::width).unwrap_or_default()
    }
}

/// TODO: docs
#[derive(Debug)]
struct SceneLine {
    runs: Vec<SceneRun>,
}

impl SceneLine {
    /// Returns the length of the line, in bytes.
    ///
    /// Note that this will be greater than the number of terminal cells used
    /// to render the line if it contains multi-byte characters. Consider using
    /// the [`width`](Self::width) method for that.
    #[inline]
    fn byte_len(&self) -> usize {
        // FIXME: this is O(n). We could do it in O(1) by either memoizing it
        // or by storing the runs in a Btree.
        self.runs.iter().map(SceneRun::byte_len).sum()
    }

    /// Extends this line to the given width by appending an empty
    /// [`SceneRun`].
    #[inline]
    fn extend(&mut self, to_width: Cells) {
        if to_width > self.width() {
            let cells = to_width - self.width();
            self.runs.push(SceneRun::new_empty(cells));
        }
    }

    /// Creates a new empty `SceneLine` with the given width.
    #[inline]
    fn new_empty(width: Cells) -> Self {
        Self { runs: vec![SceneRun::new_empty(width)] }
    }

    /// Returns the index of the [`SceneRun`] at the given offset, along with
    /// the [`Cells`] offset of the run in this line.
    ///
    /// The [`Bias`] parameter is used to determine which run to return when
    /// the given offset falls between two runs.
    #[inline]
    fn run_at(&self, offset: Cells, bias: Bias) -> (usize, Cells) {
        let mut run_offset = Cells::zero();
        let mut runs = self.runs.iter().enumerate();

        loop {
            let Some((mut run_idx, run)) = runs.next() else {
                panic!("offset out of bounds");
            };

            match (run_offset + run.width()).cmp(&offset) {
                Ordering::Less => {
                    run_offset += run.width();
                },

                Ordering::Equal => {
                    if bias == Bias::Right {
                        if let Some((next_idx, _)) = runs.next() {
                            run_idx = next_idx;
                            run_offset += run.width();
                        }
                    }

                    return (run_idx, run_offset);
                },

                Ordering::Greater => {
                    return (run_idx, run_offset);
                },
            }
        }
    }

    /// Truncates this line to the given width by dropping the runs that exceed
    /// the width.
    #[inline]
    fn truncate(&mut self, to_width: Cells) {
        let (run_idx, run_offset) = self.run_at(to_width, Bias::Right);

        if run_offset < to_width {
            self.runs[run_idx].truncate(to_width - run_offset);
            self.runs.truncate(run_idx + 1);
        } else {
            self.runs.truncate(run_idx);
        }
    }

    /// Returns the width of the line in terminal [`Cells`].
    ///
    /// Note that this will be less than the number of bytes in the line if it
    /// contains multi-byte characters. Consider using the
    /// [`byte_len`](Self::byte_len) to get the number of bytes.
    #[inline]
    fn width(&self) -> Cells {
        // FIXME: this is O(n). We could do it in O(1) by either memoizing it
        // or by storing the runs in a Btree.
        self.runs.iter().map(SceneRun::width).sum()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Bias {
    Left,
    Right,
}

/// TODO: docs
#[derive(Debug)]
struct SceneRun {
    text: RunText,
}

impl SceneRun {
    /// TODO: docs.
    #[inline]
    fn byte_len(&self) -> usize {
        self.text.byte_len()
    }

    /// Creates a new empty `SceneRun` with the given width.
    #[inline]
    fn new_empty(width: Cells) -> Self {
        Self { text: RunText::new_empty(width) }
    }

    /// Returns the text of the `SceneRun`.
    #[inline]
    fn text(&self) -> Cow<str> {
        self.text.as_str()
    }

    /// TODO: docs.
    #[inline]
    fn truncate(&mut self, to_width: Cells) {
        self.text.truncate(to_width);
    }

    /// Returns the width of the run in terminal [`Cells`].
    ///
    /// This is equal to the number of terminal cells used to render the run's
    /// [`text`](Self::text).
    #[inline]
    fn width(&self) -> Cells {
        self.text.width()
    }
}

/// TODO: docs
#[derive(Debug)]
enum RunText {
    Spaces { width: Cells },
    Text { text: CompactString, width: Memoize<Cells> },
}

impl RunText {
    #[inline]
    fn as_str(&self) -> Cow<str> {
        match self {
            Self::Spaces { width } => spaces(*width),
            Self::Text { text, .. } => Cow::Borrowed(text.as_str()),
        }
    }

    /// Returns the length of the text, in bytes.
    #[inline]
    fn byte_len(&self) -> usize {
        match self {
            Self::Spaces { width } => u32::from(*width) as usize,
            Self::Text { text, .. } => text.len(),
        }
    }

    /// Creates a new empty `SceneRun` with the given width.
    #[inline]
    fn new_empty(width: Cells) -> Self {
        Self::Spaces { width }
    }

    /// TODO: docs.
    #[inline]
    fn truncate(&mut self, to_width: Cells) {
        match self {
            Self::Spaces { width } => *width = to_width,

            Self::Text { text, width } => {
                let (left, _) = to_width.split(text.as_str());
                text.truncate(left.len());
                // Reset the memoized width, if any.
                let _ = width.take();
            },
        }
    }

    /// Returns the width of the text in terminal [`Cells`].
    #[inline]
    fn width(&self) -> Cells {
        match self {
            Self::Spaces { width } => *width,
            Self::Text { text, width } => {
                *width.get(|| Cells::measure(text.as_str()))
            },
        }
    }
}

/// TODO: docs
#[derive(Debug, Default)]
struct DiffTracker {
    /// TODO: docs.
    deleted: Option<DeleteHunk>,

    /// TODO: docs.
    inserted: Option<InsertHunk>,

    /// TODO: docs.
    truncated: Vec<TruncateHunk>,

    /// TODO: docs.
    extended: Vec<ExtendHunk>,

    /// TODO: docs
    replaced: Vec<ReplaceHunk>,

    /// TODO: docs
    paint: Vec<PaintOp>,
}

#[derive(Debug)]
struct DeleteHunk {
    delete_all_from: usize,
}

impl DeleteHunk {
    #[inline]
    fn new(delete_all_from: usize) -> Self {
        Self { delete_all_from }
    }
}

#[derive(Debug)]
struct InsertHunk {
    at_line: usize,
    num_inserted: usize,
    width: Cells,
}

impl InsertHunk {
    #[inline]
    fn new(at_line: usize, num_inserted: usize, width: Cells) -> Self {
        Self { at_line, num_inserted, width }
    }
}

#[derive(Debug)]
struct TruncateHunk {
    range: Range<Point>,
}

impl TruncateHunk {
    #[inline]
    fn new(range: Range<Point>) -> Self {
        Self { range }
    }
}

#[derive(Debug)]
struct ExtendHunk {
    at: Point,
    width: Cells,
}

impl ExtendHunk {
    #[inline]
    fn new(at: Point, width: Cells) -> Self {
        Self { at, width }
    }
}

#[derive(Debug)]
struct ReplaceHunk {
    range: Range<Point>,
    replaced_with: (usize, Cells), // (line_idx, run_idx)
}

impl ReplaceHunk {
    #[inline]
    fn new(range: Range<Point>, replaced_with: (usize, Cells)) -> Self {
        Self { range, replaced_with }
    }
}

/// A `ResizeOp` is a collection of operations that resize a `Scene`.
#[derive(Debug, Copy, Clone, Default)]
struct ResizeOp {
    old_size: Bound<Cells>,
    shrink: ShrinkOp,
    expand: ExpandOp,
}

impl ResizeOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        self.shrink.apply_to(scene);
        self.expand.apply_to(scene);
    }

    #[inline]
    fn new(old_size: Bound<Cells>, new_size: Bound<Cells>) -> Self {
        let shrink = ShrinkOp::new(old_size, new_size);
        let expand = ExpandOp::new(old_size, new_size);
        Self { old_size, shrink, expand }
    }
}

/// A `ShrinkOp` shrinks a [`Scene`] by deleting lines and/or truncating lines.
#[derive(Debug, Copy, Clone, Default)]
struct ShrinkOp {
    delete_lines: Option<DeleteLinesOp>,
    truncate_lines: Option<TruncateLinesOp>,
}

impl ShrinkOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        if let Some(delete_lines) = self.delete_lines {
            delete_lines.apply_to(scene);
        }

        if let Some(truncate_lines) = self.truncate_lines {
            truncate_lines.apply_to(scene);
        }
    }

    #[inline]
    fn new(old_size: Bound<Cells>, new_size: Bound<Cells>) -> Self {
        let delete_lines = if new_size.height() < old_size.height() {
            Some(DeleteLinesOp((old_size.height() - new_size.height()).into()))
        } else {
            None
        };

        let truncate_lines = if new_size.width() < old_size.width() {
            Some(TruncateLinesOp((old_size.width() - new_size.width()).into()))
        } else {
            None
        };

        Self { delete_lines, truncate_lines }
    }
}

/// A `DeleteLinesOp(n)` shrinks a [`Scene`] vertically by keeping its first
/// `n` lines and deleting the rest.
///
/// For example, a `DeleteLinesOp(1)` would transform the following scene:
///
/// ```txt
/// ┌──────────────┐
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒3x14▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// └──────────────┘
/// ```
///
/// into:
///
/// ```txt
/// ┌──────────────┐
/// │▒▒▒▒▒1x14▒▒▒▒▒│
/// └──────────────┘
/// ```
///
/// A `DeleteLinesOp(0)` deletes all the lines of a `Scene`.
#[derive(Debug, Clone, Copy)]
struct DeleteLinesOp(u32);

impl DeleteLinesOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let num_lines = self.0 as usize;
        scene.diff_tracker.deleted = Some(DeleteHunk::new(num_lines));
        scene.surface.lines.truncate(num_lines);
    }
}

/// A `TruncateLinesOp(n)` shrinks a [`Scene`] horizontally by keeping the
/// first `n` cells of every line and deleting the rest.
///
/// For example, a `TruncateLinesOp(10)` would transform the following scene:
///
/// ```txt
/// ┌──────────────┐
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒3x14▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// └──────────────┘
/// ```
///
/// into:
///
/// ```txt
/// ┌──────────┐
/// │▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒3x10▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒│
/// └──────────┘
/// ```
///
/// A `TruncateLinesOp(0)` deletes all the cells of a `Scene`.
#[derive(Debug, Clone, Copy)]
struct TruncateLinesOp(u32);

impl TruncateLinesOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let cells = Cells::from(self.0);

        for (idx, line) in scene.surface.lines_mut().enumerate() {
            let start = Point::new(idx, line.byte_len());
            line.truncate(cells);
            let end = Point::new(idx, line.byte_len());
            scene.diff_tracker.truncated.push(TruncateHunk::new(start..end));
        }
    }
}

/// An `ExpandOp` expands a `Scene` by inserting lines and/or extending lines.
#[derive(Debug, Clone, Copy, Default)]
struct ExpandOp {
    extend_lines: Option<ExtendLinesOp>,
    insert_lines: Option<InsertLinesOp>,
}

impl ExpandOp {
    #[inline]
    fn apply_to(self, _scene: &mut Scene) {
        if let Some(extend_lines) = self.extend_lines {
            extend_lines.apply_to(_scene);
        }

        if let Some(insert_lines) = self.insert_lines {
            insert_lines.apply_to(_scene);
        }
    }

    #[inline]
    fn new(old_size: Bound<Cells>, new_size: Bound<Cells>) -> Self {
        let extend_lines = if new_size.width() > old_size.width() {
            Some(ExtendLinesOp((new_size.width() - old_size.width()).into()))
        } else {
            None
        };

        let insert_lines = if new_size.height() > old_size.height() {
            Some(InsertLinesOp((new_size.height() - old_size.height()).into()))
        } else {
            None
        };

        Self { extend_lines, insert_lines }
    }
}

/// An `InsertLinesOp(n)` expands a [`Scene`] vertically by appending lines
/// until its height reaches `n` cells.
///
/// For example, an `InsertLinesOp(5)` would transform the following scene:
///
/// ```txt
/// ┌──────────────┐
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒3x14▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// └──────────────┘
/// ```
///
/// into:
///
/// ```txt
/// ┌──────────────┐
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒5x14▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// └──────────────┘
/// ```
#[derive(Debug, Clone, Copy)]
struct InsertLinesOp(u32);

impl InsertLinesOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let len = self.0 as usize;

        let num_inserted = len - scene.surface.lines.len();

        scene.diff_tracker.inserted = Some(InsertHunk::new(
            scene.surface.lines.len(),
            num_inserted,
            scene.width(),
        ));

        let width = scene.width();
        scene.surface.lines.resize_with(len, || SceneLine::new_empty(width));
    }
}

/// An `ExtendLinesOp(n)` expands a [`Scene`] horizontally by extending every
/// line until its width reaches `n` cells.
///
/// For example, an `ExtendLinesOp(18)` would transform the following scene:
///
/// ```txt
/// ┌──────────────┐
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒3x14▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// └──────────────┘
/// ```
///
/// into:
///
/// ```txt
/// ┌──────────────────┐
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// │▒▒▒▒▒▒▒3x18▒▒▒▒▒▒▒│
/// │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
/// └──────────────────┘
/// ```
#[derive(Debug, Clone, Copy)]
struct ExtendLinesOp(u32);

impl ExtendLinesOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let cells = Cells::from(self.0);

        let insert_hunks =
            scene.surface.lines().enumerate().map(|(idx, line)| {
                let point = Point::new(idx, line.byte_len());
                ExtendHunk::new(point, cells)
            });

        scene.diff_tracker.extended.extend(insert_hunks);

        scene.surface.lines_mut().for_each(|line| line.extend(cells));
    }
}

/// TODO: docs
#[derive(Debug)]
struct PaintOp {}

/// TODO: docs
pub(crate) struct SceneDiff<'scene> {
    surface: &'scene SceneSurface,
    deleted: Option<DeleteHunk>,
    inserted: Option<InsertHunk>,
    truncated: Drain<'scene, TruncateHunk>,
    extended: Drain<'scene, ExtendHunk>,
    replaced: Drain<'scene, ReplaceHunk>,
    _paint: Drain<'scene, PaintOp>,
}

impl<'scene> SceneDiff<'scene> {
    /// TODO: docs
    #[inline]
    pub(crate) fn apply_to(mut self, surface: &mut Surface) {
        for hunk in self.text_hunks() {
            hunk.apply_to(surface);
        }

        for hunk in self.hl_hunks() {
            hunk.apply_to(surface);
        }
    }

    /// TODO: docs.
    #[inline]
    fn hl_hunks(&mut self) -> HlHunks<'_> {
        HlHunks { _marker: &() }
    }

    /// TODO: docs.
    #[inline]
    fn text_hunks(&mut self) -> TextHunks<'_, 'scene> {
        TextHunks { diff: self, status: TextHunkStatus::Deleted }
    }
}

/// TODO: docs.
struct HlHunks<'a> {
    _marker: &'a (),
}

impl Iterator for HlHunks<'_> {
    type Item = HlHunk;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// TODO: docs.
#[derive(Debug)]
struct HlHunk {}

/// TODO: docs.
impl HlHunk {
    /// TODO: docs
    #[inline]
    fn apply_to(self, surface: &mut Surface) {
        surface.highlight_text(self.point_range(), &self.hl());
    }

    /// TODO: docs
    #[inline]
    fn point_range(&self) -> Range<Point> {
        todo!();
    }

    /// TODO: docs
    #[inline]
    fn hl(&self) -> impl Highlight {
        todo!();
        crate::highlight::Normal
    }
}

/// TODO: docs.
struct TextHunks<'a, 'scene> {
    diff: &'a mut SceneDiff<'scene>,
    status: TextHunkStatus,
}

enum TextHunkStatus {
    Deleted,
    Inserted,
    Truncated,
    Extended,
    Replaced,
    Done,
}

impl<'scene> Iterator for TextHunks<'_, 'scene> {
    type Item = TextHunk<'scene>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let diff = &mut self.diff;
        loop {
            let text_hunk = match self.status {
                TextHunkStatus::Deleted => {
                    let Some(delete) = diff.deleted.take() else {
                        self.status = TextHunkStatus::Inserted;
                        continue;
                    };

                    TextHunk::DeleteLines(delete)
                },

                TextHunkStatus::Inserted => {
                    let Some(insert) = diff.inserted.take() else {
                        self.status = TextHunkStatus::Truncated;
                        continue;
                    };

                    TextHunk::InsertLines(insert)
                },

                TextHunkStatus::Truncated => {
                    let Some(truncate) = diff.truncated.next() else {
                        self.status = TextHunkStatus::Extended;
                        continue;
                    };
                    TextHunk::new_delete(truncate.range)
                },

                TextHunkStatus::Extended => {
                    let Some(extend) = diff.extended.next() else {
                        self.status = TextHunkStatus::Replaced;
                        continue;
                    };
                    TextHunk::new_insert(extend.at, spaces(extend.width))
                },

                TextHunkStatus::Replaced => {
                    let Some(replace) = diff.replaced.next() else {
                        self.status = TextHunkStatus::Done;
                        continue;
                    };
                    let (line_idx, run_offset) = replace.replaced_with;
                    let run =
                        diff.surface.run_at(line_idx, run_offset, Bias::Right);
                    TextHunk::new(replace.range, run.text())
                },

                TextHunkStatus::Done => return None,
            };

            return Some(text_hunk);
        }
    }
}

/// TODO: docs.
#[derive(Debug)]
enum TextHunk<'scene> {
    /// TODO: docs
    Replace { range: Range<Point>, text: Cow<'scene, str> },

    /// TODO: docs
    InsertLines(InsertHunk),

    /// TODO: docs
    DeleteLines(DeleteHunk),
}

/// TODO: docs.
impl<'scene> TextHunk<'scene> {
    /// TODO: docs
    #[inline]
    fn apply_to(self, surface: &mut Surface) {
        match self {
            Self::Replace { range, text } => {
                surface.replace_text(range, text.as_ref());
            },

            Self::InsertLines(insert) => {
                let line_range = insert.at_line..insert.at_line;
                let lines =
                    (0..insert.num_inserted).map(|_| spaces(insert.width));
                surface.replace_lines(line_range, lines);
            },

            Self::DeleteLines(delete) => {
                surface.replace_lines(
                    delete.delete_all_from..,
                    core::iter::empty::<&str>(),
                );
            },
        }
    }

    #[inline]
    fn new(range: Range<Point>, text: impl Into<Cow<'scene, str>>) -> Self {
        Self::Replace { range, text: text.into() }
    }

    #[inline]
    fn new_delete(range: Range<Point>) -> Self {
        Self::new(range, "")
    }

    #[inline]
    fn new_insert(at: Point, text: impl Into<Cow<'scene, str>>) -> Self {
        Self::new(at..at, text)
    }
}

impl From<DeleteHunk> for TextHunk<'_> {
    #[inline]
    fn from(delete: DeleteHunk) -> Self {
        Self::DeleteLines(delete)
    }
}

#[inline]
fn spaces(width: Cells) -> Cow<'static, str> {
    /// The sole purpose of this constant is to avoid allocating when the
    /// text is empty.
    const SPACES: &str = r#"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                "#;

    let len = u32::from(width) as usize;

    if len > SPACES.len() {
        Cow::Owned(" ".repeat(len))
    } else {
        Cow::Borrowed(&SPACES[..len])
    }
}
