use core::convert::Infallible;

use crate::action_ctx::ModulePath;
use crate::api::{Api, ModuleApi};
use crate::command::{CommandBuilder, CommandCompletionFns, CommandHandlers};
use crate::module::{ApiCtx, ConfigFnBuilder, Module};
use crate::{Backend, BackendHandle, Name};

/// TODO: docs.
pub trait Plugin<B: Backend>: Module<Self, B> {
    /// TODO: docs.
    const COMMAND_NAME: Name = panic!();

    /// TODO: docs.
    const CONFIG_FN_NAME: Name = "setup";

    /// TODO: docs.
    fn panic_handler(&self) -> Option<Box<dyn FnMut() + 'static>> {
        todo!()
    }

    /// TODO: docs.
    fn tracing_subscriber(&self) -> Option<Box<dyn FnMut() + 'static>> {
        todo!()
    }

    #[doc(hidden)]
    #[track_caller]
    fn api(self, mut backend: B) -> B::Api<Self> {
        let mut api = B::api::<Self>(&mut backend);
        let backend = BackendHandle::new(backend);
        let mut module_api = api.as_module();
        let mut command_has_been_added = false;
        let mut command_handlers = CommandHandlers::new::<Self, Self>();
        let mut command_completions = CommandCompletionFns::default();
        let command_builder = CommandBuilder::new(
            &mut command_has_been_added,
            &mut command_handlers,
            &mut command_completions,
        );
        let mut config_builder = ConfigFnBuilder::new::<Self, Self>();
        let mut module_path = ModulePath::new(Self::NAME);
        let mut api_ctx = ApiCtx::<Self, Self, _>::new(
            &mut module_api,
            command_builder,
            &mut config_builder,
            &mut module_path,
            &backend,
        );
        Module::api(&self, &mut api_ctx);

        config_builder.finish(self);
        let mut config_fn = config_builder.build(backend.clone());
        module_api.add_function(Self::CONFIG_FN_NAME, move |value| {
            config_fn(value);
            Ok::<_, Infallible>(B::ApiValue::default())
        });

        module_api.finish();

        if command_has_been_added {
            let command = command_handlers.build(backend);
            let completion_fn = command_completions.build();
            api.add_command(command, completion_fn);
        }

        api
    }
}
