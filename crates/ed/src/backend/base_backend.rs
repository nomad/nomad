use abs_path::AbsPath;

use crate::backend::{AgentId, Backend};
use crate::{BorrowState, Context};

/// TODO: docs.
pub trait BaseBackend: Backend {
    /// TODO: docs.
    fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<impl AsMut<Self> + Backend, impl BorrowState>,
    ) -> impl Future<Output = Result<Self::BufferId, Self::CreateBufferError>>;
}
