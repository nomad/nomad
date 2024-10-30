use crate::command_args::CommandArgs;
use crate::ctx::NeovimCtx;
use crate::diagnostics::{DiagnosticMessage, DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::{Action, Module};

/// TODO: docs.
pub trait Command:
    Action<
    Args: Clone
              + for<'a> TryFrom<
        &'a mut CommandArgs,
        Error: Into<DiagnosticMessage>,
    >,
    Return = (),
>
{
    /// TODO: docs.
    fn into_callback(
        self,
    ) -> impl FnMut(CommandArgs, NeovimCtx<'static>) + 'static;
}

impl<T> Command for T
where
    T: Action<
        Args: Clone
                  + for<'a> TryFrom<
            &'a mut CommandArgs,
            Error: Into<DiagnosticMessage>,
        >,
        Return = (),
    >,
{
    fn into_callback(
        mut self,
    ) -> impl FnMut(CommandArgs, NeovimCtx<'static>) + 'static {
        Box::new(move |mut args, ctx| {
            let args = match T::Args::try_from(&mut args) {
                Ok(args) => args,
                Err(err) => {
                    let mut source = DiagnosticSource::new();
                    source
                        .push_segment(T::Module::NAME.as_str())
                        .push_segment(T::NAME.as_str());
                    err.into().emit(Level::Error, source);
                    return;
                },
            };
            if let Err(err) = self.execute(args, ctx).into_result() {
                let mut source = DiagnosticSource::new();
                source
                    .push_segment(T::Module::NAME.as_str())
                    .push_segment(T::NAME.as_str());
                err.into().emit(Level::Error, source);
            }
        })
    }
}
