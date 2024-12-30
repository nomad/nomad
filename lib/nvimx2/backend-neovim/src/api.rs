//! TODO: docs.

use core::marker::PhantomData;

use nvimx_core::api::{Api, ModuleApi};
use nvimx_core::command::{CommandArgs, CommandCompletion};
use nvimx_core::module::Module;
use nvimx_core::{ActionName, ByteOffset, Plugin, notify};

use crate::Neovim;
use crate::oxi::{Dictionary, Function, Object, api};

/// TODO: docs.
pub struct NeovimApi<P> {
    dictionary: Dictionary,
    _plugin: PhantomData<P>,
}

/// TODO: docs.
pub struct NeovimModuleApi<'a, M> {
    dictionary: &'a mut Dictionary,
    _module: PhantomData<M>,
}

impl<'a, M: Module<Neovim>> NeovimModuleApi<'a, M> {
    #[track_caller]
    #[inline]
    fn insert(
        &mut self,
        field_name: &str,
        value: impl Into<Object>,
    ) -> &mut Object {
        if self.dictionary.get(field_name).is_some() {
            panic!(
                "a field with name '{}' has already been added to {}'s API",
                field_name,
                M::NAME.as_str(),
            );
        }
        self.dictionary.insert(field_name, value.into());
        self.dictionary.get_mut(field_name).expect("just inserted it")
    }

    #[inline]
    fn new(dictionary: &'a mut Dictionary) -> Self {
        Self { dictionary, _module: PhantomData }
    }
}

impl<P> Api<P, Neovim> for NeovimApi<P>
where
    P: Plugin<Neovim>,
{
    type ModuleApi<'a, M: Module<Neovim>> = NeovimModuleApi<'a, M>;

    #[inline]
    fn add_command<Cmd, CompFun, Comps>(
        &mut self,
        mut command: Cmd,
        mut completion_fun: CompFun,
    ) where
        Cmd: FnMut(CommandArgs) + 'static,
        CompFun: FnMut(CommandArgs, ByteOffset) -> Comps + 'static,
        Comps: IntoIterator<Item = CommandCompletion>,
    {
        let command_name = P::COMMAND_NAME.as_str();

        let command =
            Function::from_fn_mut(move |args: api::types::CommandArgs| {
                command(CommandArgs::new(
                    args.args.as_deref().unwrap_or_default(),
                ))
            });

        let completion_fun = Function::from_fn_mut(
            move |(_, command_str, mut cursor_offset): (
                String,
                String,
                usize,
            )| {
                // Trim any leading whitespace.
                let initial_len = command_str.len();
                let command_str = command_str.trim_start();
                cursor_offset -= initial_len - command_str.len();

                // The command line must start with "<Command> " for Neovim to
                // invoke us.
                let subcommand_starts_from = command_name.len() + 1;
                debug_assert!(command_str.starts_with(command_name));
                debug_assert!(cursor_offset >= subcommand_starts_from);

                let args = &command_str[subcommand_starts_from..];
                cursor_offset -= subcommand_starts_from;

                completion_fun(
                    CommandArgs::new(args),
                    ByteOffset::new(cursor_offset),
                )
                .into_iter()
                .map(|comp| comp.as_str().to_owned())
                .collect::<Vec<_>>()
            },
        );

        let opts = api::opts::CreateCommandOpts::builder()
            .complete(api::types::CommandComplete::CustomList(completion_fun))
            .force(true)
            .nargs(api::types::CommandNArgs::Any)
            .build();

        api::create_user_command(command_name, command, &opts)
            .expect("all arguments are valid");
    }

    #[track_caller]
    #[inline]
    fn as_module(&mut self) -> Self::ModuleApi<'_, P> {
        NeovimModuleApi::new(&mut self.dictionary)
    }
}

impl<P, M> ModuleApi<NeovimApi<P>, P, M, Neovim> for NeovimModuleApi<'_, M>
where
    P: Plugin<Neovim>,
    M: Module<Neovim>,
{
    #[track_caller]
    #[inline]
    fn add_function<Fun, Err>(&mut self, fun_name: &ActionName, mut fun: Fun)
    where
        Fun: FnMut(Object) -> Result<Object, Err> + 'static,
        Err: notify::Error,
    {
        self.insert(
            fun_name.as_str(),
            Function::from_fn_mut(move |args| fun(args).unwrap_or_default()),
        );
    }

    #[track_caller]
    #[inline]
    fn as_module<M2: Module<Neovim>>(&mut self) -> NeovimModuleApi<'_, M2> {
        let obj = self.insert(M2::NAME.as_str(), Dictionary::default());
        // SAFETY: We just inserted a dictionary.
        let dictionary = unsafe { obj.as_dictionary_unchecked_mut() };
        NeovimModuleApi::new(dictionary)
    }

    #[inline]
    fn finish(self) {}
}

impl<P> Default for NeovimApi<P>
where
    P: Plugin<Neovim>,
{
    #[inline]
    fn default() -> Self {
        Self { dictionary: Dictionary::default(), _plugin: PhantomData }
    }
}

impl<P> From<NeovimApi<P>> for Dictionary
where
    P: Plugin<Neovim>,
{
    #[inline]
    fn from(api: NeovimApi<P>) -> Self {
        api.dictionary
    }
}
