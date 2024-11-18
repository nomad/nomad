use fxhash::FxHashMap;
use nvimx_common::MaybeResult;
use nvimx_ctx::NeovimCtx;
use nvimx_diagnostics::{DiagnosticSource, Level};

use crate::action_name::ActionNameStr;
use crate::module::Module;
use crate::module_name::ModuleName;
use crate::subcommand::{CompletionFunc, SubCommand};
use crate::subcommand_args::{SubCommandArgs, SubCommandCursor};

pub(super) struct ModuleSubCommands {
    /// The name of the module these commands belong to.
    pub(super) module_name: ModuleName,

    /// The command to run when no command is specified.
    pub(super) default_subcommand: Option<SubCommandHandle>,

    /// Map from command name to the corresponding [`Command`].
    pub(super) subcommands: FxHashMap<ActionNameStr, SubCommandHandle>,

    pub(super) neovim_ctx: NeovimCtx<'static>,
}

pub(crate) struct SubCommandHandle {
    callback: Box<dyn FnMut(SubCommandArgs)>,
    completion_func:
        Box<dyn FnMut(SubCommandArgs, SubCommandCursor) -> Vec<String>>,
}

impl ModuleSubCommands {
    #[track_caller]
    pub(crate) fn add_default_subcommand<T>(&mut self, subcommand: T)
    where
        T: SubCommand,
    {
        if self.module_name != T::Module::NAME {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                T::Module::NAME,
                self.module_name
            );
        }
        if self.default_subcommand.is_some() {
            panic!(
                "a default command has already been set for module '{}'",
                self.module_name
            );
        }
        self.default_subcommand =
            Some(SubCommandHandle::new(subcommand, self.neovim_ctx.clone()));
    }

    #[track_caller]
    pub(crate) fn add_subcommand<T>(&mut self, subcommand: T)
    where
        T: SubCommand,
    {
        if self.module_name != T::Module::NAME {
            panic!(
                "trying to register a command for module '{}' in the API for \
                 module '{}'",
                T::Module::NAME,
                self.module_name
            );
        }
        if self.subcommands.contains_key(&T::NAME.as_str()) {
            panic!(
                "a command with the name '{}' already exists in the API for \
                 module '{}'",
                T::NAME,
                self.module_name
            );
        }
        self.subcommands.insert(
            T::NAME.as_str(),
            SubCommandHandle::new(subcommand, self.neovim_ctx.clone()),
        );
    }

    pub(crate) fn default_subcommand(
        &mut self,
    ) -> Option<&mut SubCommandHandle> {
        self.default_subcommand.as_mut()
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = ActionNameStr> + '_ {
        self.subcommands.keys().copied()
    }

    pub(crate) fn new<M: Module>(neovim_ctx: NeovimCtx<'static>) -> Self {
        Self {
            module_name: M::NAME,
            default_subcommand: None,
            subcommands: FxHashMap::default(),
            neovim_ctx,
        }
    }

    pub(crate) fn subcommand<'a>(
        &'a mut self,
        subcommand_name: &'a str,
    ) -> Option<&'a mut SubCommandHandle> {
        self.subcommands.get_mut(subcommand_name)
    }
}

impl SubCommandHandle {
    pub(crate) fn call(&mut self, args: SubCommandArgs) {
        (self.callback)(args);
    }

    pub(crate) fn complete(
        &mut self,
        args: SubCommandArgs,
        cursor: SubCommandCursor,
    ) -> Vec<String> {
        (self.completion_func)(args, cursor)
    }

    fn new<T: SubCommand>(subcommand: T, ctx: NeovimCtx<'static>) -> Self {
        let completion_func = Box::new({
            let mut func = subcommand.completion_func();
            move |args: SubCommandArgs, cursor: SubCommandCursor| {
                func.call(args, cursor)
            }
        });
        let mut callback = callback_of_subcommand(subcommand);
        let ctx = ctx.clone();
        Self {
            callback: Box::new(move |args| callback(args, ctx.reborrow())),
            completion_func,
        }
    }
}

fn callback_of_subcommand<T: SubCommand>(
    mut subcommand: T,
) -> impl for<'a> FnMut(SubCommandArgs<'a>, NeovimCtx<'a>) {
    move |args, ctx: NeovimCtx<'_>| {
        let args = match T::Args::try_from(args) {
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
        if let Err(err) = subcommand.execute(args, ctx).into_result() {
            let mut source = DiagnosticSource::new();
            source
                .push_segment(T::Module::NAME.as_str())
                .push_segment(T::NAME.as_str());
            err.into().emit(Level::Error, source);
        }
    }
}
