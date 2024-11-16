use nvimx_common::MaybeResult;

use crate::action_name::ActionName;
use crate::module::Module;

/// TODO: docs
pub trait Action: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args;

    /// TODO: docs
    type Ctx<'a>;

    /// TODO: docs
    type Docs;

    /// TODO: docs
    type Module: Module;

    /// TODO: docs
    //
    // NOTE: remove once we have RTN
    // (https://github.com/rust-lang/rust/issues/109417).
    type Return;

    /// TODO: docs
    fn execute<'a>(
        &'a mut self,
        args: Self::Args,
        ctx: Self::Ctx<'a>,
    ) -> impl MaybeResult<Self::Return>;

    /// TODO: docs
    fn docs(&self) -> Self::Docs;
}
