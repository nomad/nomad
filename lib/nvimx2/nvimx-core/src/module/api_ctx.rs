use core::marker::PhantomData;

use crate::NeovimCtx;
use crate::action::ActionCtx;
use crate::backend::{
    Api,
    ApiValue,
    Backend,
    BackendExt,
    BackendHandle,
    BackendMut,
    Key,
    MapAccess,
    Value,
};
use crate::command::{Command, CommandBuilder, CommandCompletionsBuilder};
use crate::module::{Constant, Function, Module};
use crate::notify::{self, Error, MaybeResult, ModulePath, Name};
use crate::plugin::{self, Plugin};
use crate::util::OrderedMap;

/// TODO: docs.
pub(crate) fn build_api<P, B>(plugin: P, mut backend: BackendMut<B>) -> B::Api
where
    P: Plugin<B>,
    B: Backend,
{
    let plugin = Box::leak(Box::new(plugin));
    let mut command_builder = CommandBuilder::new::<P>();
    let mut command_completions_builder = CommandCompletionsBuilder::default();
    let mut config_builder = ConfigBuilder::new(plugin);
    let mut module_path = ModulePath::new(P::NAME);
    let mut api_ctx = ApiCtx {
        module_api: backend.api::<P>(),
        backend: backend.as_mut(),
        command_builder: &mut command_builder,
        completions_builder: &mut command_completions_builder,
        config_builder: &mut config_builder,
        module_path: &mut module_path,
        module: PhantomData,
    };
    Module::api(plugin, &mut api_ctx);
    let mut plugin_api = api_ctx.module_api;
    plugin_api.add_function(
        P::CONFIG_FN_NAME,
        config_builder.build::<P>(backend.handle()),
    );
    if P::COMMAND_NAME != plugin::NO_COMMAND_NAME
        && !command_builder.is_empty()
    {
        plugin_api.add_command::<P, _, _, _>(
            command_builder.build(backend.handle()),
            command_completions_builder.build(),
        );
    }
    plugin_api
}

/// TODO: docs.
pub struct ApiCtx<'a, M, B>
where
    M: Module<B>,
    B: Backend,
{
    backend: BackendMut<'a, B>,
    command_builder: &'a mut CommandBuilder<B>,
    completions_builder: &'a mut CommandCompletionsBuilder,
    config_builder: &'a mut ConfigBuilder<B>,
    module_api: B::Api,
    module_path: &'a mut ModulePath,
    module: PhantomData<M>,
}

struct ConfigBuilder<B: Backend> {
    handler: Box<dyn FnMut(ApiValue<B>, &mut NeovimCtx<B>)>,
    module_name: Name,
    submodules: OrderedMap<Name, Self>,
}

impl<M, B> ApiCtx<'_, M, B>
where
    M: Module<B>,
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn with_command<Cmd>(&mut self, command: Cmd) -> &mut Self
    where
        Cmd: Command<B>,
    {
        self.completions_builder.add_command(&command);
        self.command_builder.add_command(command);
        self
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_constant<Const>(&mut self, value: Const) -> &mut Self
    where
        Const: Constant,
    {
        let value = match self.backend.serialize(&value).into_result() {
            Ok(value) => value,
            Err(err) => {
                let source = notify::Source {
                    module_path: self.module_path,
                    action_name: Some(Const::NAME),
                };
                let msg = err.to_message(source).map(|(_, msg)| msg);
                panic!(
                    "couldn't serialize {:?}{colon}{reason:?}",
                    Const::NAME,
                    colon = if msg.is_some() { ": " } else { "" },
                    reason =
                        msg.as_ref().map(|msg| msg.as_str()).unwrap_or(""),
                );
            },
        };
        self.module_api.add_constant(Const::NAME, value);
        self
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_function<Fun>(&mut self, mut function: Fun) -> &mut Self
    where
        Fun: Function<B>,
    {
        let backend = self.backend.handle();
        let module_path = self.module_path.clone();
        let fun = move |value| {
            let fun = &mut function;
            let module_path = &module_path;
            backend.with_mut(move |mut backend| {
                let source = notify::Source {
                    module_path,
                    action_name: Some(Fun::NAME),
                };
                let args = match backend
                    .deserialize::<Fun::Args<'_>>(value)
                    .into_result()
                {
                    Ok(args) => args,
                    Err(err) => {
                        backend.emit_err(source, err);
                        return None;
                    },
                };
                let mut action_ctx = ActionCtx::new(
                    NeovimCtx::new(backend.as_mut(), module_path),
                    Fun::NAME,
                );
                let ret = match fun.call(args, &mut action_ctx).into_result() {
                    Ok(ret) => ret,
                    Err(err) => {
                        backend.emit_err(source, err);
                        return None;
                    },
                };
                match backend.serialize(&ret).into_result() {
                    Ok(ret) => Some(ret),
                    Err(err) => {
                        backend.emit_err(source, err);
                        None
                    },
                }
            })
        };
        self.module_api.add_function(Fun::NAME, fun);
        self
    }

    /// TODO: docs.
    #[inline]
    pub fn with_module<Mod>(&mut self, module: Mod) -> &mut Self
    where
        Mod: Module<B>,
    {
        self.module_path.push(Mod::NAME);
        let submodule_api = self.add_submodule::<Mod>(module);
        self.module_api.add_submodule::<Mod>(submodule_api);
        self.module_path.pop();
        self
    }

    #[inline]
    fn add_submodule<S: Module<B>>(&mut self, sub: S) -> B::Api {
        let sub = Box::leak(Box::new(sub));
        let mut ctx = ApiCtx {
            module_api: self.backend.api::<S>(),
            backend: self.backend.as_mut(),
            command_builder: self.command_builder.add_module::<S>(),
            completions_builder: self.completions_builder.add_module::<S, _>(),
            config_builder: self.config_builder.add_module(sub),
            module_path: self.module_path,
            module: PhantomData,
        };
        sub.api(&mut ctx);
        ctx.module_api
    }
}

impl<B: Backend> ConfigBuilder<B> {
    #[inline]
    fn add_module<M: Module<B>>(&mut self, module: &'static M) -> &mut Self {
        self.submodules.insert(M::NAME, ConfigBuilder::new(module))
    }

    #[inline]
    fn build<P: Plugin<B>>(
        mut self,
        backend: BackendHandle<B>,
    ) -> impl FnMut(ApiValue<B>) -> Option<ApiValue<B>> {
        move |config| {
            backend.with_mut(|backend| {
                let mut config_path = ModulePath::new(self.module_name);
                self.handle::<P>(config, &mut config_path, backend);
            });
            None
        }
    }

    #[inline]
    fn handle<P: Plugin<B>>(
        &mut self,
        mut config: ApiValue<B>,
        config_path: &mut ModulePath,
        mut backend: BackendMut<B>,
    ) {
        let mut map_access = match config.map_access() {
            Ok(map_access) => map_access,
            Err(err) => {
                backend.emit_map_access_error_in_config::<P>(config_path, err);
                return;
            },
        };
        loop {
            let Some(key) = map_access.next_key() else { break };
            let key_str = match key.as_str() {
                Ok(key) => key,
                Err(err) => {
                    backend.emit_key_as_str_error_in_config::<P>(
                        config_path,
                        err,
                    );
                    return;
                },
            };
            let Some(submodule) = self.submodules.get_mut(key_str) else {
                continue;
            };
            drop(key);
            let config = map_access.take_next_value();
            config_path.push(submodule.module_name);
            submodule.handle::<P>(config, config_path, backend.as_mut());
            config_path.pop();
        }
        drop(map_access);
        (self.handler)(config, &mut NeovimCtx::new(backend, config_path));
    }

    #[inline]
    fn new<M: Module<B>>(module: &'static M) -> Self {
        Self {
            handler: Box::new(|config, ctx| {
                match ctx
                    .backend_mut()
                    .deserialize::<M::Config>(config)
                    .into_result()
                {
                    Ok(config) => {
                        module.on_new_config(config, ctx);
                    },
                    Err(err) => todo!(),
                }
            }),
            module_name: M::NAME,
            submodules: Default::default(),
        }
    }
}
