use core::ops::Range;
use std::collections::hash_map;

use compact_str::CompactString;
use editor::Shared;
use nohash::IntMap as NoHashMap;
use slotmap::SlotMap;

use crate::buffer::{BufferId, Point};
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Clone)]
pub(crate) struct DecorationProvider {
    inner: Shared<DecorationProviderInner>,
}

/// TODO: docs.
pub(crate) struct HighlightRange {
    decoration_provider: DecorationProvider,
    buffer_id: BufferId,
    range_key: slotmap::DefaultKey,
}

struct DecorationProviderInner {
    namespace_id: u32,
    highlight_ranges: NoHashMap<BufferId, HighlightRanges>,
}

/// The highlight ranges to be drawn in a given buffer.
#[derive(Debug)]
struct HighlightRanges {
    buffer_id: BufferId,
    inner: SlotMap<slotmap::DefaultKey, HighlightRangeInner>,
}

#[derive(Debug)]
struct HighlightRangeInner {
    highlight_group_name: CompactString,
    point_range: Range<Point>,
}

impl HighlightRange {
    /// The ID of the buffer this range is in.
    #[inline]
    pub(crate) fn buffer_id(&self) -> BufferId {
        self.buffer_id
    }

    /// Moves the [`HighlightRange`] to the given [`Point`] range.
    #[inline]
    pub(crate) fn r#move(&self, point_range: Range<Point>) {
        self.with_inner(|range| {
            range.point_range = point_range;
        })
    }

    #[inline]
    pub(crate) fn set_hl_group(&self, hl_group_name: &str) {
        self.with_inner(|range| {
            range.highlight_group_name = hl_group_name.into();
        })
    }

    #[inline]
    fn with_inner<T>(
        &self,
        fun: impl FnOnce(&mut HighlightRangeInner) -> T,
    ) -> T {
        self.decoration_provider.inner.with_mut(|decoration_provider| {
            let inner = decoration_provider
                .highlight_ranges
                .get_mut(&self.buffer_id)
                .expect(
                    "not removed until all ranges on the buffer are dropped",
                )
                .inner
                .get_mut(self.range_key)
                .expect("not removed until this range is dropped");

            fun(inner)
        })
    }
}

impl DecorationProvider {
    #[inline]
    pub(crate) fn highlight_range(
        &self,
        buffer_id: BufferId,
        point_range: Range<Point>,
        highlight_group_name: &str,
    ) -> HighlightRange {
        let range_inner = HighlightRangeInner {
            highlight_group_name: highlight_group_name.into(),
            point_range,
        };

        let range_key = self.inner.with_mut(|inner| {
            let ranges = match inner.highlight_ranges.entry(buffer_id) {
                hash_map::Entry::Occupied(entry) => entry.into_mut(),
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(HighlightRanges {
                        buffer_id,
                        inner: SlotMap::new(),
                    })
                },
            };

            ranges.inner.insert(range_inner)
        });

        HighlightRange {
            decoration_provider: self.clone(),
            buffer_id,
            range_key,
        }
    }

    #[inline]
    pub(crate) fn namespace_id(&self) -> u32 {
        self.inner.with(|inner| inner.namespace_id)
    }

    #[inline]
    pub(crate) fn new(namespace_id: u32) -> Self {
        let this = Self {
            inner: Shared::new(DecorationProviderInner {
                namespace_id,
                highlight_ranges: Default::default(),
            }),
        };

        let opts = api::opts::DecorationProviderOpts::builder()
            .on_start(this.on_start().catch_unwind().map(
                |maybe_dont_skip_cycle| maybe_dont_skip_cycle.unwrap_or(false),
            ))
            .on_win(this.on_win().catch_unwind().map(
                |maybe_dont_skip_on_lines| {
                    maybe_dont_skip_on_lines.unwrap_or(false)
                },
            ))
            .build();

        api::set_decoration_provider(namespace_id, &opts)
            .expect("couldn't set decoration provider");

        this
    }

    #[inline]
    fn on_start(
        &self,
    ) -> impl Fn(api::opts::OnStartArgs) -> api::opts::DontSkipRedrawCycle + 'static
    {
        let inner = self.inner.clone();

        move |_args| {
            inner.with(|inner| {
                // The return value informs Neovim whether to execute the
                // various callbacks for the current redraw cycle.
                !inner.highlight_ranges.is_empty()
            })
        }
    }

    #[inline]
    fn on_win(
        &self,
    ) -> impl Fn(api::opts::OnWinArgs) -> api::opts::DontSkipOnLines + 'static
    {
        let inner = self.inner.clone();

        move |(_, _win, buf, _toprow, _botrow)| {
            let buf_id = BufferId::from(buf);

            inner.with_mut(|inner| {
                // Draw the highlight ranges for the given buffer.
                if let Some(ranges) = inner.highlight_ranges.get_mut(&buf_id) {
                    ranges.redraw(inner.namespace_id);
                }
            });

            false
        }
    }
}

impl HighlightRanges {
    fn redraw(&mut self, namespace_id: u32) {
        for range in self.inner.values_mut() {
            let opts = api::opts::SetExtmarkOpts::builder()
                .end_row(range.point_range.end.newline_offset)
                .end_col(range.point_range.end.byte_offset)
                .ephemeral(true)
                .hl_group(&*range.highlight_group_name)
                .build();

            // NOTE: not in the docs, but when setting ephemeral extmarks
            // nvim_buf_set_extmark always returns 0 as the extmark ID, which
            // isn't a valid ID because it's not positive.
            api::Buffer::from(self.buffer_id)
                .set_extmark(
                    namespace_id,
                    range.point_range.start.newline_offset,
                    range.point_range.start.byte_offset,
                    &opts,
                )
                .expect("couldn't set extmark");
        }
    }
}

impl Drop for HighlightRange {
    #[inline]
    fn drop(&mut self) {
        self.decoration_provider.inner.with_mut(|inner| {
            let highlight_ranges = &mut inner
                .highlight_ranges
                .get_mut(&self.buffer_id)
                .expect(
                    "not removed until all ranges on the buffer are dropped",
                )
                .inner;

            highlight_ranges.remove(self.range_key);

            if highlight_ranges.is_empty() {
                inner.highlight_ranges.remove(&self.buffer_id);
            }
        });
    }
}
