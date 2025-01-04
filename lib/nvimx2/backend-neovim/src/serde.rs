//! TODO: docs.

use nvimx_core::notify;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::oxi;
use crate::value::NeovimValue;

/// TODO: docs.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct NeovimSerializeError {
    inner: serde_path_to_error::Error<oxi::serde::SerializeError>,
}

/// TODO: docs.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct NeovimDeserializeError {
    inner: serde_path_to_error::Error<oxi::serde::DeserializeError>,
}

#[inline]
pub(crate) fn serialize<T: ?Sized + Serialize>(
    value: &T,
) -> Result<NeovimValue, NeovimSerializeError> {
    serde_path_to_error::serialize(value, oxi::serde::Serializer::new())
        .map(NeovimValue::new)
        .map_err(|inner| NeovimSerializeError { inner })
}

#[inline]
pub(crate) fn deserialize<T: DeserializeOwned>(
    value: NeovimValue,
) -> Result<T, NeovimDeserializeError> {
    serde_path_to_error::deserialize(oxi::serde::Deserializer::new(
        value.into_inner(),
    ))
    .map_err(|inner| NeovimDeserializeError { inner })
}

impl notify::Error for NeovimSerializeError {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        let mut message = notify::Message::new();
        message.push_str("couldn't serialize value");
        if self.inner.path().iter().len() > 1 {
            message.push_str(" at ").push_info(self.inner.path().to_string());
        }
        message.push_str(": ").push_str(self.inner.inner().to_string());
        message
    }
}

impl notify::Error for NeovimDeserializeError {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        todo!()
    }
}
