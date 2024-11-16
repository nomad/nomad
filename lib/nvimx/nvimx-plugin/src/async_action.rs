use core::future::Future;

use nvimx_common::MaybeResult;
use nvimx_ctx::NeovimCtx;
use nvimx_diagnostics::{DiagnosticSource, Level};

use crate::{Action, ActionName, Module};

/// TODO: docs
pub trait AsyncAction: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args: 'static;

    /// TODO: docs
    type Docs;

    /// TODO: docs
    type Module: Module;

    /// TODO: docs
    fn execute(
        &mut self,
        args: Self::Args,
        ctx: NeovimCtx<'_>,
    ) -> impl Future<Output = impl MaybeResult<()>>;

    /// TODO: docs
    fn docs(&self) -> Self::Docs;
}

impl<T: AsyncAction + Clone> Action for T {
    const NAME: ActionName = T::NAME;
    type Args = T::Args;
    type Ctx<'a> = NeovimCtx<'a>;
    type Docs = T::Docs;
    type Module = T::Module;
    type Return = ();

    fn execute<'a>(&'a mut self, args: Self::Args, ctx: NeovimCtx<'a>) {
        let mut this = self.clone();
        ctx.spawn(|ctx| async move {
            if let Err(message) =
                this.execute(args, ctx).await.into_result().map_err(Into::into)
            {
                let mut source = DiagnosticSource::new();
                source
                    .push_segment(Self::Module::NAME.as_str())
                    .push_segment(Self::NAME.as_str());
                message.emit(Level::Warning, source);
            }
        })
        .detach();
    }

    fn docs(&self) -> Self::Docs {
        self.docs()
    }
}
