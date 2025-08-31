#![allow(dead_code)]
#![allow(unused_imports)]

pub(crate) mod buffer;
pub(crate) mod cursor;
pub(crate) mod selection;
mod test_editor;

pub(crate) use cursor::{CursorCreation, CursorEvent, CursorMovement};
pub(crate) use test_editor::{ContextExt, TestEditor};
