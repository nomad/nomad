use nomad::ctx::BufferCtx;
use nomad::{BufferId, ByteOffset};

type Peer = String;

/// TODO: docs.
pub(super) struct PeerTooltip {
    at_offset: ByteOffset,
    in_buffer: BufferId,
    peer: Peer,
}

impl PeerTooltip {
    pub(super) fn create(
        peer: Peer,
        at_offset: ByteOffset,
        ctx: BufferCtx<'_>,
    ) -> Self {
        Self { at_offset, in_buffer: ctx.buffer_id(), peer }
    }

    /// The [`Peer`] this tooltip is for.
    pub(super) fn peer(&self) -> &Peer {
        &self.peer
    }

    /// # Panics
    ///
    /// Panics if the [`PeerTooltip`] was created in a different buffer.
    pub(super) fn relocate(
        &mut self,
        new_offset: ByteOffset,
        ctx: BufferCtx<'_>,
    ) {
        assert_eq!(
            self.in_buffer,
            ctx.buffer_id(),
            "relocating tooltip in wrong buffer"
        );
        todo!();
    }

    /// # Panics
    ///
    /// Panics if the [`PeerTooltip`] was created in a different buffer.
    pub(super) fn remove(self, ctx: BufferCtx<'_>) {
        assert_eq!(
            self.in_buffer,
            ctx.buffer_id(),
            "removing tooltip from wrong buffer"
        );
        todo!();
    }
}
