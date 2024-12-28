//! TODO: docs.

use nvim_oxi::Object;
use nvimx_core::notify;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::oxi;

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
) -> Result<oxi::Object, NeovimSerializeError> {
    serde_path_to_error::serialize(value, oxi::serde::Serializer::new())
        .map_err(|inner| NeovimSerializeError { inner })
}

#[inline]
pub(crate) fn deserialize<T: DeserializeOwned>(
    object: Object,
) -> Result<T, NeovimDeserializeError> {
    serde_path_to_error::deserialize(oxi::serde::Deserializer::new(object))
        .map_err(|inner| NeovimDeserializeError { inner })
}

impl notify::Error for NeovimSerializeError {
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        Some(notify::Level::Error)
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        todo!()
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
