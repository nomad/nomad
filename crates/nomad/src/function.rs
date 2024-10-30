use nvim_oxi::Object as NvimObject;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::ctx::NeovimCtx;
use crate::diagnostics::{DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::{Action, Module};

/// TODO: docs.
pub trait Function:
    for<'ctx> Action<NeovimCtx<'ctx>, Args: DeserializeOwned, Return: Serialize>
{
    /// TODO: docs.
    fn into_callback(
        self,
    ) -> impl for<'ctx> FnMut(NvimObject, NeovimCtx<'ctx>) -> NvimObject;
}

impl<T> Function for T
where
    T: for<'ctx> Action<
        NeovimCtx<'ctx>,
        Args: DeserializeOwned,
        Return: Serialize,
    >,
{
    fn into_callback(
        mut self,
    ) -> impl for<'ctx> FnMut(NvimObject, NeovimCtx<'ctx>) -> NvimObject {
        move |args, ctx| {
            let args = match crate::serde::deserialize(args) {
                Ok(args) => args,
                Err(err) => {
                    let mut source = DiagnosticSource::new();
                    source
                        .push_segment(T::Module::NAME.as_str())
                        .push_segment(T::NAME.as_str());
                    err.into_msg().emit(Level::Warning, source);
                    return NvimObject::nil();
                },
            };
            let ret = match self.execute(args, ctx).into_result() {
                Ok(ret) => ret,
                Err(err) => {
                    let mut source = DiagnosticSource::new();
                    source
                        .push_segment(T::Module::NAME.as_str())
                        .push_segment(T::NAME.as_str());
                    err.into().emit(Level::Warning, source);
                    return NvimObject::nil();
                },
            };
            crate::serde::serialize(&ret)
        }
    }
}
