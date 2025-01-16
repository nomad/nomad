//! TODO: docs.

use core::fmt;

use nvimx_core::notify::{self, Namespace};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::oxi;
use crate::value::NeovimValue;

/// TODO: docs.
#[derive(Debug)]
pub struct NeovimSerializeError {
    inner: serde_path_to_error::Error<oxi::serde::SerializeError>,
}

/// TODO: docs.
#[derive(Debug)]
pub struct NeovimDeserializeError {
    inner: serde_path_to_error::Error<oxi::serde::DeserializeError>,
    config_path: Option<Namespace>,
}

struct Path<'a> {
    inner: &'a serde_path_to_error::Path,
    config_path: Option<&'a Namespace>,
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
pub(crate) fn deserialize<'de, T: Deserialize<'de>>(
    value: NeovimValue,
) -> Result<T, NeovimDeserializeError> {
    serde_path_to_error::deserialize(oxi::serde::Deserializer::new(
        value.into_inner(),
    ))
    .map_err(|inner| NeovimDeserializeError { inner, config_path: None })
}

impl NeovimSerializeError {
    #[inline]
    fn path(&self) -> Path<'_> {
        Path { inner: self.inner.path(), config_path: None }
    }
}

impl NeovimDeserializeError {
    #[inline]
    pub(crate) fn set_config_path(&mut self, config_path: Namespace) {
        self.config_path = Some(config_path);
    }

    #[inline]
    fn path(&self) -> Path<'_> {
        Path {
            inner: self.inner.path(),
            config_path: self.config_path.as_ref(),
        }
    }
}

impl Path<'_> {
    /// If the path is not empty, pushes " at {self}" to the given message.
    #[inline]
    pub(crate) fn push_at(&self, message: &mut notify::Message) {
        if !self.is_empty() {
            message.push_str(" at ").push_info(self.to_string());
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.inner.iter().len() == 0
            && self
                .config_path
                .map(|path| {
                    // The first name is the plugin's, which we don't display.
                    path.names().len() <= 1
                })
                .unwrap_or(true)
    }
}

impl notify::Error for NeovimSerializeError {
    #[inline]
    fn to_message(
        &self,
        _: &notify::Namespace,
    ) -> (notify::Level, notify::Message) {
        let mut message = notify::Message::new();
        message
            .push_str("couldn't serialize value")
            .push_with(|message| self.path().push_at(message))
            .push_str(": ")
            .push_str(self.inner.inner().to_string());
        (notify::Level::Error, message)
    }
}

impl notify::Error for NeovimDeserializeError {
    #[allow(clippy::too_many_lines)]
    #[inline]
    fn to_message(
        &self,
        _: &notify::Namespace,
    ) -> (notify::Level, notify::Message) {
        let mut message = notify::Message::new();

        message
            .push_str("couldn't deserialize ")
            .push_str(if self.config_path.is_some() {
                "config"
            } else {
                "value"
            })
            .push_with(|message| self.path().push_at(message))
            .push_str(": ");

        let (actual, &expected) = match self.inner.inner() {
            oxi::serde::DeserializeError::Custom { msg } => {
                message.push_str(msg);
                return (notify::Level::Error, message);
            },
            oxi::serde::DeserializeError::DuplicateField { field } => {
                message.push_str("duplicate field ").push_info(field);
                return (notify::Level::Error, message);
            },
            oxi::serde::DeserializeError::MissingField { field } => {
                message.push_str("missing field ").push_info(field);
                return (notify::Level::Error, message);
            },
            oxi::serde::DeserializeError::UnknownField { field, expected } => {
                message
                    .push_str("invalid field ")
                    .push_invalid(field)
                    .push_str(", ");
                (field, expected)
            },
            oxi::serde::DeserializeError::UnknownVariant {
                variant,
                expected,
            } => {
                message
                    .push_str("invalid variant ")
                    .push_invalid(variant)
                    .push_str(", ");
                (variant, expected)
            },
        };

        let levenshtein_threshold = 2;

        let mut guesses = expected
            .iter()
            .map(|candidate| {
                let distance = strsim::levenshtein(candidate, actual);
                (candidate, distance)
            })
            .filter(|&(_, distance)| distance <= levenshtein_threshold)
            .collect::<SmallVec<[_; 2]>>();

        guesses.sort_by_key(|&(_, distance)| distance);

        if let Some((best_guess, _)) = guesses.first() {
            message
                .push_str("did you mean ")
                .push_expected(best_guess)
                .push_str("?");
        } else {
            message
                .push_str("expected one of ")
                .push_comma_separated(expected, notify::SpanKind::Expected);
        }

        (notify::Level::Error, message)
    }
}

impl fmt::Display for Path<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(config_path) = self.config_path {
            let mut names = config_path.names().peekable();
            // The first name is the plugin's.
            names.next();
            loop {
                let Some(name) = names.next() else { break };
                f.write_str(name)?;
                if names.peek().is_some() {
                    f.write_str(".")?;
                }
            }
        }
        if self.inner.iter().len() > 0 {
            write!(f, ".{}", self.inner)?;
        }
        Ok(())
    }
}
