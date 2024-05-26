use core::str::FromStr;

use api::opts::{BufAttachOpts, OnBytesArgs, OptionOpts};
use nvim_oxi::api::{self, Buffer};
use nvimx_common::{Apply, ByteOffset, Replacement, Shared};

/// TODO: docs.
pub struct TextBuffer {
    /// TODO: docs.
    attach_status: AttachStatus,

    /// TODO: docs.
    inner: Buffer,
}

impl TextBuffer {
    /// Attaches to the buffer if not already attached, and returns a mutable
    /// reference to the [`AttachState`].
    #[inline]
    fn attach(&mut self) -> &mut Shared<AttachState> {
        if let AttachStatus::Attached(state) = &mut self.attach_status {
            return state;
        }

        let state = Shared::default();

        let on_edit = {
            let state = state.clone();
            move |args: OnBytesArgs| {
                let replacement = Replacement::from(args);
                state.with_mut(|state| state.on_edit(replacement));
                Ok(false)
            }
        };

        let opts = BufAttachOpts::builder().on_bytes(on_edit).build();

        self.inner.attach(false, &opts).map_err(|err| todo!());

        self.attach_status = AttachStatus::Attached(state);

        return self.attach();
    }

    /// TODO: docs.
    #[inline]
    pub fn current() -> Result<Self, NotTextBufferError> {
        let buffer = Buffer::current();

        match buffer.buftype() {
            Buftype::Text => Ok(Self::new(buffer)),
            Buftype::Help => Err(NotTextBufferError::Help),
            Buftype::Quickfix => Err(NotTextBufferError::Quickfix),
            Buftype::Terminal => Err(NotTextBufferError::Terminal),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn edit<E>(&mut self, edit: E) -> <Self as Apply<E>>::Diff
    where
        Self: Apply<E>,
    {
        if let AttachStatus::Attached(state) = &self.attach_status {
            state.with_mut(|state| state.edit_side = EditSide::Ours);
        }

        self.apply(edit)
    }

    /// Creates a new text buffer from the given [`Buffer`].
    ///
    /// # Panics
    ///
    /// Panics if the buffer's type is not [`Buftype::Text`].
    #[inline]
    fn new(inner: Buffer) -> Self {
        debug_assert!(inner.buftype().is_text());
        Self { attach_status: AttachStatus::NotAttached, inner }
    }
}

enum AttachStatus {
    Attached(Shared<AttachState>),
    NotAttached,
}

#[derive(Default)]
struct AttachState {
    /// Whether the edit was performed by calling [`Buffer::edit`].
    edit_side: EditSide,

    /// Callbacks registered to be called when the buffer is edited.
    on_edit_callbacks: Vec<Box<dyn FnMut(&Replacement<ByteOffset>)>>,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub enum EditSide {
    /// TODO: docs.
    Ours,

    /// TODO: docs.
    #[default]
    Theirs,
}

trait BufferExt {
    fn buftype(&self) -> Buftype;
}

impl BufferExt for Buffer {
    #[inline]
    fn buftype(&self) -> Buftype {
        let opts = OptionOpts::builder().buffer(self.clone()).build();

        api::get_option_value::<String>("buftype", &opts)
            .expect("always set")
            .parse()
            .unwrap_or_else(|other| panic!("unknown buftype: {other}"))
    }
}

enum Buftype {
    Text,
    Help,
    Quickfix,
    Terminal,
}

impl Buftype {
    #[inline]
    fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }
}

impl FromStr for Buftype {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // `:h buftype` for more infos.
        match s {
            "" => Ok(Self::Text),
            "help" => Ok(Self::Help),
            "quickfix" => Ok(Self::Quickfix),
            "terminal" => Ok(Self::Terminal),
            other => Err(other.to_owned()),
        }
    }
}

/// Error type returned by [`TextBuffer::current`] when the current buffer
/// is not a text buffer.
#[derive(Debug)]
pub enum NotTextBufferError {
    /// The current buffer is a help file.
    Help,

    /// The current buffer is a quickfix list.
    Quickfix,

    /// The current buffer houses a terminal emulator.
    Terminal,
}
