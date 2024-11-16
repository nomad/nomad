use nvimx_common::MaybeResult;
use nvimx_ctx::NeovimCtx;
use nvimx_diagnostics::DiagnosticMessage;

use crate::action_name::ActionName;
use crate::subcommand_args::SubCommandArgs;
use crate::{Action, Module};

/// TODO: docs
pub trait SubCommand: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args: Clone
        + for<'args> TryFrom<
            &'args mut SubCommandArgs,
            Error: Into<DiagnosticMessage>,
        >;

    /// TODO: docs
    type Docs;

    /// TODO: docs
    type Module: Module;

    /// TODO: docs
    fn execute<'a>(
        &'a mut self,
        args: Self::Args,
        ctx: NeovimCtx<'a>,
    ) -> impl MaybeResult<()>;

    /// TODO: docs
    fn docs(&self) -> Self::Docs;
}

impl<A> SubCommand for A
where
    A: for<'a> Action<Ctx<'a> = NeovimCtx<'a>, Return = ()>,
    A::Args: Clone
        + for<'args> TryFrom<
            &'args mut SubCommandArgs,
            Error: Into<DiagnosticMessage>,
        >,
{
    const NAME: ActionName = A::NAME;
    type Args = A::Args;
    type Docs = A::Docs;
    type Module = A::Module;

    fn execute<'a>(
        &'a mut self,
        args: Self::Args,
        ctx: NeovimCtx<'a>,
    ) -> impl MaybeResult<()> {
        A::execute(self, args, ctx)
    }

    fn docs(&self) -> Self::Docs {
        A::docs(self)
    }
}
