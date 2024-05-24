use alloc::borrow::Cow;
use alloc::vec::Drain;
use core::cmp::Ordering;
use core::marker::PhantomData;
use core::mem;
use core::ops::Range;
use core::ptr::NonNull;

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
        SceneLineBorrow::new(self, line_idx)
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
    // Q: do we really have to split the text into runs?
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
    fn splice<Runs>(&mut self, range: Range<Cells>, runs: Runs) -> usize
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
            return splice_start;
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

        splice_start
    }

    /// Converts the given [`Cells`] offset into the corresponding byte offset.
    #[inline]
    fn to_byte_offset(&self, cell_offset: Cells) -> usize {
        let mut cells = Cells::zero();
        let mut byte_offset = 0;

        for run in &self.runs {
            if cells + run.width() >= cell_offset {
                return byte_offset + run.to_byte_offset(cell_offset - cells);
            }
            cells += run.width();
            byte_offset += run.byte_len();
        }

        unreachable!();
    }

    /// Converts the given [`Cells`] range into the corresponding byte range.
    #[inline]
    fn to_byte_range(&self, cell_range: Range<Cells>) -> Range<usize> {
        self.to_byte_offset(cell_range.start)
            ..self.to_byte_offset(cell_range.end)
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

    /// Sets the text of this `SceneRun`.
    #[inline]
    fn set_text(&mut self, text: &str) {
        self.text = RunText::Text {
            text: CompactString::from(text),
            width: Memoize::new(),
        };
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

    /// Converts the given [`Cells`] offset into the corresponding byte offset.
    #[inline]
    fn to_byte_offset(&self, cell_offset: Cells) -> usize {
        self.text.to_byte_offset(cell_offset)
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
    fn split(&mut self, split_at: Cells) -> Option<Self> {
        if split_at == self.width() {
            return None;
        }

        match self {
            Self::Spaces { width } => {
                let remainder = *width - split_at;
                *width = split_at;
                Some(Self::Spaces { width: remainder })
            },

            Self::Text { text, width } => {
                let (left, right) = split_at.split(text.as_str());

                let split = Some(Self::Text {
                    text: CompactString::from(right),
                    width: Memoize::new(),
                });

                text.truncate(left.len());
                let _ = width.take();

                split
            },
        }
    }

    /// Converts the given [`Cells`] offset into the corresponding byte offset.
    #[inline]
    fn to_byte_offset(&self, cell_offset: Cells) -> usize {
        match self {
            Self::Spaces { .. } => u32::from(cell_offset) as usize,
            Self::Text { text, .. } => {
                let (left, _) = cell_offset.split(text.as_str());
                left.len()
            },
        }
    }

    /// TODO: docs.
    #[inline]
    fn truncate(&mut self, to_width: Cells) {
        match self {
            Self::Spaces { width } => *width = to_width,

            Self::Text { text, width } => {
                let (left, _) = to_width.split(text.as_str());
                text.truncate(left.len());
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
    window_resize: Option<Bound<Cells>>,

    /// TODO: docs.
    vertical_resize: Option<VerticalResizeHunk>,

    /// TODO: docs.
    horizontal_shrink: Vec<HorizontalShrinkHunk>,

    /// TODO: docs.
    horizontal_expand: Vec<HorizontalExpandHunk>,

    /// TODO: docs.
    ///
    /// It's crucial that the hunks are applied to the [`Surface`] in the exact
    /// order in which they are stored in the vector, as the byte range in each
    /// hunk is relative to the state of the scene once all the previous hunks
    /// have been applied.
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
                surface.delete_lines(delete_from_line_idx..);
            },

            Self::Expand { num_lines_start, num_inserted, lines_width } => {
                let lines = (0..num_inserted).map(|_| spaces(lines_width));
                surface.insert_lines(num_lines_start, lines);
            },
        }
    }
}

#[derive(Debug)]
struct HorizontalShrinkHunk {
    line: usize,
    range: Range<usize>,
}

impl HorizontalShrinkHunk {
    #[inline]
    fn new(line: usize, range: Range<usize>) -> Self {
        Self { line, range }
    }
}

#[derive(Debug)]
struct HorizontalExpandHunk {
    /// The point at which we should expand the line.
    at: Point<usize>,
    /// The width to expand the line to.
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
    line_idx: usize,
    byte_range: Range<usize>,
    replaced_with_run_at: Cells,
}

impl ReplaceHunk {
    #[inline]
    fn new(
        line_idx: usize,
        byte_range: Range<usize>,
        replaced_with_run_at: Cells,
    ) -> Self {
        Self { line_idx, byte_range, replaced_with_run_at }
    }
}

/// A `ResizeOp` is a collection of operations that resize a `Scene`.
#[derive(Debug, Copy, Clone, Default)]
struct ResizeOp {
    window: Option<Bound<Cells>>,
    vertical: Option<VerticalOp>,
    horizontal: Option<HorizontalOp>,
}

#[derive(Debug, Copy, Clone)]
enum VerticalOp {
    Shrink(VerticalShrinkOp),
    Expand(VerticalExpandOp),
}

impl VerticalOp {
    #[inline]
    fn apply_to(self, scene: &mut Scene) {
        match self {
            Self::Shrink(shrink) => shrink.apply_to(scene),
            Self::Expand(expand) => expand.apply_to(scene),
        }
    }
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
        scene.diff_tracker.window_resize = self.window;
        self.vertical.inspect(|op| op.apply_to(scene));
        self.horizontal.inspect(|op| op.apply_to(scene));
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

        Self {
            window: (old_size != new_size).then_some(new_size),
            vertical,
            horizontal,
        }
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
            let start = line.byte_len();
            line.truncate(cells);
            let end = line.byte_len();
            scene
                .diff_tracker
                .horizontal_shrink
                .push(HorizontalShrinkHunk::new(idx, start..end));
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
                let point = Point::new(line.byte_len(), idx);
                HorizontalExpandHunk::new(point, cells)
            });

        scene.diff_tracker.horizontal_expand.extend(insert_hunks);

        scene.surface.lines_mut().for_each(|line| line.extend(cells));
    }
}

/// TODO: docs
#[derive(Clone, Copy)]
pub(crate) struct SceneLineBorrow<'scene> {
    run: SceneRunBorrow<'scene>,
}

impl<'scene> SceneLineBorrow<'scene> {
    /// TODO: docs
    #[inline]
    pub(crate) fn into_run(self) -> SceneRunBorrow<'scene> {
        self.run
    }

    #[inline]
    fn new(scene: &'scene mut Scene, line_idx: usize) -> Self {
        Self { run: SceneRunBorrow::new(scene, line_idx) }
    }
}

/// TODO: docs
#[derive(Clone, Copy)]
pub(crate) struct SceneRunBorrow<'scene> {
    /// TODO: docs.
    diff_tracker: NonNull<DiffTracker>,

    /// Used to track whether calling the [`set_text`](Self::set_text) method
    /// should push a new hunk to the diff tracker.
    ///
    /// It does the first time it's called, and doesn't do it on subsequent
    /// calls.
    has_set_text: HasSetText,

    /// The line this run is a part of.
    line: NonNull<SceneLine>,

    /// The index of the line this run is a part of.
    line_idx: usize,

    /// The offset in the line where this run starts.
    offset: Cells,

    /// The width of the run.
    width: Cells,

    _lifetime: PhantomData<&'scene SceneLine>,
}

#[derive(Debug, Copy, Clone)]
enum HasSetText {
    No,
    Yes { run_idx: usize },
}

impl<'scene> SceneRunBorrow<'scene> {
    /// Returns a mutable reference to the [`DiffTracker`].
    ///
    /// Note that this method is not part of this type's public API and should
    /// only be used internally by other methods.
    #[inline]
    fn diff_tracker_mut(&mut self) -> &mut DiffTracker {
        // SAFETY: this type doesn't give references, so there can't be
        // multiple mutable aliases.
        unsafe { self.diff_tracker.as_mut() }
    }

    /// Returns a mutable reference to this run's [`SceneLine`].
    ///
    /// Note that this method is not part of this type's public API and should
    /// only be used internally by other methods.
    #[inline]
    fn line_mut(&mut self) -> &mut SceneLine {
        // SAFETY: this type doesn't give references, so there can't be
        // multiple mutable aliases.
        unsafe { self.line.as_mut() }
    }

    #[inline]
    fn new(scene: &'scene mut Scene, line_idx: usize) -> Self {
        let line = scene.surface.line_mut(line_idx);
        let width = line.width();
        Self {
            diff_tracker: NonNull::from(&mut scene.diff_tracker),
            has_set_text: HasSetText::No,
            line: NonNull::from(line),
            line_idx,
            offset: Cells::zero(),
            width,
            _lifetime: PhantomData,
        }
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn set_hl_group(&mut self, _hl_group: &HighlightGroup) {
        todo!();
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn set_text(&mut self, text: &str) {
        debug_assert_eq!(Cells::measure(text), self.width());

        // This check is needed because `split()` allows the offset to be
        // both zero and equal to the run's width, which in both cases causes
        // one of the two returned runs to be empty.
        if self.width() == Cells::zero() {
            return;
        }

        let run_idx = match self.has_set_text {
            HasSetText::Yes { run_idx } => run_idx,
            HasSetText::No => {
                let cell_range = self.offset..self.offset + self.width();

                let hunk = ReplaceHunk::new(
                    self.line_idx,
                    self.line_mut().to_byte_range(cell_range.clone()),
                    self.offset,
                );

                self.diff_tracker_mut().replaced.push(hunk);

                let run_width = self.width();

                let run_idx = self
                    .line_mut()
                    .splice(cell_range, [SceneRun::new_empty(run_width)]);

                self.has_set_text = HasSetText::Yes { run_idx };

                run_idx
            },
        };

        self.line_mut().runs[run_idx].set_text(text);
    }

    /// Splits the run at the given offset.
    #[inline]
    pub(crate) fn split(self, split_at: Cells) -> (Self, Self) {
        debug_assert!(split_at <= self.width);

        let mut left = self;
        let mut right = self;

        left.has_set_text = HasSetText::No;
        right.has_set_text = HasSetText::No;

        right.offset += split_at;

        left.width = split_at;
        right.width -= split_at;

        (left, right)
    }

    /// Returns the run's width.
    #[inline]
    pub(crate) fn width(&self) -> Cells {
        self.width
    }
}

/// TODO: docs
#[derive(Debug)]
struct PaintOp {}

/// TODO: docs
pub(crate) struct SceneDiff<'scene> {
    surface: &'scene SceneSurface,
    resize_window: Option<Bound<Cells>>,
    resize_vertical: Option<VerticalResizeHunk>,
    resize_horizontal: HorizontalHunks<'scene>,
    replaced: Drain<'scene, ReplaceHunk>,
    _paint: Drain<'scene, PaintOp>,
}

impl core::fmt::Debug for SceneDiff<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SceneDiff")
            .field("resize_window", &self.resize_window)
            .field("resize_vertical", &self.resize_vertical)
            .field("resize_horizontal", &self.resize_horizontal)
            .field("replaced", &self.replaced.as_slice())
            .finish()
    }
}

enum HorizontalHunks<'scene> {
    Shrink(Drain<'scene, HorizontalShrinkHunk>),
    Expand(Drain<'scene, HorizontalExpandHunk>),
}

impl core::fmt::Debug for HorizontalHunks<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Shrink(hunks) => {
                f.debug_tuple("Shrink").field(&hunks.as_slice()).finish()
            },
            Self::Expand(hunks) => {
                f.debug_tuple("Expand").field(&hunks.as_slice()).finish()
            },
        }
    }
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
            resize_window: tracker.window_resize.take(),
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

impl HlHunk {
    /// Applies the hunk to the [`Surface`], consuming it.
    #[inline]
    fn apply_to(self, surface: &mut Surface) {
        surface.highlight_text(self.range.clone(), &self.hl_group);
    }

    #[inline]
    fn _new(range: Range<Point<usize>>, hl_group: HighlightGroup) -> Self {
        Self { range, hl_group }
    }
}

/// TODO: docs.
struct TextHunks<'a, 'scene> {
    diff: &'a mut SceneDiff<'scene>,
    status: TextHunksStatus,
}

enum TextHunksStatus {
    ResizeWindow,
    ResizeVertical,
    ResizeHorizontal,
    Replaced,
    Done,
}

impl<'a, 'scene> TextHunks<'a, 'scene> {
    #[inline]
    fn new(diff: &'a mut SceneDiff<'scene>) -> Self {
        Self { diff, status: TextHunksStatus::ResizeWindow }
    }
}

impl<'scene> Iterator for TextHunks<'_, 'scene> {
    type Item = TextHunk<'scene>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let diff = &mut self.diff;

        loop {
            let text_hunk = match self.status {
                TextHunksStatus::ResizeWindow => {
                    let Some(resize) = diff.resize_window.take() else {
                        self.status = TextHunksStatus::ResizeVertical;
                        continue;
                    };

                    TextHunk::WindowResize(resize)
                },

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

                    let text = diff
                        .surface
                        .run_at(
                            replace.line_idx,
                            replace.replaced_with_run_at,
                            Bias::Right,
                        )
                        .text();

                    TextHunk::Replace {
                        line: replace.line_idx,
                        range: replace.byte_range,
                        text,
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
    WindowResize(Bound<Cells>),
    VerticalResize(VerticalResizeHunk),
    Replace { line: usize, range: Range<usize>, text: Cow<'scene, str> },
}

/// TODO: docs.
impl<'scene> TextHunk<'scene> {
    /// TODO: docs
    #[inline]
    fn apply_to(self, surface: &mut Surface) {
        match self {
            Self::WindowResize(new_size) => {
                surface.resize_window(new_size);
            },

            Self::Replace { line, range, text } => {
                surface.replace_text(line, range, text.as_ref());
            },

            Self::VerticalResize(resize) => resize.apply_to(surface),
        }
    }
}

impl From<HorizontalShrinkHunk> for TextHunk<'_> {
    #[inline]
    fn from(shrink: HorizontalShrinkHunk) -> Self {
        Self::Replace {
            line: shrink.line,
            range: shrink.range,
            text: Cow::Borrowed(""),
        }
    }
}

impl From<HorizontalExpandHunk> for TextHunk<'_> {
    #[inline]
    fn from(expand: HorizontalExpandHunk) -> Self {
        Self::Replace {
            line: expand.at.y(),
            range: expand.at.x()..expand.at.x(),
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
