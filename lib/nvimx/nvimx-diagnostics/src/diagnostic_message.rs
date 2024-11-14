use core::iter;

use nvim_oxi::api;

use crate::diagnostic_source::DiagnosticSource;
use crate::highlight_group::HighlightGroup;
use crate::level::Level;

/// TODO: docs.
#[derive(Default)]
pub struct DiagnosticMessage {
    chunks: Vec<(nvim_oxi::String, Option<HighlightGroup>)>,
}

impl DiagnosticMessage {
    /// TODO: docs.
    pub fn emit(self, level: Level, source: DiagnosticSource) {
        emit(level, source, self);
    }

    /// Creates a new, empty [`DiagnosticMessage`].
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO: docs.
    pub fn push_comma_separated<T, I>(
        &mut self,
        iter: I,
        hl: HighlightGroup,
    ) -> &mut Self
    where
        T: AsRef<str>,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.push_separated(iter, hl, ", ")
    }

    /// TODO: docs.
    pub fn push_dot_separated<T, I>(
        &mut self,
        iter: I,
        hl: HighlightGroup,
    ) -> &mut Self
    where
        T: AsRef<str>,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.push_separated(iter, hl, ".")
    }

    /// TODO: docs.
    pub fn push_str<T: AsRef<str>>(&mut self, s: T) -> &mut Self {
        self.push_chunk(s.as_ref(), None)
    }

    /// TODO: docs.
    pub fn push_str_highlighted<T: AsRef<str>>(
        &mut self,
        s: T,
        hl: HighlightGroup,
    ) -> &mut Self {
        self.push_chunk(s.as_ref(), Some(hl))
    }

    fn push_chunk(
        &mut self,
        s: &str,
        hl: Option<HighlightGroup>,
    ) -> &mut Self {
        self.chunks.push((nvim_oxi::String::from(s), hl));
        self
    }

    fn push_separated<T, I>(
        &mut self,
        iter: I,
        hl: HighlightGroup,
        separator: &str,
    ) -> &mut Self
    where
        T: AsRef<str>,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = iter.into_iter();
        let len = iter.len();
        for (idx, text) in iter.enumerate() {
            self.push_str_highlighted(text.as_ref(), hl.clone());
            let is_last = idx + 1 == len;
            if !is_last {
                self.push_str(separator);
            }
        }
        self
    }
}

fn emit(level: Level, source: DiagnosticSource, msg: DiagnosticMessage) {
    let source_chunk = (source.to_string().into(), Some(level.into()));
    let space_chunk = (" ".into(), None);
    let chunks = iter::once(source_chunk)
        .chain(iter::once(space_chunk))
        .chain(msg.chunks);
    let opts = api::opts::EchoOpts::default();
    api::echo(chunks, true, &opts).expect("all parameters are valid");
}

impl From<core::convert::Infallible> for DiagnosticMessage {
    fn from(_: core::convert::Infallible) -> Self {
        unreachable!()
    }
}
