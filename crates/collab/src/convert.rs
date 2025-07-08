/// Same as [`Into`], but for types defined in other crates (for which we
/// couldn't implement [`Into`] because of the orphan rule).
pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<collab_project::text::TextReplacement> for ed::Replacement {
    fn convert(self) -> collab_project::text::TextReplacement {
        collab_project::text::TextReplacement {
            deleted_range: self.removed_range(),
            inserted_text: self.inserted_text().into(),
        }
    }
}

impl Convert<ed::Replacement> for collab_project::text::TextReplacement {
    fn convert(self) -> ed::Replacement {
        ed::Replacement::new(self.deleted_range, &*self.inserted_text)
    }
}
