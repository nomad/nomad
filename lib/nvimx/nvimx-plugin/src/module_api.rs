use core::marker::PhantomData;

use nvimx_common::{oxi, MaybeResult};
use nvimx_ctx::NeovimCtx;
use nvimx_diagnostics::{DiagnosticSource, Level};

use crate::module_subcommands::ModuleSubCommands;
use crate::{Function, Module, SubCommand};

/// TODO: docs.
pub struct ModuleApi<M: Module> {
    pub(crate) dictionary: oxi::Dictionary,
    pub(crate) commands: ModuleSubCommands,
    ty: PhantomData<M>,
}

impl<M: Module> ModuleApi<M> {
    /// TODO: docs.
    pub fn default_subcommand<T>(mut self, command: T) -> Self
    where
        T: SubCommand<Module = M>,
    {
        self.commands.add_default_subcommand(command);
        self
    }

    /// TODO: docs.
    pub fn function<T>(mut self, function: T) -> Self
    where
        T: Function<Module = M>,
    {
        if self.dictionary.get(T::NAME.as_str()).is_some() {
            panic!(
                "a function with the name '{}' has already been added to the \
                 API for module '{}'",
                T::NAME,
                M::NAME,
            );
        }
        let ctx = self.neovim_ctx().to_static();
        let mut callback = callback_of_function(function);
        self.dictionary.insert(
            T::NAME.as_str(),
            oxi::Function::from_fn_mut(move |obj| {
                callback(obj, ctx.reborrow())
            }),
        );
        self
    }

    /// Creates a new [`ModuleApi`].
    pub fn new(neovim_ctx: NeovimCtx<'static>) -> Self {
        Self {
            dictionary: oxi::Dictionary::default(),
            commands: ModuleSubCommands::new::<M>(neovim_ctx),
            ty: PhantomData,
        }
    }

    /// TODO: docs.
    pub fn subcommand<T>(mut self, command: T) -> Self
    where
        T: SubCommand<Module = M>,
    {
        self.commands.add_subcommand(command);
        self
    }

    fn neovim_ctx(&self) -> NeovimCtx<'_> {
        self.commands.neovim_ctx.reborrow()
    }
}

fn callback_of_function<T: Function>(
    mut function: T,
) -> impl for<'ctx> FnMut(oxi::Object, NeovimCtx<'ctx>) -> oxi::Object {
    move |args, ctx| {
        let args = match crate::serde::deserialize(args) {
            Ok(args) => args,
            Err(err) => {
                let mut source = DiagnosticSource::new();
                source
                    .push_segment(T::Module::NAME.as_str())
                    .push_segment(T::NAME.as_str());
                err.into_msg().emit(Level::Warning, source);
                return oxi::Object::nil();
            },
        };
        let ret = match function.execute(args, ctx).into_result() {
            Ok(ret) => ret,
            Err(err) => {
                let mut source = DiagnosticSource::new();
                source
                    .push_segment(T::Module::NAME.as_str())
                    .push_segment(T::NAME.as_str());
                err.into().emit(Level::Warning, source);
                return oxi::Object::nil();
            },
        };
        crate::serde::serialize(&ret)
    }
}
