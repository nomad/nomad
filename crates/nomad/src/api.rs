//! TODO: docs

use core::convert::Infallible;

use nvim::{self, Object};

use crate::command::{CommandArgs, ModuleCommands};
use crate::prelude::*;
use crate::serde::{deserialize, serialize};

/// TODO: docs
pub struct Api<M: Module> {
    pub(crate) commands: ModuleCommands,
    pub(crate) functions: Functions,
    pub(crate) module: M,
}

impl<M: Module> Api<M> {
    /// TODO: docs
    #[inline]
    pub fn new(module: M) -> Self {
        Self {
            commands: ModuleCommands::new(M::NAME),
            functions: Functions::default(),
            module,
        }
    }

    /// TODO: docs
    #[inline]
    pub fn with_command<A>(mut self, action: A) -> Self
    where
        A: Action<M, Return = ()>,
        A::Args: TryFrom<CommandArgs>,
        <A::Args as TryFrom<CommandArgs>>::Error: Into<WarningMsg>,
    {
        self.commands.add(action);
        self
    }

    /// TODO: docs
    #[inline]
    pub fn with_function<A>(mut self, action: A) -> Self
    where
        A: Action<M>,
    {
        self.functions.add(action);
        self
    }
}

/// TODO: docs
#[derive(Default)]
pub(crate) struct Functions {
    functions: nvim::Dictionary,
}

impl Functions {
    /// TODO: docs
    #[inline]
    pub(crate) fn into_dict(self) -> nvim::Dictionary {
        self.functions
    }

    /// TODO: docs
    #[inline]
    fn add<M: Module, A: Action<M>>(&mut self, action: A) {
        #[inline(always)]
        fn inner<M: Module, A: Action<M>>(
            a: &A,
            obj: Object,
        ) -> Result<Object, WarningMsg> {
            let arg = deserialize::<A::Args>(obj)?;
            let ret = a.execute(arg).into_result().map_err(Into::into)?;
            serialize(&ret).map_err(Into::into)
        }

        let function =
            nvim::Function::from_fn::<_, Infallible>(move |args: Object| {
                match inner(&action, args) {
                    Ok(obj) => Ok(obj),

                    Err(err) => {
                        Warning::new()
                            .module(M::NAME)
                            .action(A::NAME)
                            .msg(err)
                            .print();

                        Ok(Object::nil())
                    },
                }
            });

        self.functions.insert(A::NAME.as_str(), function);
    }
}
