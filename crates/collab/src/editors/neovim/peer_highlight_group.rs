use core::any;
use core::cell::Cell;

use collab_types::PeerId;
use compact_str::format_compact;
use neovim::oxi::api;

/// A trait implemented by types that represent highlight groups used to
/// highlight a piece of UI (like a cursor or selection) that belongs to a
/// remote peer.
pub(super) trait PeerHighlightGroup {
    /// The prefix of each highlight group name.
    const NAME_PREFIX: &'static str;

    /// Returns the `opts` to pass to [`api::set_hl`] when creating the
    /// highlight group.
    fn set_hl_opts() -> api::opts::SetHighlightOpts;

    /// Calls the given function with the (possibly uninitialized) highlight
    /// group IDs used by this type.
    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R;

    #[track_caller]
    fn create_all() {
        debug_assert!(
            Self::with_group_ids(|ids| ids.iter().all(|id| id.get() == 0)),
            "{}::create_all() has already been called",
            any::type_name::<Self>()
        );

        Self::with_group_ids(|group_ids| {
            for (group_idx, group_id) in group_ids.iter().enumerate() {
                group_id.set(Self::create(group_idx));
            }
        });
    }

    /// Returns the highlight group ID to use to highlight UI belonging to
    /// the peer with the given ID.
    #[track_caller]
    fn new(peer_id: PeerId) -> impl api::SetExtmarkHlGroup {
        Self::with_group_ids(|group_ids| {
            let group_idx = peer_id.into_u64().saturating_sub(1) as usize
                % group_ids.len();

            let group_id = group_ids[group_idx].get();

            debug_assert!(
                group_id > 0,
                "{}::create_all() has not been called",
                any::type_name::<Self>()
            );

            i64::from(group_id)
        })
    }

    #[doc(hidden)]
    fn create(group_idx: usize) -> u32 {
        let name = format_compact!("{}{}", Self::NAME_PREFIX, group_idx + 1);

        api::set_hl(0, name.as_ref(), &Self::set_hl_opts())
            .expect("couldn't create highlight group");

        api::get_hl_id_by_name(name.as_ref())
            .expect("couldn't get highlight group ID")
    }
}
