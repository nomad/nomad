use ed::backend::Backend;
use ed::fs::{DirectoryEvent, Fs};

/// TODO: docs.
pub(crate) enum Event<B: Backend> {
    Directory(DirectoryEvent<<B::Fs as Fs>::Directory>),
}
