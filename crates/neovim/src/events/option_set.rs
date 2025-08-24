use core::marker::PhantomData;

use editor::AccessMut;

use crate::Neovim;
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::option::NeovimOption;
use crate::oxi::{self, api};
use crate::utils::CallbackExt;

/// TODO: docs.
pub(crate) trait WatchedOption: NeovimOption {
    fn callbacks(
        events: &mut Events,
    ) -> &mut Option<Callbacks<OptionSet<Self>>>;

    fn event_kind() -> EventKind;
}

/// TODO: docs.
pub(crate) struct OptionSet<T>(PhantomData<T>);

impl<T: NeovimOption> OptionSet<T> {
    #[inline]
    pub(crate) fn register_inner<F>(
        events: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
        on_option_set: F,
    ) -> AutocmdId
    where
        F: Fn(bool, &T::Value, &T::Value, &mut Neovim) -> bool + 'static,
    {
        let callback = (move |_: api::types::AutocmdCallbackArgs| {
            let is_local = api::get_vvar::<oxi::String>("option_type")
                .expect("couldn't get option_type")
                == "local";

            let old_value = api::get_vvar::<T::Value>("option_old")
                .expect("couldn't get option_old");

            let new_value = api::get_vvar::<T::Value>("option_new")
                .expect("couldn't get option_new");

            nvim.with_mut(|nvim| {
                on_option_set(is_local, &old_value, &new_value, nvim)
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::create_autocmd(
            ["OptionSet"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .patterns([T::LONG_NAME])
                .callback(callback)
                .build(),
        )
        .expect("couldn't create autocmd on OptionSet")
    }
}

impl<T: WatchedOption> Event for OptionSet<T> {
    /// A tuple of `(is_local, old_value, new_value)`.
    type Args<'a> = (bool, &'a T::Value, &'a T::Value);
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
    fn register(
        &self,
        events: &Events,
        nvim: impl AccessMut<Neovim> + 'static,
    ) -> Self::RegisterOutput {
        Self::register_inner(
            events,
            nvim,
            |is_local, old_value, new_value, nvim| {
                let Some(callbacks) = T::callbacks(&mut nvim.events) else {
                    return true;
                };

                for callback in callbacks.cloned() {
                    callback((is_local, old_value, new_value));
                }

                false
            },
        )
    }

    #[inline]
    fn unregister(autocmd_id: Self::RegisterOutput) {
        let _ = api::del_autocmd(autocmd_id);
    }
}
