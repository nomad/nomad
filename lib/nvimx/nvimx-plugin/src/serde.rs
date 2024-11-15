use core::fmt;

use nvim_oxi::serde::{
    DeserializeError as NvimDeserializeError,
    Deserializer as NvimDeserializer,
    Serializer as NvimSerializer,
};
use nvim_oxi::Object as NvimObject;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::diagnostics::{DiagnosticMessage, HighlightGroup};

pub(super) fn deserialize<T>(obj: NvimObject) -> Result<T, DeserializeError>
where
    T: DeserializeOwned,
{
    serde_path_to_error::deserialize(NvimDeserializer::new(obj))
        .map_err(|inner| DeserializeError { inner })
}

/// # Panics
///
/// Panics if the [`Serialize`] implementation for `T` returns an error.
#[track_caller]
pub(super) fn serialize<T>(item: &T) -> NvimObject
where
    T: Serialize,
{
    match item.serialize(NvimSerializer::new()) {
        Ok(obj) => obj,
        Err(err) => {
            panic!(
                "couldn't serialize value of type '{}': {}",
                std::any::type_name::<T>(),
                err
            );
        },
    }
}

pub(super) struct DeserializeError {
    inner: serde_path_to_error::Error<NvimDeserializeError>,
}

impl DeserializeError {
    pub(super) fn into_msg(self) -> DiagnosticMessage {
        let mut msg = DiagnosticMessage::new();
        msg.push_str("couldn't deserialize ");

        let segments = self.segments();

        if segments.len() == 0 {
            msg.push_str("object: ");
        } else {
            msg.push_dot_separated(
                segments.map(ToString::to_string),
                HighlightGroup::special(),
            );
            msg.push_str(": ");
        }

        match self.inner.inner() {
            NvimDeserializeError::Custom { msg: err } => {
                msg.push_str(err);
            },
            NvimDeserializeError::DuplicateField { field } => {
                msg.push_str("duplicate field '")
                    .push_str_highlighted(field, HighlightGroup::special())
                    .push_str("'");
            },
            NvimDeserializeError::MissingField { field } => {
                msg.push_str("missing field '")
                    .push_str_highlighted(field, HighlightGroup::special())
                    .push_str("'");
            },
            NvimDeserializeError::UnknownField { field, expected } => {
                msg.push_str("unknown field '")
                    .push_str_highlighted(field, HighlightGroup::special())
                    .push_str("', expected one of")
                    .push_comma_separated(
                        expected.iter(),
                        HighlightGroup::special(),
                    );
            },
            NvimDeserializeError::UnknownVariant { variant, expected } => {
                msg.push_str("unknown variant '")
                    .push_str_highlighted(variant, HighlightGroup::special())
                    .push_str("', expected one of")
                    .push_comma_separated(
                        expected.iter(),
                        HighlightGroup::special(),
                    );
            },
        }

        msg
    }

    fn segments(&self) -> impl ExactSizeIterator<Item = &impl fmt::Display> {
        self.inner.path().iter()
    }
}
