//! TODO: docs

use crate::prelude::{ActionName, ModuleName};

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
    pub(crate) fn _module(mut self, module: ModuleName) -> Self {
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
pub(crate) struct Chunk {
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
    pub(crate) fn add<C: Into<Chunk>>(&mut self, chunk: C) -> &mut Self {
        self.chunks.push(chunk.into());
        self
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
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
