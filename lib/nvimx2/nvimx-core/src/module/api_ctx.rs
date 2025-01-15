use core::marker::PhantomData;

use crate::NeovimCtx;
use crate::action::ActionCtx;
use crate::backend::{Api, ApiValue, Backend, Key, MapAccess, Value};
use crate::command::{Command, CommandBuilder, CommandCompletionsBuilder};
use crate::module::{Constant, Function, Module};
use crate::notify::{self, Error, MaybeResult, ModulePath, Name};
use crate::plugin::{self, Plugin};
use crate::state::{StateHandle, StateMut};
use crate::util::OrderedMap;

/// TODO: docs.
pub(crate) fn build_api<P, B>(plugin: P, mut state: StateMut<B>) -> B::Api
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
        module_api: state.api::<P>(),
        command_builder: &mut command_builder,
        completions_builder: &mut command_completions_builder,
        config_builder: &mut config_builder,
        module_path: &mut module_path,
        module: PhantomData,
        state: state.as_mut(),
    };
    Module::api(plugin, &mut api_ctx);
    let mut plugin_api = api_ctx.module_api;
    plugin_api.add_function(
        P::CONFIG_FN_NAME,
        config_builder.build::<P>(state.handle()),
    );
    if P::COMMAND_NAME != plugin::NO_COMMAND_NAME
        && !command_builder.is_empty()
    {
        plugin_api.add_command::<P, _, _, _>(
            command_builder.build(state.handle()),
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
    command_builder: &'a mut CommandBuilder<B>,
    completions_builder: &'a mut CommandCompletionsBuilder,
    config_builder: &'a mut ConfigBuilder<B>,
    module_api: B::Api,
    module_path: &'a mut ModulePath,
    module: PhantomData<M>,
    state: StateMut<'a, B>,
}

type ConfigHandler<B> = Box<
    dyn FnMut(
        ApiValue<B>,
        &mut NeovimCtx<B>,
    ) -> Result<(), <B as Backend>::DeserializeError>,
>;

struct ConfigBuilder<B: Backend> {
    handler: ConfigHandler<B>,
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
        let value = match self.state.serialize(&value).into_result() {
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
        let state = self.state.handle();
        let module_path = self.module_path.clone();
        let fun = move |value| {
            let fun = &mut function;
            let module_path = &module_path;
            state.with_mut(move |mut state| {
                let source = notify::Source {
                    module_path,
                    action_name: Some(Fun::NAME),
                };
                let args = match state
                    .deserialize::<Fun::Args<'_>>(value)
                    .into_result()
                {
                    Ok(args) => args,
                    Err(err) => {
                        state.emit_err(source, err);
                        return None;
                    },
                };
                let mut action_ctx = ActionCtx::new(
                    NeovimCtx::new(module_path, state.as_mut()),
                    Fun::NAME,
                );
                let ret = match fun.call(args, &mut action_ctx).into_result() {
                    Ok(ret) => ret,
                    Err(err) => {
                        state.emit_err(source, err);
                        return None;
                    },
                };
                match state.serialize(&ret).into_result() {
                    Ok(ret) => Some(ret),
                    Err(err) => {
                        state.emit_err(source, err);
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
            module_api: self.state.api::<S>(),
            command_builder: self.command_builder.add_module::<S>(),
            completions_builder: self.completions_builder.add_module::<S, _>(),
            config_builder: self.config_builder.add_module(sub),
            module_path: self.module_path,
            module: PhantomData,
            state: self.state.as_mut(),
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
        state: StateHandle<B>,
    ) -> impl FnMut(ApiValue<B>) -> Option<ApiValue<B>> {
        move |config| {
            state.with_mut(|state| {
                let mut config_path = ModulePath::new(self.module_name);
                let module_path = notify::ModulePath::new(P::NAME);
                let source = notify::Source {
                    module_path: &module_path,
                    action_name: Some(P::CONFIG_FN_NAME),
                };
                self.handle::<P>(config, source, &mut config_path, state);
            });
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn handle<P: Plugin<B>>(
        &mut self,
        mut config: ApiValue<B>,
        source: notify::Source,
        config_path: &mut ModulePath,
        mut state: StateMut<B>,
    ) {
        let mut map_access = match config.map_access() {
            Ok(map_access) => map_access,
            Err(err) => {
                state.emit_map_access_error_in_config::<P>(
                    config_path,
                    source,
                    err,
                );
                return;
            },
        };
        loop {
            let Some(key) = map_access.next_key() else { break };
            let key_str = match key.as_str() {
                Ok(key) => key,
                Err(err) => {
                    state.emit_key_as_str_error_in_config::<P>(
                        config_path,
                        source,
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
            submodule.handle::<P>(config, source, config_path, state.as_mut());
            config_path.pop();
        }
        drop(map_access);
        let mut ctx = NeovimCtx::new(source.module_path, state.as_mut());
        match (self.handler)(config, &mut ctx) {
            Ok(()) => {},
            Err(err) => {
                state.emit_deserialize_error_in_config::<P>(
                    config_path,
                    source,
                    err,
                );
            },
        }
    }

    #[inline]
    fn new<M: Module<B>>(module: &'static M) -> Self {
        Self {
            handler: Box::new(|config, ctx| {
                ctx.backend_mut()
                    .deserialize::<M::Config>(config)
                    .into_result()
                    .map(|config| {
                        module.on_new_config(config, ctx);
                    })
            }),
            module_name: M::NAME,
            submodules: Default::default(),
        }
    }
}
