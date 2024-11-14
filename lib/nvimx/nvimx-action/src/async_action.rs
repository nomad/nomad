use core::future::Future;

use nvimx_common::MaybeResult;
use nvimx_diagnostics::{DiagnosticSource, Level};

use crate::action_name::ActionName;
use crate::into_module_name::IntoModuleName;
use crate::Action;

/// TODO: docs
pub trait AsyncAction<M: IntoModuleName>: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args;

    /// TODO: docs
    type Ctx<'a>;

    /// TODO: docs
    type Docs;

    /// TODO: docs
    fn execute(
        &mut self,
        args: Self::Args,
        ctx: Self::Ctx<'_>,
    ) -> impl Future<Output = impl MaybeResult<()>>;

    /// TODO: docs
    fn docs(&self) -> Self::Docs;
}

impl<M, T> Action<M> for T
where
    M: IntoModuleName,
    T: AsyncAction<M> + Clone,
{
    const NAME: ActionName = T::NAME;
    type Args = T::Args;
    type Ctx<'a> = T::Ctx<'a>;
    type Docs = T::Docs;
    type Return = ();

    fn execute<'a>(&'a mut self, args: Self::Args, ctx: Self::Ctx<'a>) {
        todo!();
        // let mut this = self.clone();
        // ctx.spawn(|ctx| async move {
        //     if let Err(message) =
        //         this.execute(args, ctx).await.into_result().map_err(Into::into)
        //     {
        //         let mut source = DiagnosticSource::new();
        //         source.push_segment(M::NAME).push_segment(Self::NAME.as_str());
        //         message.emit(Level::Warning, source);
        //     }
        // })
        // .detach();
    }

    fn docs(&self) -> Self::Docs {
        self.docs()
    }
}
