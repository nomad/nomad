use core::fmt;
use core::ops::Range;

use compact_str::{CompactString, ToCompactString};
use smallvec::SmallVec;

use crate::ByteOffset;

/// TODO: docs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Message {
    inner: CompactString,
    spans: SmallVec<[SpanInner; 4]>,
}

/// TODO: docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    /// TODO: docs.
    Expected,

    /// TODO: docs.
    Actual,

    /// TODO: docs.
    Invalid,

    /// TODO: docs.
    Info,

    /// TODO: docs.
    Warning,

    /// TODO: docs.
    Error,
}

/// TODO: docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'msg> {
    message: &'msg Message,
    byte_range: Range<ByteOffset>,
    kind: Option<&'msg SpanKind>,
}

/// TODO: docs.
#[derive(Clone)]
pub struct Spans<'msg> {
    message: &'msg Message,
    state: SpansState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpanInner {
    byte_range: Range<ByteOffset>,
    kind: SpanKind,
}

#[derive(Clone)]
enum SpansState {
    InGap { gap_offset: usize, byte_range: Range<ByteOffset> },
    InSpan { idx: usize },
    Done,
}

impl Message {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// TODO: docs.
    #[inline]
    pub fn byte_len(&self) -> ByteOffset {
        self.inner.len().into()
    }

    /// TODO: docs.
    #[inline]
    pub fn from_debug<S: fmt::Debug>(s: S) -> Self {
        struct DisplayAsDebug<T>(T);
        impl<T: fmt::Debug> fmt::Display for DisplayAsDebug<T> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }
        Self::from_display(DisplayAsDebug(s))
    }

    /// TODO: docs.
    #[inline]
    pub fn from_display<S: fmt::Display>(s: S) -> Self {
        Self { inner: s.to_compact_string(), spans: Default::default() }
    }

    /// TODO: docs.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Self {
        Self { inner: s.as_ref().into(), spans: Default::default() }
    }

    /// TODO: docs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Constructs a new, empty [`Message`].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO: docs.
    #[inline]
    pub fn push_comma_separated<I, S>(
        &mut self,
        iter: I,
        span_kind: SpanKind,
    ) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.push_separated(iter, span_kind, ", ")
    }

    /// TODO: docs.
    #[inline]
    pub fn push_span<S: AsRef<str>>(
        &mut self,
        s: S,
        span_kind: SpanKind,
    ) -> &mut Self {
        let s = s.as_ref();
        if s.is_empty() {
            return self;
        }
        let start = self.inner.len().into();
        self.inner.push_str(s);
        let end = self.inner.len().into();
        self.spans.push(SpanInner { byte_range: start..end, kind: span_kind });
        self
    }

    /// TODO: docs.
    #[inline]
    pub fn push_actual<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        self.push_span(s, SpanKind::Actual)
    }

    /// TODO: docs.
    #[inline]
    pub fn push_expected<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        self.push_span(s, SpanKind::Expected)
    }

    /// TODO: docs.
    #[inline]
    pub fn push_info<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        self.push_span(s, SpanKind::Info)
    }

    /// TODO: docs.
    #[inline]
    pub fn push_invalid<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        self.push_span(s, SpanKind::Invalid)
    }

    /// TODO: docs.
    #[inline]
    pub fn push_str<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        self.inner.push_str(s.as_ref());
        self
    }

    /// TODO: docs.
    #[inline]
    pub fn push_with<F>(&mut self, fun: F) -> &mut Self
    where
        F: FnOnce(&mut Self),
    {
        fun(self);
        self
    }

    /// TODO: docs.
    #[inline]
    pub fn spans(&self) -> Spans<'_> {
        Spans::new(self)
    }

    #[inline]
    fn push_separated<I, S>(
        &mut self,
        iter: I,
        span_kind: SpanKind,
        separator: &str,
    ) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = iter.into_iter().peekable();
        loop {
            let Some(s) = iter.next() else { break };
            self.push_span(s, span_kind.clone());
            if iter.peek().is_some() {
                self.push_str(separator);
            }
        }
        self
    }
}

impl<'a> Span<'a> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        let br = &self.byte_range;
        let range: Range<usize> = br.start.into()..br.end.into();
        &self.message.as_str()[range]
    }

    /// TODO: docs.
    #[inline]
    pub fn byte_range(&self) -> Range<ByteOffset> {
        self.byte_range.clone()
    }

    /// TODO: docs.
    #[inline]
    pub fn kind(&self) -> Option<&'a SpanKind> {
        self.kind
    }
}

impl<'a> Spans<'a> {
    #[inline]
    fn new(message: &'a Message) -> Self {
        Self { message, state: SpansState::new(message) }
    }
}

impl SpansState {
    #[inline]
    fn advance(&mut self, msg: &Message) {
        let next_state = match &self {
            Self::InGap { gap_offset, .. } => {
                let next_idx = *gap_offset;
                if next_idx < msg.spans.len() {
                    Self::InSpan { idx: next_idx }
                } else {
                    Self::Done
                }
            },
            Self::InSpan { idx } => {
                let this = &msg.spans[*idx];
                let next_idx = *idx + 1;

                match msg.spans.get(next_idx) {
                    Some(next) => {
                        let gap = this.byte_range.end..next.byte_range.start;
                        if gap.is_empty() {
                            Self::InSpan { idx: next_idx }
                        } else {
                            Self::InGap {
                                gap_offset: next_idx,
                                byte_range: gap,
                            }
                        }
                    },
                    None => {
                        let range = this.byte_range.end..msg.byte_len();
                        if range.is_empty() {
                            Self::Done
                        } else {
                            Self::InGap {
                                gap_offset: next_idx,
                                byte_range: range,
                            }
                        }
                    },
                }
            },
            Self::Done => return,
        };
        *self = next_state;
    }

    #[inline]
    fn new(message: &Message) -> Self {
        let first_span = match message.spans.first() {
            Some(span) => span,
            None if !message.is_empty() => {
                return Self::InGap {
                    gap_offset: 0,
                    byte_range: ByteOffset::range(0..message.as_str().len()),
                };
            },
            None => return Self::Done,
        };
        let start = usize::from(first_span.byte_range.start);
        if start > 0 {
            Self::InGap {
                gap_offset: 0,
                byte_range: ByteOffset::range(0..start),
            }
        } else {
            Self::InSpan { idx: 0 }
        }
    }
}

impl fmt::Display for Message {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'a> Iterator for Spans<'a> {
    type Item = Span<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let span = match &self.state {
            SpansState::InGap { byte_range, .. } => Span {
                message: self.message,
                byte_range: byte_range.clone(),
                kind: None,
            },
            SpansState::InSpan { idx } => {
                let span_inner = &self.message.spans[*idx];
                Span {
                    message: self.message,
                    byte_range: span_inner.byte_range.clone(),
                    kind: Some(&span_inner.kind),
                }
            },
            SpansState::Done => return None,
        };
        self.state.advance(self.message);
        Some(span)
    }
}
