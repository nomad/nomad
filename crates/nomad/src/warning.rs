//! TODO: docs

use crate::{ActionName, ModuleName};

/// TODO: docs
#[derive(Default)]
pub(crate) struct Warning {
    /// TODO: docs
    msg: WarningMsg,

    /// TODO: docs
    on_action: Option<ActionName>,

    /// TODO: docs
    on_module: Option<ModuleName>,
}

impl Warning {
    /// TODO: docs
    #[inline]
    pub(crate) fn action(mut self, action: ActionName) -> Self {
        self.on_action = Some(action);
        self
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn module(mut self, module: ModuleName) -> Self {
        self.on_module = Some(module);
        self
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn msg(mut self, msg: WarningMsg) -> Self {
        self.msg = msg;
        self
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn print(self) {
        if self.msg.chunks.is_empty() {
            return;
        }

        let tag = NomadTag::new(self.on_module, self.on_action);

        let chunks = [Chunk::warning(tag), Chunk::space()]
            .into_iter()
            .chain(self.msg.chunks)
            .map(Chunk::into_tuple);

        let _ = nvim::api::echo(chunks, true, &Default::default());
    }
}

/// TODO: docs
pub struct Chunk {
    text: nvim::String,
    highlight_group: Option<&'static str>,
}

impl Chunk {
    /// TODO: docs
    #[inline]
    fn into_tuple(self) -> (nvim::String, Option<&'static str>) {
        (self.text, self.highlight_group)
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new<T: Into<nvim::String>>(
        text: T,
        highlight_group: Option<&'static str>,
    ) -> Self {
        Self { text: text.into(), highlight_group }
    }

    /// TODO: docs
    #[inline]
    fn space() -> Self {
        Self::new(" ", None)
    }

    /// TODO: docs
    #[inline]
    fn warning<T: Into<nvim::String>>(text: T) -> Self {
        Self { text: text.into(), highlight_group: Some("WarningMsg") }
    }
}

/// TODO: docs
#[derive(Default)]
pub struct WarningMsg {
    chunks: Vec<Chunk>,
}

impl WarningMsg {
    /// TODO: docs
    #[inline]
    pub fn add<C: Into<Chunk>>(&mut self, chunk: C) -> &mut Self {
        self.chunks.push(chunk.into());
        self
    }

    /// TODO: docs
    #[inline]
    pub fn add_invalid(
        &mut self,
        invalid: impl AsRef<str>,
        mut valid: impl ExactSizeIterator<Item = impl AsRef<str>> + Clone,
        what: &str,
    ) -> &mut Self {
        let invalid = invalid.as_ref();

        match InvalidMsgKind::new(invalid, valid.clone()) {
            InvalidMsgKind::ListAll => list_all(invalid, what, valid, self),

            InvalidMsgKind::SuggestClosest { closest_idx } => {
                let Some(closest) = valid.nth(closest_idx) else {
                    unreachable!("iterator has at least idx+1 elements");
                };

                suggest_closest(invalid, what, closest.as_ref(), self)
            },
        }

        self
    }

    /// TODO: docs
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<core::convert::Infallible> for WarningMsg {
    #[inline]
    fn from(_: core::convert::Infallible) -> Self {
        unreachable!("Infallible can't be constructed")
    }
}

impl From<&str> for Chunk {
    #[inline]
    fn from(text: &str) -> Self {
        Self::new(text, None)
    }
}

impl From<String> for Chunk {
    #[inline]
    fn from(text: String) -> Self {
        Self::new(text.as_str(), None)
    }
}

/// TODO: docs
struct NomadTag {
    module: Option<ModuleName>,
    action: Option<ActionName>,
}

impl NomadTag {
    #[inline]
    fn new(module: Option<ModuleName>, action: Option<ActionName>) -> Self {
        Self { module, action }
    }
}

impl From<NomadTag> for nvim::String {
    #[inline]
    fn from(tag: NomadTag) -> Self {
        let mut s = String::from("[nomad");

        if let Some(module) = tag.module {
            s.push('.');
            s.push_str(module.as_str());
        }

        if let Some(action) = tag.action {
            s.push('.');
            s.push_str(action.as_str());
        }

        s.push(']');

        s.as_str().into()
    }
}

/// TODO: docs
pub(crate) trait ChunkExt: Into<Chunk> {
    fn highlight(self) -> Chunk;
}

impl<T: Into<Chunk>> ChunkExt for T {
    #[inline]
    fn highlight(self) -> Chunk {
        let mut chunk = self.into();
        chunk.highlight_group = Some("Identifier");
        chunk
    }
}

/// TODO: docs
enum InvalidMsgKind {
    ListAll,
    SuggestClosest { closest_idx: usize },
}

impl InvalidMsgKind {
    #[inline]
    fn new(
        invalid: &str,
        valid: impl ExactSizeIterator<Item = impl AsRef<str>>,
    ) -> Self {
        if valid.len() == 0 {
            return Self::ListAll;
        }

        let mut min_distance = usize::MAX;

        let mut closest_idx = 0;

        for (idx, valid) in valid.enumerate() {
            let distance =
                strsim::damerau_levenshtein(invalid, valid.as_ref());

            if distance < min_distance {
                min_distance = distance;
                closest_idx = idx;
            }
        }

        let should_suggest_closest = match invalid.len() {
            // These ranges and cutoffs are arbitrary.
            3 => min_distance <= 1,
            4..=6 => min_distance <= 2,
            7..=10 => min_distance <= 3,
            _ => false,
        };

        if should_suggest_closest {
            Self::SuggestClosest { closest_idx }
        } else {
            Self::ListAll
        }
    }
}

/// TODO: docs
#[inline]
fn list_all(
    invalid: &str,
    invalid_what: &str,
    mut valid: impl ExactSizeIterator<Item = impl AsRef<str>>,
    msg: &mut WarningMsg,
) {
    msg.add("invalid ").add(invalid_what).add(" ").add(invalid.highlight());

    match valid.len() {
        0 => {},

        1 => {
            let Some(valid) = valid.next() else {
                unreachable!("the iterator has exactly one element")
            };

            msg.add(", the only valid ")
                .add(invalid_what)
                .add(" is ")
                .add(valid.as_ref().highlight());
        },

        num_valid => {
            msg.add(", the valid ").add(invalid_what).add("s are ");

            for (idx, valid) in valid.enumerate() {
                msg.add(valid.as_ref().highlight());

                let is_last = idx + 1 == num_valid;

                if is_last {
                    break;
                }

                let is_second_to_last = idx + 2 == num_valid;

                if is_second_to_last {
                    msg.add(" and ");
                } else {
                    msg.add(", ");
                }
            }
        },
    }
}

/// TODO: docs
#[inline]
fn suggest_closest(
    invalid: &str,
    invalid_what: &str,
    closest: impl AsRef<str>,
    msg: &mut WarningMsg,
) {
    msg.add("invalid ")
        .add(invalid_what)
        .add(" ")
        .add(invalid.highlight())
        .add(", did you mean ")
        .add(closest.as_ref().highlight())
        .add("?");
}
