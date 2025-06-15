use core::ops::Range;

use ed::ByteOffset;

/// Same as [`Into`], but for types defined in other crates (for which we
/// couldn't implement [`Into`] because of the orphan rule).
pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<collab_project::text::TextReplacement> for ed::Replacement {
    fn convert(self) -> collab_project::text::TextReplacement {
        collab_project::text::TextReplacement {
            deleted_range: self.removed_range().convert(),
            inserted_text: self.inserted_text().into(),
        }
    }
}

impl Convert<Range<usize>> for Range<ByteOffset> {
    fn convert(self) -> Range<usize> {
        self.start.into()..self.end.into()
    }
}
