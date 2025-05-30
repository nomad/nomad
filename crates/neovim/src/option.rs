use core::marker::PhantomData;
use core::ops::Deref;

use ed::backend::Buffer;

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{Callbacks, Event, EventKind, Events, EventsBorrow};
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

/// TODO: docs.
pub(crate) trait WatchedOption: NeovimOption {
    fn callbacks(
        events: &mut Events,
    ) -> &mut Option<Callbacks<OptionSet<Self>>>;

    fn event_kind() -> EventKind;
}

/// The "binary" option.
pub(crate) struct Binary;

/// The "endofline" option.
pub(crate) struct EndOfLine;

/// The "fixendofline" option.
pub(crate) struct FixEndOfLine;

/// TODO: docs.
pub(crate) struct OptionSet<T>(PhantomData<T>);

/// The [`Opts`](NeovimOption::Opts) for all buffer-local options.
pub(crate) struct BufferLocalOpts(api::opts::OptionOpts);

impl<T: WatchedOption> OptionSet<T> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self(PhantomData)
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

impl WatchedOption for EndOfLine {
    #[inline]
    fn callbacks(
        events: &mut Events,
    ) -> &mut Option<Callbacks<OptionSet<Self>>> {
        &mut events.on_end_of_line_set
    }

    #[inline]
    fn event_kind() -> EventKind {
        EventKind::EndOfLineSet(OptionSet::<Self>::new())
    }
}

impl WatchedOption for FixEndOfLine {
    #[inline]
    fn callbacks(
        events: &mut Events,
    ) -> &mut Option<Callbacks<OptionSet<Self>>> {
        &mut events.on_fix_end_of_line_set
    }

    #[inline]
    fn event_kind() -> EventKind {
        EventKind::FixEndOfLineSet(OptionSet::<Self>::new())
    }
}

impl From<&NeovimBuffer<'_>> for BufferLocalOpts {
    #[inline]
    fn from(buf: &NeovimBuffer) -> Self {
        Self(api::opts::OptionOpts::builder().buffer(buf.id().into()).build())
    }
}

impl Deref for BufferLocalOpts {
    type Target = api::opts::OptionOpts;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: WatchedOption> Event for OptionSet<T> {
    /// A tuple of `(buffer, old_value, new_value)`, where `buffer` is only
    /// present for buffer-local options.
    type Args<'a> = (Option<NeovimBuffer<'a>>, &'a T::Value, &'a T::Value);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = u32;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        T::callbacks(events)
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn kind(&self) -> EventKind {
        T::event_kind()
    }

    #[inline]
    fn register(&self, events: EventsBorrow) -> Self::RegisterOutput {
        let augroup_id = events.augroup_id;

        let buf_fields = events.borrow.buffer_fields.clone();
        let events = events.handle;

        let opts = api::opts::CreateAutocmdOpts::builder()
            .group(augroup_id)
            .patterns([T::LONG_NAME])
            .callback(move |_: api::types::AutocmdCallbackArgs| {
                let is_local = api::get_vvar::<oxi::String>("option_type")
                    .expect("couldn't get option_type")
                    == "local";

                let buffer = is_local.then(|| {
                    Events::buffer(
                        BufferId::of_focused(),
                        &events,
                        &buf_fields,
                    )
                });

                let old_value = api::get_vvar::<T::Value>("option_old")
                    .expect("couldn't get option_old");

                let new_value = api::get_vvar::<T::Value>("option_new")
                    .expect("couldn't get option_new");

                let Some(callbacks) = events.with_mut(|ev| {
                    T::callbacks(ev).as_ref().map(Callbacks::cloned)
                }) else {
                    return true;
                };

                for callback in callbacks {
                    callback((buffer, &old_value, &new_value));
                }

                false
            })
            .build();

        api::create_autocmd(["OptionSet"], &opts)
            .expect("couldn't create autocmd on OptionSet")
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
