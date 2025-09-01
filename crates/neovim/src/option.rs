use core::ops::Deref;

use crate::buffer::BufferId;
use crate::events::{AugroupId, AutocmdId};
use crate::oxi::{self, api};
use crate::utils::CallbackExt;

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
    #[allow(dead_code)]
    #[track_caller]
    #[inline]
    fn set(&mut self, value: Self::Value, opts: &Self::Opts) {
        if let Err(err) = api::set_option_value(Self::LONG_NAME, value, opts) {
            panic!("couldn't set option {:?}: {err}", Self::LONG_NAME);
        }
    }

    /// TODO: docs.
    #[inline]
    fn on_set(
        augroup_id: AugroupId,
        buffer_id: BufferId,
        mut fun: impl FnMut(api::Buffer, Self::Value, Self::Value) -> bool
        + 'static,
    ) -> AutocmdId {
        let on_option_set = (move |args: api::types::AutocmdCallbackArgs| {
            // Don't call the function if the autocmd is triggered for a
            // different buffer.
            api::Buffer::from(buffer_id) == args.buffer
                && fun(args.buffer, Self::old_value(), Self::new_value())
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::create_autocmd(
            ["OptionSet"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(augroup_id)
                .patterns([Self::LONG_NAME])
                .callback(on_option_set)
                .build(),
        )
        .expect("couldn't create autocmd on OptionSet")
    }

    fn old_value() -> Self::Value {
        api::get_vvar("option_old").expect("couldn't get option_old")
    }

    fn new_value() -> Self::Value {
        api::get_vvar("option_old").expect("couldn't get option_old")
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
    fn get_inner(
        eol: impl FnOnce() -> bool,
        fix_eol: impl FnOnce() -> bool,
        binary: impl FnOnce() -> bool,
    ) -> bool {
        eol() || (fix_eol() && !binary())
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

    #[inline]
    fn on_set(
        augroup_id: AugroupId,
        buffer_id: BufferId,
        mut fun: impl FnMut(api::Buffer, Self::Value, Self::Value) -> bool
        + 'static,
    ) -> AutocmdId {
        let on_option_set = (move |args: api::types::AutocmdCallbackArgs| {
            // Don't call the function if the autocmd is triggered for a
            // different buffer.
            if api::Buffer::from(buffer_id) != args.buffer {
                return false;
            }

            enum Option {
                Binary,
                Eol,
                FixEol,
            }

            let option = match &*args.r#match {
                EndOfLine::LONG_NAME => Option::Eol,
                FixEndOfLine::LONG_NAME => Option::FixEol,
                Binary::LONG_NAME => Option::Binary,
                other => panic!("unexpected option name: {other}"),
            };

            let opts = BufferLocalOpts::new(args.buffer.clone());

            let option = |option_value: bool| match option {
                Option::Binary => Self::get_inner(
                    || EndOfLine.get(&opts),
                    || FixEndOfLine.get(&opts),
                    || option_value,
                ),
                Option::Eol => Self::get_inner(
                    || option_value,
                    || FixEndOfLine.get(&opts),
                    || Binary.get(&opts),
                ),
                Option::FixEol => Self::get_inner(
                    || EndOfLine.get(&opts),
                    || option_value,
                    || Binary.get(&opts),
                ),
            };

            fun(
                args.buffer,
                option(Self::old_value()),
                option(Self::new_value()),
            )
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::create_autocmd(
            ["OptionSet"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(augroup_id)
                .patterns([
                    EndOfLine::LONG_NAME,
                    FixEndOfLine::LONG_NAME,
                    Binary::LONG_NAME,
                ])
                .callback(on_option_set)
                .build(),
        )
        .expect("couldn't create autocmd on OptionSet")
    }
}

impl Deref for BufferLocalOpts {
    type Target = api::opts::OptionOpts;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
