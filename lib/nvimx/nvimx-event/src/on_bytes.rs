use nvimx_common::MaybeResult;
pub use nvimx_ctx::OnBytesArgs;
use nvimx_ctx::{RegisterOnBytesArgs, ShouldDetach, TextBufferCtx};
use nvimx_plugin::{Action, Module};

use crate::Event;

/// TODO: docs.
pub struct OnBytes<A> {
    action: A,
}

impl<A> OnBytes<A> {
    /// Creates a new [`OnBytes`] with the given action.
    pub fn new(action: A) -> Self {
        Self { action }
    }
}

impl<A> Event for OnBytes<A>
where
    A: for<'ctx> Action<Args = OnBytesArgs, Ctx<'ctx> = TextBufferCtx<'ctx>>,
    A::Return: Into<ShouldDetach>,
{
    type Ctx<'ctx> = TextBufferCtx<'ctx>;

    fn register(mut self, ctx: Self::Ctx<'_>) {
        let callback = move |args, ctx: TextBufferCtx<'_>| {
            self.action
                .execute(args, ctx)
                .into_result()
                .map(Into::into)
                .map_err(Into::into)
        };
        let args = RegisterOnBytesArgs {
            callback,
            module_name: Some(A::Module::NAME.as_str()),
            callback_name: Some(A::NAME.as_str()),
        };
        ctx.register_on_bytes(args);
    }
}
