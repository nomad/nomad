//! TODO: docs.

use core::convert::Infallible;
use core::fmt;

use crate::command::{CommandArgs, CommandCompletion};
use crate::notify::{self, Name};
use crate::{ByteOffset, Editor};

/// TODO: docs.
pub type ApiValue<B> = <<B as Editor>::Api as Api>::Value;

/// TODO: docs.
pub trait Api: 'static + Sized {
    /// TODO: docs.
    type Value: Value;

    /// TODO: docs.
    fn add_constant(&mut self, constant_name: Name, value: Self::Value);

    /// TODO: docs.
    fn add_function<Fun>(&mut self, function_name: Name, function: Fun)
    where
        Fun: FnMut(Self::Value) -> Option<Self::Value> + 'static;

    /// TODO: docs.
    fn add_submodule(&mut self, module_name: Name, module_api: Self);

    /// TODO: docs.
    fn add_command<Command, CompletionFn, Completions>(
        &mut self,
        command_name: Name,
        command: Command,
        completion_fn: CompletionFn,
    ) where
        Command: FnMut(CommandArgs) + 'static,
        CompletionFn: FnMut(CommandArgs, ByteOffset) -> Completions + 'static,
        Completions: IntoIterator<Item = CommandCompletion>;

    /// TODO: docs.
    fn new(module_name: Name) -> Self;
}

/// TODO: docs.
pub trait Value: 'static {
    /// TODO: docs.
    type MapAccess<'a>: MapAccess<Value = Self>;

    /// TODO: docs.
    type MapAccessError<'a>: notify::Error
    where
        Self: 'a;

    /// TODO: docs.
    fn map_access(
        &mut self,
    ) -> Result<Self::MapAccess<'_>, Self::MapAccessError<'_>>;
}

/// TODO: docs.
pub trait MapAccess {
    /// TODO: docs.
    type Key<'a>: Key
    where
        Self: 'a;

    /// TODO: docs.
    type Value;

    /// TODO: docs.
    fn next_key(&mut self) -> Option<Self::Key<'_>>;

    /// TODO: docs.
    fn take_next_value(&mut self) -> Self::Value;
}

/// TODO: docs.
pub trait Key: fmt::Debug {
    /// TODO: docs.
    type AsStrError<'a>: notify::Error
    where
        Self: 'a;

    /// TODO: docs.
    fn as_str(&self) -> Result<&str, Self::AsStrError<'_>>;
}

impl<MA: MapAccess> MapAccess for &mut MA {
    type Key<'a>
        = MA::Key<'a>
    where
        Self: 'a;

    type Value = MA::Value;

    #[inline]
    fn next_key(&mut self) -> Option<Self::Key<'_>> {
        MA::next_key(self)
    }

    #[inline]
    fn take_next_value(&mut self) -> Self::Value {
        MA::take_next_value(self)
    }
}

impl Key for &str {
    type AsStrError<'a>
        = Infallible
    where
        Self: 'a;

    #[inline]
    fn as_str(&self) -> Result<&str, Self::AsStrError<'_>> {
        Ok(self)
    }
}
