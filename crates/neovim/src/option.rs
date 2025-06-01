use core::marker::PhantomData;
use core::ops::Deref;

use ed::Shared;
use ed::backend::{AgentId, Buffer};

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{
    AutocmdId,
    Callbacks,
    Event,
    EventKind,
    Events,
    EventsBorrow,
};
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
pub(crate) struct UneditableEndOfLine;

/// TODO: docs.
pub(crate) struct OptionSet<T>(PhantomData<T>);

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

impl<T: WatchedOption> OptionSet<T> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: NeovimOption> OptionSet<T> {
    #[inline]
    fn register_inner<F>(events: EventsBorrow, on_option_set: F) -> AutocmdId
    where
        F: Fn(
                Option<NeovimBuffer>,
                &T::Value,
                &T::Value,
                &Shared<Events>,
            ) -> bool
            + 'static,
    {
        let augroup_id = events.augroup_id;
        let bufs_state = events.borrow.buffers_state.clone();
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
                        &bufs_state,
                    )
                });

                let old_value = api::get_vvar::<T::Value>("option_old")
                    .expect("couldn't get option_old");

                let new_value = api::get_vvar::<T::Value>("option_new")
                    .expect("couldn't get option_new");

                on_option_set(buffer, &old_value, &new_value, &events)
            })
            .build();

        api::create_autocmd(["OptionSet"], &opts)
            .expect("couldn't create autocmd on OptionSet")
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
    type RegisterOutput = AutocmdId;

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
        Self::register_inner(events, |buffer, old_value, new_value, events| {
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
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}

impl Event for UneditableEndOfLine {
    type Args<'a> = (NeovimBuffer<'a>, bool, bool, AgentId);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = (AutocmdId, AutocmdId, AutocmdId);

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_uneditable_eol_set
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::UneditableEolSet(Self)
    }

    #[allow(clippy::too_many_lines)]
    #[inline]
    fn register(&self, mut events: EventsBorrow) -> Self::RegisterOutput {
        let on_option_set =
            |buffer: NeovimBuffer,
             old_value: bool,
             new_value: bool,
             events: &Shared<Events>| {
                let Some((callbacks, set_by)) = events.with_mut(|events| {
                    let callbacks = events.on_uneditable_eol_set.as_ref()?;
                    let set_by = events
                        .agent_ids
                        .set_uneditable_eol
                        .remove(&buffer.id())
                        .unwrap_or(AgentId::UNKNOWN);
                    Some((callbacks.cloned(), set_by))
                }) else {
                    return true;
                };

                for callback in callbacks {
                    callback((buffer, old_value, new_value, set_by));
                }

                false
            };

        let eol_autocmd_id = OptionSet::<EndOfLine>::register_inner(
            events.reborrow(),
            move |maybe_buffer, &old_eol, &new_eol, events| {
                let buffer = maybe_buffer.expect("'eol' is buffer-local");
                let opts = (&buffer).into();
                let fix_eol = FixEndOfLine.get(&opts);
                let binary = Binary.get(&opts);
                let old_value = UneditableEndOfLine::get_inner(
                    || old_eol,
                    || fix_eol,
                    || binary,
                );
                let new_value = UneditableEndOfLine::get_inner(
                    || new_eol,
                    || fix_eol,
                    || binary,
                );
                on_option_set(buffer, old_value, new_value, events)
            },
        );

        let fixeol_autocmd_id = OptionSet::<FixEndOfLine>::register_inner(
            events.reborrow(),
            move |maybe_buffer, &old_fix_eol, &new_fix_eol, events| {
                let buffer = maybe_buffer.expect("'fixeol' is buffer-local");
                let opts = (&buffer).into();
                let eol = EndOfLine.get(&opts);
                let binary = Binary.get(&opts);
                let old_value = UneditableEndOfLine::get_inner(
                    || eol,
                    || old_fix_eol,
                    || binary,
                );
                let new_value = UneditableEndOfLine::get_inner(
                    || eol,
                    || new_fix_eol,
                    || binary,
                );
                on_option_set(buffer, old_value, new_value, events)
            },
        );

        let binary_autocmd_id = OptionSet::<Binary>::register_inner(
            events.reborrow(),
            move |maybe_buffer, &old_binary, &new_binary, events| {
                let buffer = maybe_buffer.expect("'binary' is buffer-local");
                let opts = (&buffer).into();
                let eol = EndOfLine.get(&opts);
                let fix_eol = FixEndOfLine.get(&opts);
                let old_value = UneditableEndOfLine::get_inner(
                    || eol,
                    || fix_eol,
                    || old_binary,
                );
                let new_value = UneditableEndOfLine::get_inner(
                    || eol,
                    || fix_eol,
                    || new_binary,
                );
                on_option_set(buffer, old_value, new_value, events)
            },
        );

        (eol_autocmd_id, fixeol_autocmd_id, binary_autocmd_id)
    }

    #[inline]
    fn unregister(
        (
            eol_autocmd_id,
            fixeol_autocmd_id,
            binary_autocmd_id,
        ): Self::RegisterOutput,
    ) {
        let _ = api::del_autocmd(eol_autocmd_id);
        let _ = api::del_autocmd(fixeol_autocmd_id);
        let _ = api::del_autocmd(binary_autocmd_id);
    }
}
