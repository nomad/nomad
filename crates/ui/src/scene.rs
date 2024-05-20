use alloc::borrow::Cow;
use alloc::vec::Drain;
use core::cmp::Ordering;
use core::mem;
use core::ops::Range;

use compact_str::CompactString;

use crate::{
    Bound,
    Cells,
    HighlightGroup,
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
        SceneFragment::new(self)
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn diff(&mut self) -> SceneDiff<'_> {
        SceneDiff::new(self)
    }

    /// Returns the height of the scene in terminal [`Cells`].
    #[inline]
    pub(crate) fn height(&self) -> Cells {
        self.surface.height()
    }

    /// Returns the [`SceneLine`] at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    #[inline]
    pub(crate) fn line_mut(&mut self, line_idx: usize) -> SceneLineBorrow<'_> {
        SceneLineBorrow {
            line: self.surface.line_mut(line_idx),
            diff_tracker: &mut self.diff_tracker,
            line_idx,
            offset: Cells::zero(),
        }
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
    fn line_mut(&mut self, line_idx: usize) -> &mut SceneLine {
        &mut self.lines[line_idx]
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

    /// TODO: docs.
    #[inline]
    fn splice<Runs>(&mut self, range: Range<Cells>, runs: Runs)
    where
        Runs: IntoIterator<Item = SceneRun>,
    {
        let (start_idx, start_offset) = self.run_at(range.start, Bias::Right);

        let (end_idx, end_offset) = self.run_at(range.end, Bias::Left);

        if start_idx == end_idx {
            let run = &mut self.runs[start_idx];
            let remainder = run.split(range.end - start_offset);
            let _ = run.split(range.start - start_offset);
            let splice_start = start_idx;
            let splice_end = start_idx + run.width().is_zero() as usize;
            let runs = runs.into_iter().chain(remainder);
            self.runs.splice(splice_start..splice_end, runs);
            return;
        }

        let start_run = &mut self.runs[start_idx];

        let start_remainder = start_run.split(range.start - start_offset);

        let splice_start = start_idx + start_remainder.is_some() as usize;

        let end_run = &mut self.runs[end_idx];

        let end_remainder = end_run
            .split(range.end - end_offset)
            .map(|split| mem::replace(end_run, split));

        let splice_end = end_idx - end_remainder.is_none() as usize;

        let runs = runs.into_iter().chain(end_remainder);

        self.runs.splice(splice_start..splice_end, runs);
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

    /// TODO: docs.
    #[inline]
    fn split(&mut self, at: Cells) -> Option<Self> {
        self.text.split(at).map(|text| Self { text })
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
    fn split(&mut self, _at: Cells) -> Option<Self> {
        todo!();
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
    vertical_resize: Option<VerticalResizeHunk>,

    /// TODO: docs.
    horizontal_shrink: Vec<HorizontalShrinkHunk>,

    /// TODO: docs.
    horizontal_expand: Vec<HorizontalExpandHunk>,

    /// TODO: docs
    replaced: Vec<ReplaceHunk>,

    /// TODO: docs
    paint: Vec<PaintOp>,
}

#[derive(Debug)]
enum VerticalResizeHunk {
    Shrink { delete_from_line_idx: usize },
    Expand { num_lines_start: usize, num_inserted: usize, lines_width: Cells },
}

impl VerticalResizeHunk {
    #[inline]
    fn apply_to(self, surface: &mut Surface) {
        match self {
            Self::Shrink { delete_from_line_idx } => {
                surface.replace_lines(
                    delete_from_line_idx..,
                    core::iter::empty::<&str>(),
                );
            },

            Self::Expand { num_lines_start, num_inserted, lines_width } => {
                let line_range = num_lines_start..num_lines_start;
                let lines = (0..num_inserted).map(|_| spaces(lines_width));
                surface.replace_lines(line_range, lines);
            },
        }
    }
}

#[derive(Debug)]
struct HorizontalShrinkHunk {
    range: Range<Point<usize>>,
}

impl HorizontalShrinkHunk {
    #[inline]
    fn new(range: Range<Point<usize>>) -> Self {
        Self { range }
    }
}

#[derive(Debug)]
struct HorizontalExpandHunk {
    at: Point<usize>,
    width: Cells,
}

impl HorizontalExpandHunk {
    #[inline]
    fn new(at: Point<usize>, width: Cells) -> Self {
        Self { at, width }
    }
}

#[derive(Debug)]
struct ReplaceHunk {
    range: Range<Point<usize>>,
    replaced_with: (usize, Cells), // (line_idx, run_idx)
}

impl ReplaceHunk {
    #[inline]
    fn new(range: Range<Point<usize>>, replaced_with: (usize, Cells)) -> Self {
        Self { range, replaced_with }
    }
}

/// A `ResizeOp` is a collection of operations that resize a `Scene`.
#[derive(Debug, Copy, Clone, Default)]
struct ResizeOp {
    vertical: Option<VerticalOp>,
    horizontal: Option<HorizontalOp>,
}

#[derive(Debug, Copy, Clone)]
enum VerticalOp {
    Shrink(VerticalShrinkOp),
    Expand(VerticalExpandOp),
}

#[derive(Debug, Copy, Clone)]
enum HorizontalOp {
    Shrink(HorizontalShrinkOp),
    Expand(HorizontalExpandOp),
}

impl HorizontalOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        match self {
            Self::Shrink(shrink) => shrink.apply_to(scene),
            Self::Expand(expand) => expand.apply_to(scene),
        }
    }
}

impl ResizeOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        match self.vertical {
            Some(VerticalOp::Shrink(v_shrink)) => {
                v_shrink.apply_to(scene);
                self.horizontal.inspect(|op| op.apply_to(scene));
            },

            Some(VerticalOp::Expand(v_expand)) => {
                self.horizontal.inspect(|op| op.apply_to(scene));
                v_expand.apply_to(scene);
            },

            None => {
                self.horizontal.inspect(|op| op.apply_to(scene));
            },
        }
    }

    #[inline]
    fn new(old_size: Bound<Cells>, new_size: Bound<Cells>) -> Self {
        let vertical = match old_size.height().cmp(&new_size.height()) {
            Ordering::Less => Some(VerticalOp::Expand(VerticalExpandOp(
                (new_size.height() - old_size.height()).into(),
            ))),
            Ordering::Greater => Some(VerticalOp::Shrink(VerticalShrinkOp(
                (old_size.height() - new_size.height()).into(),
            ))),
            Ordering::Equal => None,
        };

        let horizontal = match old_size.width().cmp(&new_size.width()) {
            Ordering::Less => Some(HorizontalOp::Expand(HorizontalExpandOp(
                (new_size.width() - old_size.width()).into(),
            ))),
            Ordering::Greater => {
                Some(HorizontalOp::Shrink(HorizontalShrinkOp(
                    (old_size.width() - new_size.width()).into(),
                )))
            },
            Ordering::Equal => None,
        };

        Self { vertical, horizontal }
    }
}

/// A `VerticalShrinkOp(n)` shrinks a [`Scene`] vertically by keeping its first
/// `n` lines and deleting the rest.
///
/// For example, a `VerticalShrinkOp(1)` would transform the following scene:
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
/// A `VerticalShrinkOp(0)` deletes all the lines of a `Scene`.
#[derive(Debug, Clone, Copy)]
struct VerticalShrinkOp(u32);

impl VerticalShrinkOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let num_lines = self.0 as usize;

        scene.diff_tracker.vertical_resize =
            Some(VerticalResizeHunk::Shrink {
                delete_from_line_idx: num_lines,
            });

        scene.surface.lines.truncate(num_lines);
    }
}

/// A `HorizontalShrinkOp(n)` shrinks a [`Scene`] horizontally by keeping the
/// first `n` cells of every line and deleting the rest.
///
/// For example, a `HorizontalShrinkOp(10)` would transform the following scene:
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
/// A `HorizontalShrinkOp(0)` deletes all the cells of a `Scene`.
#[derive(Debug, Clone, Copy)]
struct HorizontalShrinkOp(u32);

impl HorizontalShrinkOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let cells = Cells::from(self.0);

        for (idx, line) in scene.surface.lines_mut().enumerate() {
            let start = Point::new(idx, line.byte_len());
            line.truncate(cells);
            let end = Point::new(idx, line.byte_len());
            scene
                .diff_tracker
                .horizontal_shrink
                .push(HorizontalShrinkHunk::new(start..end));
        }
    }
}

/// A `VerticalExpandOp(n)` expands a [`Scene`] vertically by appending lines
/// until its height reaches `n` cells.
///
/// For example, a `VerticalExpandOp(5)` would transform the following scene:
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
struct VerticalExpandOp(u32);

impl VerticalExpandOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let len = self.0 as usize;

        let num_inserted = len - scene.surface.lines.len();

        scene.diff_tracker.vertical_resize =
            Some(VerticalResizeHunk::Expand {
                num_lines_start: scene.surface.lines.len(),
                num_inserted,
                lines_width: scene.width(),
            });

        let width = scene.width();
        scene.surface.lines.resize_with(len, || SceneLine::new_empty(width));
    }
}

/// A `HorizontalExpandOp(n)` expands a [`Scene`] horizontally by extending
/// every line until its width reaches `n` cells.
///
/// For example, a `HorizontalExpandOp(18)` would transform the following
/// scene:
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
struct HorizontalExpandOp(u32);

impl HorizontalExpandOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        let cells = Cells::from(self.0);

        let insert_hunks =
            scene.surface.lines().enumerate().map(|(idx, line)| {
                let point = Point::new(idx, line.byte_len());
                HorizontalExpandHunk::new(point, cells)
            });

        scene.diff_tracker.horizontal_expand.extend(insert_hunks);

        scene.surface.lines_mut().for_each(|line| line.extend(cells));
    }
}

/// TODO: docs
pub(crate) struct SceneLineBorrow<'scene> {
    line: &'scene mut SceneLine,
    diff_tracker: &'scene mut DiffTracker,
    line_idx: usize,
    offset: Cells,
}

impl<'scene> SceneLineBorrow<'scene> {
    /// TODO: docs
    #[inline]
    pub(crate) fn split_run(
        self,
        split_at: Cells,
    ) -> (SceneRunBorrow<'scene>, Option<Self>) {
        todo!();
    }

    /// TODO: docs
    #[inline]
    pub fn width(&self) -> Cells {
        self.line.width() - self.offset
    }
}

/// TODO: docs
pub(crate) struct SceneRunBorrow<'scene> {
    run: &'scene mut SceneRun,
    diff_tracker: &'scene mut DiffTracker,
    line_idx: usize,
    offset: Cells,
}

impl<'scene> SceneRunBorrow<'scene> {
    /// TODO: docs.
    pub(crate) fn set_text(&mut self, _text: &str) {
        todo!();
    }

    /// TODO: docs.
    pub(crate) fn set_highlight(&mut self, _hl_group: &HighlightGroup) {
        todo!();
    }

    /// TODO: docs
    #[inline]
    pub fn width(&self) -> Cells {
        self.run.width()
    }
}

impl Drop for SceneLineBorrow<'_> {
    #[inline]
    fn drop(&mut self) {
        todo!();
    }
}

/// TODO: docs
#[derive(Debug)]
struct PaintOp {}

/// TODO: docs
pub(crate) struct SceneDiff<'scene> {
    surface: &'scene SceneSurface,
    resize_vertical: Option<VerticalResizeHunk>,
    resize_horizontal: HorizontalHunks<'scene>,
    replaced: Drain<'scene, ReplaceHunk>,
    _paint: Drain<'scene, PaintOp>,
}

enum HorizontalHunks<'scene> {
    Shrink(Drain<'scene, HorizontalShrinkHunk>),
    Expand(Drain<'scene, HorizontalExpandHunk>),
}

impl<'scene> SceneDiff<'scene> {
    /// TODO: docs
    #[inline]
    pub(crate) fn apply_to(mut self, surface: &mut Surface) {
        for text_hunk in self.text_hunks() {
            text_hunk.apply_to(surface);
        }

        for hl_hunk in self.hl_hunks() {
            hl_hunk.apply_to(surface);
        }
    }

    /// TODO: docs.
    #[inline]
    fn hl_hunks(&mut self) -> HlHunks<'_> {
        HlHunks { _marker: &() }
    }

    /// TODO: docs.
    #[inline]
    fn new(scene: &'scene mut Scene) -> Self {
        let tracker = &mut scene.diff_tracker;

        let resize_horizontal = if tracker.horizontal_shrink.is_empty() {
            HorizontalHunks::Expand(tracker.horizontal_expand.drain(..))
        } else {
            HorizontalHunks::Shrink(tracker.horizontal_shrink.drain(..))
        };

        Self {
            surface: &scene.surface,
            resize_vertical: tracker.vertical_resize.take(),
            resize_horizontal,
            replaced: tracker.replaced.drain(..),
            _paint: tracker.paint.drain(..),
        }
    }

    /// TODO: docs.
    #[inline]
    fn text_hunks(&mut self) -> TextHunks<'_, 'scene> {
        TextHunks::new(self)
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
struct HlHunk {
    range: Range<Point<usize>>,
    hl_group: HighlightGroup,
}

/// TODO: docs.
impl HlHunk {
    /// TODO: docs
    #[inline]
    fn apply_to(self, surface: &mut Surface) {
        surface.highlight_text(self.range.clone(), &self.hl_group);
    }

    /// TODO: docs
    #[inline]
    fn new(range: Range<Point<usize>>, hl_group: HighlightGroup) -> Self {
        Self { range, hl_group }
    }
}

/// TODO: docs.
struct TextHunks<'a, 'scene> {
    diff: &'a mut SceneDiff<'scene>,
    status: TextHunksStatus,
}

enum TextHunksStatus {
    ResizeVertical,
    ResizeHorizontal,
    Replaced,
    Done,
}

impl<'a, 'scene> TextHunks<'a, 'scene> {
    #[inline]
    fn new(diff: &'a mut SceneDiff<'scene>) -> Self {
        Self { diff, status: TextHunksStatus::ResizeVertical }
    }
}

impl<'scene> Iterator for TextHunks<'_, 'scene> {
    type Item = TextHunk<'scene>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let diff = &mut self.diff;

        loop {
            let text_hunk = match self.status {
                TextHunksStatus::ResizeVertical => {
                    let Some(resize) = diff.resize_vertical.take() else {
                        self.status = TextHunksStatus::ResizeHorizontal;
                        continue;
                    };

                    TextHunk::VerticalResize(resize)
                },

                TextHunksStatus::ResizeHorizontal => {
                    let Some(text_hunk) = (match &mut diff.resize_horizontal {
                        HorizontalHunks::Shrink(shrinks) => {
                            shrinks.next().map(Into::into)
                        },
                        HorizontalHunks::Expand(expands) => {
                            expands.next().map(Into::into)
                        },
                    }) else {
                        self.status = TextHunksStatus::Replaced;
                        continue;
                    };

                    text_hunk
                },

                TextHunksStatus::Replaced => {
                    let Some(replace) = diff.replaced.next() else {
                        self.status = TextHunksStatus::Done;
                        continue;
                    };

                    let (line_idx, run_offset) = replace.replaced_with;

                    TextHunk::Replace {
                        range: replace.range,
                        text: diff
                            .surface
                            .run_at(line_idx, run_offset, Bias::Right)
                            .text(),
                    }
                },

                TextHunksStatus::Done => return None,
            };

            return Some(text_hunk);
        }
    }
}

/// TODO: docs.
#[derive(Debug)]
enum TextHunk<'scene> {
    VerticalResize(VerticalResizeHunk),
    Replace { range: Range<Point<usize>>, text: Cow<'scene, str> },
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

            Self::VerticalResize(resize) => resize.apply_to(surface),
        }
    }
}

impl From<HorizontalShrinkHunk> for TextHunk<'_> {
    #[inline]
    fn from(shrink: HorizontalShrinkHunk) -> Self {
        Self::Replace { range: shrink.range, text: Cow::Borrowed("") }
    }
}

impl From<HorizontalExpandHunk> for TextHunk<'_> {
    #[inline]
    fn from(expand: HorizontalExpandHunk) -> Self {
        Self::Replace {
            range: expand.at..expand.at,
            text: spaces(expand.width),
        }
    }
}

#[inline]
fn spaces(width: Cells) -> Cow<'static, str> {
    /// The purpose of this constant is to avoid allocating when the desired
    /// width is <= 512.
    const SPACES: &str = r#"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                "#;

    let len = u32::from(width) as usize;

    if len > SPACES.len() {
        Cow::Owned(" ".repeat(len))
    } else {
        Cow::Borrowed(&SPACES[..len])
    }
}
