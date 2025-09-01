use core::ops::Deref;

use crate::buffer::BufferId;
use crate::events::{AugroupId, AutocmdId};
use crate::oxi::{self, api};

/// TODO: docs.
pub(crate) trait NeovimOption: 'static + Sized {
    /// TODO: docs.
    const LONG_NAME: &'static str;

    /// TODO: docs.
    type Value: oxi::conversion::ToObject + oxi::conversion::FromObject;

    /// TODO: docs.
    type Opts: ?Sized + Deref<Target = api::opts::OptionOpts>;

    /// TODO: docs.
    #[track_caller]
    #[inline]
    fn get(&self, opts: &Self::Opts) -> Self::Value {
        match api::get_option_value(Self::LONG_NAME, opts) {
            Ok(value) => value,
            Err(err) => {
                panic!("couldn't get option {:?}: {err}", Self::LONG_NAME)
            },
        }
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    fn set(&mut self, value: Self::Value, opts: &Self::Opts) {
        if let Err(err) = api::set_option_value(Self::LONG_NAME, value, opts) {
            panic!("couldn't set option {:?}: {err}", Self::LONG_NAME);
        }
    }
}

/// The "binary" option.
pub(crate) struct Binary;

/// The "endofline" option.
pub(crate) struct EndOfLine;

/// The "fixendofline" option.
pub(crate) struct FixEndOfLine;

/// TODO: docs.
pub(crate) struct UneditableEndOfLine;

/// The [`Opts`](NeovimOption::Opts) for all buffer-local options.
pub(crate) struct BufferLocalOpts(api::opts::OptionOpts);

impl UneditableEndOfLine {
    #[inline]
    pub(crate) fn get_inner(
        eol: impl FnOnce() -> bool,
        fix_eol: impl FnOnce() -> bool,
        binary: impl FnOnce() -> bool,
    ) -> bool {
        eol() || (fix_eol() && !binary())
    }

    #[inline]
    pub(crate) fn on_set_on(
        _buffer_id: BufferId,
        _augroup: AugroupId,
        _fun: impl FnMut(api::Buffer, bool, bool) -> bool + 'static,
    ) -> (AutocmdId, AutocmdId, AutocmdId) {
        todo!();
    }
}

impl BufferLocalOpts {
    #[inline]
    pub(crate) fn new(buffer: api::Buffer) -> Self {
        Self(api::opts::OptionOpts::builder().buf(buffer).build())
    }
}

impl NeovimOption for Binary {
    const LONG_NAME: &'static str = "binary";
    type Value = bool;
    type Opts = BufferLocalOpts;
}

impl NeovimOption for EndOfLine {
    const LONG_NAME: &'static str = "endofline";
    type Value = bool;
    type Opts = BufferLocalOpts;
}

impl NeovimOption for FixEndOfLine {
    const LONG_NAME: &'static str = "fixendofline";
    type Value = bool;
    type Opts = BufferLocalOpts;
}

impl NeovimOption for UneditableEndOfLine {
    const LONG_NAME: &'static str = unimplemented!();
    type Value = bool;
    type Opts = BufferLocalOpts;

    #[inline]
    fn get(&self, opts: &Self::Opts) -> Self::Value {
        Self::get_inner(
            || EndOfLine.get(opts),
            || FixEndOfLine.get(opts),
            || Binary.get(opts),
        )
    }

    #[inline]
    fn set(&mut self, value: Self::Value, opts: &Self::Opts) {
        if value {
            EndOfLine.set(true, opts);
        } else {
            EndOfLine.set(false, opts);
            FixEndOfLine.set(false, opts);
        }
    }
}

impl Deref for BufferLocalOpts {
    type Target = api::opts::OptionOpts;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
