use core::cell::Cell;

use neovim::buffer::HighlightRangeHandle;
use neovim::oxi::api;

use crate::editors::neovim::PeerHighlightGroup;

pub struct NeovimPeerSelection {
    pub(super) selection_highlight_handle: HighlightRangeHandle,
}

/// The highlight group used to highlight a remote peer's selection.
pub(super) struct PeerSelectionHighlightGroup;

impl PeerSelectionHighlightGroup {
    thread_local! {
        static GROUP_IDS: Cell<[u32; 16]> = const { Cell::new([0; _]) };
    }
}

impl PeerHighlightGroup for PeerSelectionHighlightGroup {
    const NAME_PREFIX: &str = "NomadCollabPeerSelection";

    fn set_hl_opts() -> api::opts::SetHighlightOpts {
        api::opts::SetHighlightOpts::builder().link("Visual").build()
    }

    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R {
        Self::GROUP_IDS.with(|ids| fun(ids.as_array_of_cells().as_slice()))
    }
}
