//! TODO: docs.

use core::{fmt, iter};

use nvim_oxi::{api, String as NvimString};
use smol_str::SmolStr;

pub(super) fn emit(
    level: Level,
    source: DiagnosticSource,
    msg: DiagnosticMessage,
) {
    let source_chunk = (source.to_string().into(), Some(level.into()));
    let space_chunk = (" ".into(), None);
    let chunks = iter::once(source_chunk)
        .chain(iter::once(space_chunk))
        .chain(msg.chunks);
    let opts = api::opts::EchoOpts::default();
    api::echo(chunks, true, &opts).expect("all parameters are valid");
}

pub(super) enum Level {
    Warning,
    Error,
}

impl From<Level> for HighlightGroup {
    fn from(level: Level) -> Self {
        match level {
            Level::Warning => HighlightGroup::warning(),
            Level::Error => HighlightGroup::error(),
        }
    }
}

#[derive(Default)]
pub(crate) struct DiagnosticSource {
    segments: Vec<SmolStr>,
}

impl DiagnosticSource {
    pub(super) fn new() -> Self {
        Self { segments: Vec::new() }
    }

    pub(super) fn push_segment(&mut self, segment: &str) -> &mut Self {
        self.segments.push(SmolStr::new(segment));
        self
    }
}

impl fmt::Display for DiagnosticSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}", crate::Nomad::DIAGNOSTICS_SEGMENT_NAME)?;

        for (idx, segment) in self.segments.iter().enumerate() {
            let is_last = idx + 1 == self.segments.len();
            write!(f, "{}", segment)?;
            if !is_last {
                write!(f, ".")?;
            }
        }

        write!(f, "]")
    }
}

/// TODO: docs.
#[derive(Default)]
pub struct DiagnosticMessage {
    chunks: Vec<(NvimString, Option<HighlightGroup>)>,
}

impl DiagnosticMessage {
    /// Creates a new, empty [`DiagnosticMessage`].
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn emit(self, level: Level, source: DiagnosticSource) {
        emit(level, source, self);
    }

    pub(super) fn push_dot_separated<T, I>(
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

    pub(super) fn push_comma_separated<T, I>(
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

    pub(super) fn push_str<T: AsRef<str>>(&mut self, s: T) -> &mut Self {
        self.push_chunk(s.as_ref(), None)
    }

    pub(super) fn push_str_highlighted<T: AsRef<str>>(
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
        self.chunks.push((NvimString::from(s), hl));
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

/// TODO: docs.
#[derive(Clone)]
pub struct HighlightGroup(SmolStr);

impl HighlightGroup {
    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(super) fn error() -> Self {
        Self::new("ErrorMsg")
    }

    pub(super) fn new(group: &str) -> Self {
        Self(SmolStr::new(group))
    }

    pub(super) fn special() -> Self {
        Self::new("Special")
    }

    pub(super) fn warning() -> Self {
        Self::new("WarningMsg")
    }
}

impl From<core::convert::Infallible> for DiagnosticMessage {
    fn from(_: core::convert::Infallible) -> Self {
        unreachable!()
    }
}

impl From<HighlightGroup> for NvimString {
    fn from(group: HighlightGroup) -> Self {
        group.as_str().into()
    }
}
