use ed::notify;
use serde::{Deserialize, Serialize};

use crate::value::Value;

/// TODO: docs.
pub struct SerializeError {
    inner: serde_json::Error,
}

/// TODO: docs.
pub struct DeserializeError {
    inner: serde_json::Error,
}

pub(crate) fn serialize<T>(value: &T) -> Result<Value, SerializeError>
where
    T: ?Sized + Serialize,
{
    serde_json::to_value(value)
        .map(Into::into)
        .map_err(|inner| SerializeError { inner })
}

pub(crate) fn deserialize<'de, T>(value: Value) -> Result<T, DeserializeError>
where
    T: Deserialize<'de>,
{
    serde_json::Value::try_from(value)
        .and_then(T::deserialize)
        .map_err(|inner| DeserializeError { inner })
}

impl notify::Error for SerializeError {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (
            notify::Level::Error,
            notify::Message::from_str(self.inner.to_string()),
        )
    }
}

impl notify::Error for DeserializeError {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (
            notify::Level::Error,
            notify::Message::from_str(self.inner.to_string()),
        )
    }
}
