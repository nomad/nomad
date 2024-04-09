//! TODO: docs

use core::cell::RefCell;
use core::convert::Infallible;
use std::rc::Rc;

use futures::FutureExt;
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
        let action = Rc::new(RefCell::new(action));

        let action = move |args| exec_action(Rc::clone(&action), args);

        self.functions
            .insert(A::NAME.as_str(), nvim::Function::from_fn(action));
    }
}

/// TODO: docs
#[inline]
fn exec_action<M, A>(
    action: Rc<RefCell<A>>,
    args: Object,
) -> Result<Object, Infallible>
where
    M: Module,
    A: Action<M>,
    A::Args: DeserializeOwned,
    A::Return: Serialize,
{
    let mut task = spawn(async_exec_action(action, args));

    let Some(syncness) = (&mut task).now_or_never() else {
        // The action is async and it's not done yet.
        task.detach();
        return Ok(Object::nil());
    };

    let res = match syncness {
        ActionSyncness::Sync(future_res) => match future_res {
            Ok(action_ret) => {
                serialize(&action_ret, "result").map_err(Into::into)
            },

            Err(ExecuteActionError::Borrow) => {
                // Should we maybe return an error to notify the user that the
                // action couldn't be executed?
                Ok(Object::nil())
            },

            Err(ExecuteActionError::Deserialize(de_err)) => {
                Err(WarningMsg::from(de_err))
            },

            Err(ExecuteActionError::ReadyFailed(msg)) => Err(msg),
        },

        // The action was async but it resolved on the first poll.
        ActionSyncness::Async => Ok(Object::nil()),
    };

    match res {
        Ok(obj) => Ok(obj),

        Err(warning_msg) => {
            Warning::new()
                .module(M::NAME)
                .action(A::NAME)
                .msg(warning_msg)
                .print();

            Ok(Object::nil())
        },
    }
}

/// TODO: docs
#[allow(clippy::await_holding_refcell_ref)]
async fn async_exec_action<M, A>(
    action: Rc<RefCell<A>>,
    args: Object,
) -> ActionSyncness<Result<A::Return, ExecuteActionError>>
where
    M: Module,
    A: Action<M>,
    A::Args: DeserializeOwned,
    A::Return: Serialize,
{
    let args = match deserialize::<A::Args>(args, "args") {
        Ok(args) => args,
        Err(de_err) => {
            return ActionSyncness::Sync(Err(ExecuteActionError::Deserialize(
                de_err,
            )))
        },
    };

    let Ok(action) = action.try_borrow_mut() else {
        return ActionSyncness::Sync(Err(ExecuteActionError::Borrow));
    };

    let future = match action.execute(args).into_enum() {
        MaybeFutureEnum::Ready(res) => {
            let res = res
                .into_result()
                .map_err(Into::into)
                .map_err(ExecuteActionError::ReadyFailed);

            return ActionSyncness::Sync(res);
        },

        MaybeFutureEnum::Future(future) => future,
    };

    if let Err(err) = future.await.into_result() {
        Warning::new().module(M::NAME).action(A::NAME).msg(err.into()).print();
    }

    ActionSyncness::Async
}

enum ActionSyncness<T> {
    /// The action is synchronous, so the future is guaranteed to resolve
    /// immediately.
    Sync(T),

    /// The action is asynchronous, so the future may not resolve
    /// immediately.
    Async,
}

enum ExecuteActionError {
    /// It wasn't possible to obtain a mutable reference to the action because
    /// it's still being used by a previous execution that hasn't yet finished.
    Borrow,

    /// The arguments didn't deserialize correctly.
    Deserialize(crate::serde::DeserializeError),

    /// The action was sync and it returned immediately, but it returned an
    /// error.
    ReadyFailed(WarningMsg),
}
