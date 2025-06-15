use crate::command::{Command, CommandBuilder, CommandCompletionsBuilder};
use crate::module::{Constant, Function, Module};
use crate::notify::{self, Error, MaybeResult, Name, Namespace};
use crate::plugin::{self, Plugin, PluginId};
use crate::state::{StateHandle, StateMut};
use crate::util::OrderedMap;
use crate::{Api, ApiValue, Borrowed, Context, Editor, Key, MapAccess, Value};

/// TODO: docs.
pub(crate) fn build_api<P, Ed>(plugin: P, mut state: StateMut<Ed>) -> Ed::Api
where
    P: Plugin<Ed>,
    Ed: Editor,
{
    let plugin = state.add_plugin(plugin);
    let mut command_builder = CommandBuilder::new::<P>();
    let mut command_completions_builder = CommandCompletionsBuilder::default();
    let mut config_builder = ConfigBuilder::new(plugin);
    let mut namespace = Namespace::new(P::NAME);
    let mut api_ctx = ApiCtx {
        plugin_id: <P as Plugin<_>>::id(),
        module_api: Ed::Api::new(P::NAME),
        command_builder: &mut command_builder,
        completions_builder: &mut command_completions_builder,
        config_builder: &mut config_builder,
        namespace: &mut namespace,
        state: state.as_mut(),
    };
    Module::api(plugin, &mut api_ctx);
    api_ctx.state.with_ctx(api_ctx.namespace, api_ctx.plugin_id, |ctx| {
        plugin.on_init(ctx);
    });
    let mut plugin_api = api_ctx.module_api;
    plugin_api.add_function(
        P::CONFIG_FN_NAME,
        config_builder.build::<P>(state.handle()),
    );
    if P::COMMAND_NAME != plugin::NO_COMMAND_NAME
        && !command_builder.is_empty()
    {
        plugin_api.add_command::<_, _, _>(
            P::COMMAND_NAME,
            command_builder.build(state.handle()),
            command_completions_builder.build(),
        );
    }
    plugin_api
}

/// TODO: docs.
pub struct ApiCtx<'a, Ed: Editor> {
    plugin_id: PluginId,
    command_builder: &'a mut CommandBuilder<Ed>,
    completions_builder: &'a mut CommandCompletionsBuilder,
    config_builder: &'a mut ConfigBuilder<Ed>,
    module_api: Ed::Api,
    namespace: &'a mut Namespace,
    state: StateMut<'a, Ed>,
}

type ConfigHandler<B> = Box<
    dyn FnMut(
        ApiValue<B>,
        &mut Context<B, Borrowed<'_>>,
    ) -> Result<(), <B as Editor>::DeserializeError>,
>;

struct ConfigBuilder<Ed: Editor> {
    handler: ConfigHandler<Ed>,
    /// The module's name.
    module_name: Name,
    /// Whether the module's `Config` type is `()`.
    is_config_unit: bool,
    submodules: OrderedMap<Name, Self>,
}

impl<Ed: Editor> ApiCtx<'_, Ed> {
    /// Returns an exclusive reference to the editor.
    #[inline]
    pub fn editor_mut(&mut self) -> &mut Ed {
        &mut self.state
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_command<Cmd>(&mut self, command: Cmd) -> &mut Self
    where
        Cmd: Command<Ed>,
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
                let (_, msg) = err.to_message();
                panic!(
                    "couldn't serialize {:?}: {:?}",
                    Const::NAME,
                    msg.as_str(),
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
        Fun: Function<Ed>,
    {
        let state = self.state.handle();
        let mut namespace = self.namespace.clone();
        namespace.push(Fun::NAME);
        let plugin_id = self.plugin_id;
        let fun = move |value| {
            let fun = &mut function;
            let namespace = &mut namespace;
            state.with_mut(move |mut state| {
                let args = match state
                    .deserialize::<Fun::Args<'_>>(value)
                    .into_result()
                {
                    Ok(args) => args,
                    Err(err) => {
                        state.emit_err(namespace, err);
                        return None;
                    },
                };
                let res = state.with_ctx(namespace, plugin_id, |ctx| {
                    fun.call(args, ctx).into_result()
                });
                let ret = match res? {
                    Ok(ret) => ret,
                    Err(err) => {
                        state.emit_err(namespace, err);
                        return None;
                    },
                };
                match state.serialize(&ret).into_result() {
                    Ok(ret) => Some(ret),
                    Err(err) => {
                        state.emit_err(namespace, err);
                        None
                    },
                }
            })
        };
        self.module_api.add_function(Fun::NAME, fun);
        self
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_module<Mod>(&mut self, module: Mod) -> &mut Self
    where
        Mod: Module<Ed>,
    {
        self.namespace.push(Mod::NAME);
        let submodule_api = self.add_submodule::<Mod>(module);
        self.module_api.add_submodule(Mod::NAME, submodule_api);
        self.namespace.pop();
        self
    }

    #[track_caller]
    #[inline]
    fn add_submodule<S: Module<Ed>>(&mut self, sub: S) -> Ed::Api {
        let sub = self.state.add_module(sub);
        let mut ctx = ApiCtx {
            module_api: Ed::Api::new(S::NAME),
            command_builder: self.command_builder.add_module::<S>(),
            completions_builder: self.completions_builder.add_module::<S, _>(),
            config_builder: self.config_builder.add_module(sub),
            namespace: self.namespace,
            plugin_id: self.plugin_id,
            state: self.state.as_mut(),
        };
        sub.api(&mut ctx);
        ctx.state.with_ctx(ctx.namespace, ctx.plugin_id, |ctx| {
            sub.on_init(ctx);
        });
        ctx.module_api
    }
}

impl<Ed: Editor> ConfigBuilder<Ed> {
    #[inline]
    fn add_module<M: Module<Ed>>(&mut self, module: &'static M) -> &mut Self {
        self.submodules.insert(M::NAME, ConfigBuilder::new(module))
    }

    #[inline]
    fn build<P: Plugin<Ed>>(
        mut self,
        state: StateHandle<Ed>,
    ) -> impl FnMut(ApiValue<Ed>) -> Option<ApiValue<Ed>> {
        self.remove_empty_modules();
        let mut namespace = notify::Namespace::new(P::NAME);
        namespace.push(P::CONFIG_FN_NAME);
        move |config| {
            state.with_mut(|state| {
                let mut config_path = Namespace::new(self.module_name);
                self.handle::<P>(config, &namespace, &mut config_path, state);
            });
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn handle<P: Plugin<Ed>>(
        &mut self,
        mut config: ApiValue<Ed>,
        namespace: &Namespace,
        config_path: &mut Namespace,
        mut state: StateMut<Ed>,
    ) {
        let mut map_access = match config.map_access() {
            Ok(map_access) => map_access,
            Err(err) => {
                state.emit_map_access_error_in_config::<P>(
                    config_path,
                    namespace,
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
                        namespace,
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
            submodule.handle::<P>(
                config,
                namespace,
                config_path,
                state.as_mut(),
            );
            config_path.pop();
        }
        drop(map_access);
        if let Some(Err(err)) = state.with_ctx(
            config_path,
            <P as Plugin<_>>::id(),
            |ctx: &mut Context<Ed, Borrowed<'_>>| (self.handler)(config, ctx),
        ) {
            state.emit_deserialize_error_in_config::<P>(
                config_path,
                namespace,
                err,
            );
        }
    }

    #[inline]
    fn new<M: Module<Ed>>(module: &'static M) -> Self {
        Self {
            handler: Box::new(|config, ctx| {
                ctx.deserialize::<M::Config>(config).into_result().map(
                    |config| {
                        module.on_new_config(config, ctx);
                    },
                )
            }),
            module_name: M::NAME,
            is_config_unit: M::Config::is_unit(),
            submodules: Default::default(),
        }
    }

    /// Recursively removes the modules that shouldn't appear in the config.
    #[inline]
    fn remove_empty_modules(&mut self) {
        let mut idx = 0;
        loop {
            let Some((_, builder)) = self.submodules.get_index_mut(idx) else {
                break;
            };
            builder.remove_empty_modules();
            if builder.is_config_unit && builder.submodules.is_empty() {
                self.submodules.remove_index(idx);
            } else {
                idx += 1;
            }
        }
    }
}

trait IsUnit {
    fn is_unit() -> bool;
}

impl<T> IsUnit for T {
    #[inline]
    default fn is_unit() -> bool {
        false
    }
}

impl IsUnit for () {
    #[inline]
    fn is_unit() -> bool {
        true
    }
}
