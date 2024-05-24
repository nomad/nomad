use core::fmt;

use nvim::serde::{Deserializer, Serializer};
use nvim::Object;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::{ChunkExt, ModuleName, WarningMsg};

/// TODO: docs
pub(crate) fn deserialize<T: DeserializeOwned>(
    object: Object,
    what: &'static str,
) -> Result<T, DeserializeError> {
    serde_path_to_error::deserialize(Deserializer::new(object))
        .map_err(|err| DeserializeError::new(err, what))
}

/// TODO: docs
pub(crate) fn serialize<T: Serialize>(
    value: &T,
    what: &'static str,
) -> Result<Object, SerializeError> {
    serde_path_to_error::serialize(value, Serializer::new())
        .map_err(|err| SerializeError::new(err, what))
}

/// TODO: docs
pub(crate) struct DeserializeError {
    module_name: Option<ModuleName>,
    inner: serde_path_to_error::Error<nvim::serde::DeserializeError>,
    what: &'static str,
}

impl DeserializeError {
    #[inline]
    fn new(
        err: serde_path_to_error::Error<nvim::serde::DeserializeError>,
        what: &'static str,
    ) -> Self {
        Self { module_name: None, inner: err, what }
    }

    #[inline]
    fn path(&self) -> Path<'_> {
        Path { err: self }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn set_module_name(&mut self, module: ModuleName) {
        self.module_name = Some(module);
    }
}

impl From<DeserializeError> for WarningMsg {
    #[inline]
    fn from(err: DeserializeError) -> WarningMsg {
        let mut msg = WarningMsg::new();

        msg.add("couldn't deserialize ");

        if err.path().is_empty() {
            msg.add(err.what);
        } else {
            msg.add(err.path().to_string().highlight());
        }

        msg.add(": ");

        use nvim::serde::DeserializeError::*;

        match err.inner.inner() {
            Custom { msg: err_msg } => {
                msg.add(err_msg.as_str());
            },

            DuplicateField { field } => {
                msg.add("duplicate field ").add(field.highlight());
            },

            MissingField { field } => {
                msg.add("missing field ").add(field.highlight());
            },

            UnknownField { field, expected } => {
                msg.add_invalid(field, expected.iter(), "field");
            },

            UnknownVariant { variant, expected } => {
                msg.add_invalid(variant, expected.iter(), "variant");
            },
        }

        msg
    }
}

/// TODO: docs
pub(crate) struct SerializeError {
    inner: serde_path_to_error::Error<nvim::serde::SerializeError>,
    what: &'static str,
}

impl SerializeError {
    #[inline]
    fn new(
        inner: serde_path_to_error::Error<nvim::serde::SerializeError>,
        what: &'static str,
    ) -> Self {
        Self { inner, what }
    }
}

impl From<SerializeError> for WarningMsg {
    #[inline]
    fn from(err: SerializeError) -> WarningMsg {
        let mut msg = WarningMsg::new();

        msg.add("couldn't serialize ");

        if err.inner.path().iter().len() == 0 {
            msg.add(err.what);
        } else {
            msg.add(err.inner.path().to_string().highlight());
        }

        msg.add(": ").add(err.inner.inner().msg.as_str());

        msg
    }
}

struct Path<'a> {
    err: &'a DeserializeError,
}

impl<'a> Path<'a> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn len(&self) -> usize {
        let has_module = self.err.module_name.is_some() as usize;
        let num_segments = self.err.inner.path().iter().len();
        let include_last = self.should_include_last_segment() as usize;
        has_module + num_segments.saturating_sub(include_last)
    }

    #[inline]
    fn segments(&self) -> impl Iterator<Item = Segment<'_>> + '_ {
        let path_segments = self.err.inner.path().iter();
        let num_segments = path_segments.len();
        let take = num_segments
            .saturating_sub(self.should_include_last_segment() as usize);

        self.err
            .module_name
            .map(Segment::Module)
            .into_iter()
            .chain(path_segments.map(Segment::Others).take(take))
    }

    #[inline]
    fn should_include_last_segment(&self) -> bool {
        matches!(
            self.err.inner.inner(),
            nvim::serde::DeserializeError::Custom { .. }
                | nvim::serde::DeserializeError::UnknownVariant { .. }
        )
    }
}

impl fmt::Display for Path<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut separator = "";

        for segment in self.segments() {
            write!(f, "{}{}", separator, segment)?;
            separator = ".";
        }

        Ok(())
    }
}

enum Segment<'a> {
    Module(ModuleName),
    Others(&'a serde_path_to_error::Segment),
}

impl fmt::Display for Segment<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Segment::Module(module) => write!(f, "{}", module),
            Segment::Others(segment) => write!(f, "{}", segment),
        }
    }
}
