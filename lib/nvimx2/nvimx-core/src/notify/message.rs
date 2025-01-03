use core::fmt;
use core::ops::Range;

use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::ByteOffset;

/// TODO: docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    inner: SmolStr,
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
    Unknown,

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
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// TODO: docs.
    #[inline]
    pub fn spans(&self) -> Spans<'_> {
        Spans::new(self)
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
