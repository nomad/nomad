use editor::{AccessMut, AgentId, Editor, Shared};

use crate::buffer::{BufferId, NeovimBuffer};
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::option::{
    Binary,
    BufferLocalOpts,
    EndOfLine,
    FixEndOfLine,
    NeovimOption,
    UneditableEndOfLine,
};
use crate::oxi::api;
use crate::{Neovim, events};

#[derive(Clone, Copy)]
pub(crate) struct SetUneditableEndOfLine;

#[derive(Clone, Debug, Default)]
pub(crate) struct SetUneditableEolAgentIds {
    set_eol: Shared<AgentId>,
    set_fix_eol: Shared<AgentId>,
}

impl SetUneditableEolAgentIds {
    #[inline]
    pub(crate) fn set(&self, agent_id: AgentId) {
        debug_assert!(!agent_id.is_unknown());
        self.set_eol.set(agent_id);
        self.set_fix_eol.set(agent_id);
    }
}

impl Event for SetUneditableEndOfLine {
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
            let buffer = api::Buffer::current();

            let opts = BufferLocalOpts::new(buffer.clone());

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
                Option::Eol => ids.set_eol.take(),
                Option::FixEol => ids.set_fix_eol.take(),
                Option::Binary => AgentId::UNKNOWN,
            };

            let buffer_id = BufferId::from(buffer.clone());

            let Some(mut buffer) = nvim.buffer(buffer_id) else {
                let buffer = api::Buffer::from(buffer_id);
                tracing::error!(
                    buffer_name = ?buffer.get_name().ok(),
                    "UneditableEndOfLine triggered for an invalid buffer",
                );
                return true;
            };

            for callback in callbacks {
                callback((buffer.reborrow(), old_value, new_value, set_by));
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
