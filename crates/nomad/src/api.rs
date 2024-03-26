//! TODO: docs

use core::convert::Infallible;

use nvim::Object;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

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
        A::Args: DeserializeOwned,
        A::Return: Serialize,
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
    fn add<M: Module, A: Action<M>>(&mut self, action: A)
    where
        A::Args: DeserializeOwned,
        A::Return: Serialize,
    {
        #[inline(always)]
        fn inner<M: Module, A: Action<M>>(
            _a: &A,
            obj: Object,
        ) -> Result<Object, WarningMsg>
        where
            A::Args: DeserializeOwned,
            A::Return: Serialize,
        {
            let _args = deserialize::<A::Args>(obj, "args")?;
            todo!();
            // let ret = a.execute(args).into_result().map_err(Into::into)?;
            // serialize(&ret, "result").map_err(Into::into)
        }

        let function = move |args: Object| match inner(&action, args) {
            Ok(obj) => Ok(obj),

            Err(err) => {
                Warning::new()
                    .module(M::NAME)
                    .action(A::NAME)
                    .msg(err)
                    .print();

                Ok(Object::nil())
            },
        };

        self.functions.insert(
            A::NAME.as_str(),
            nvim::Function::from_fn::<_, Infallible>(function),
        );
    }
}
