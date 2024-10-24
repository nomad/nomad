use core::marker::PhantomData;

use nvim_oxi::{Dictionary as NvimDictionary, Function as NvimFunction};

use crate::ctx::NeovimCtx;
use crate::module_commands::ModuleCommands;
use crate::{Command, Event, Function, Module};

/// TODO: docs.
pub struct ModuleApi<M: Module> {
    pub(crate) dictionary: NvimDictionary,
    pub(crate) commands: ModuleCommands,
    neovim_ctx: NeovimCtx<'static>,
    ty: PhantomData<M>,
}

impl<M: Module> ModuleApi<M> {
    /// TODO: docs.
    pub fn command<T>(mut self, command: T) -> Self
    where
        T: Command<Module = M>,
    {
        self.commands.add_command(command);
        self
    }

    /// TODO: docs.
    pub fn event<T>(self, event: T) -> Self
    where
        T: for<'a> Event<Ctx<'a> = NeovimCtx<'a>>,
    {
        event.register(self.neovim_ctx.reborrow());
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
        self.dictionary.insert(
            T::NAME.as_str(),
            NvimFunction::from_fn_mut(function.into_callback()),
        );
        self
    }

    /// Creates a new [`ModuleApi`].
    pub fn new(neovim_ctx: NeovimCtx<'static>) -> Self {
        Self {
            dictionary: NvimDictionary::default(),
            commands: ModuleCommands::new::<M>(),
            neovim_ctx,
            ty: PhantomData,
        }
    }
}
