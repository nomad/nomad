use core::mem;
use core::ops::Deref;

use editor::{AccessMut, AgentId, Buffer, Editor};

use crate::Neovim;
use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{self, AutocmdId, Callbacks, Event, EventKind, Events};
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
    pub(crate) fn new(buffer_id: BufferId) -> Self {
        Self(api::opts::OptionOpts::builder().buffer(buffer_id.into()).build())
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

#[derive(Default)]
pub(crate) struct SetUneditableEolAgentIds {
    set_eol: AgentId,
    set_fix_eol: AgentId,
}

impl SetUneditableEolAgentIds {
    #[inline]
    pub(crate) fn set(&mut self, agent_id: AgentId) {
        debug_assert!(!agent_id.is_unknown());
        self.set_eol = agent_id;
        self.set_fix_eol = agent_id;
    }
}

impl Event for UneditableEndOfLine {
    type Args<'a> = (&'a NeovimBuffer<'a>, bool, bool, AgentId);
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
    fn register(
        &self,
        events: &Events,
        nvim: impl AccessMut<Neovim> + Clone + 'static,
    ) -> Self::RegisterOutput {
        enum Option {
            Binary,
            Eol,
            FixEol,
        }

        let on_option_set = |old_option_value: bool,
                             new_option_value: bool,
                             option: Option,
                             nvim: &mut Neovim| {
            let buffer_id = BufferId::of_focused();

            let opts = BufferLocalOpts::new(buffer_id);

            let value = |option_value: bool| match option {
                Option::Binary => UneditableEndOfLine::get_inner(
                    || EndOfLine.get(&opts),
                    || FixEndOfLine.get(&opts),
                    || option_value,
                ),
                Option::Eol => UneditableEndOfLine::get_inner(
                    || option_value,
                    || FixEndOfLine.get(&opts),
                    || Binary.get(&opts),
                ),
                Option::FixEol => UneditableEndOfLine::get_inner(
                    || EndOfLine.get(&opts),
                    || option_value,
                    || Binary.get(&opts),
                ),
            };

            let old_value = value(old_option_value);
            let new_value = value(new_option_value);

            let Some(callbacks) = nvim
                .events
                .on_uneditable_eol_set
                .as_ref()
                .map(|cbs| cbs.cloned())
            else {
                return true;
            };

            let ids = &mut nvim.events.agent_ids.set_uneditable_eol;

            let set_by = match option {
                Option::Eol => mem::take(&mut ids.set_eol),
                Option::FixEol => mem::take(&mut ids.set_fix_eol),
                Option::Binary => AgentId::UNKNOWN,
            };

            let Some(buffer) = nvim.buffer(buffer_id) else {
                let buffer = api::Buffer::from(buffer_id);
                tracing::error!(
                    buffer_name = ?buffer.get_name().ok(),
                    "UneditableEndOfLine triggered for an invalid buffer",
                );
                return true;
            };

            for callback in callbacks {
                callback((&buffer, old_value, new_value, set_by));
            }

            false
        };

        let eol_autocmd_id = events::OptionSet::<EndOfLine>::register_inner(
            events,
            nvim.clone(),
            move |is_buffer_local, &old_eol, &new_eol, nvim| {
                debug_assert!(is_buffer_local);
                on_option_set(old_eol, new_eol, Option::Eol, nvim)
            },
        );

        let fixeol_autocmd_id =
            events::OptionSet::<FixEndOfLine>::register_inner(
                events,
                nvim.clone(),
                move |is_buffer_local, &old_fix_eol, &new_fix_eol, nvim| {
                    debug_assert!(is_buffer_local);
                    on_option_set(
                        old_fix_eol,
                        new_fix_eol,
                        Option::FixEol,
                        nvim,
                    )
                },
            );

        let binary_autocmd_id = events::OptionSet::<Binary>::register_inner(
            events,
            nvim.clone(),
            move |is_buffer_local, &old_binary, &new_binary, nvim| {
                debug_assert!(is_buffer_local);
                on_option_set(old_binary, new_binary, Option::Binary, nvim)
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
