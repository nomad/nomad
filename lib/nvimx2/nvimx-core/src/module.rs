//! TODO: docs.

use serde::de::DeserializeOwned;

use crate::api::{Api, ModuleApi};
use crate::backend::{Key, MapAccess, Value};
use crate::command::{Command, CommandBuilder};
use crate::util::OrderedMap;
use crate::{
    Backend,
    BackendExt,
    BackendHandle,
    Function,
    MaybeResult,
    NeovimCtx,
    Plugin,
    notify,
};

/// TODO: docs.
pub trait Module<B: Backend>: 'static + Sized {
    /// TODO: docs.
    const NAME: &'static ModuleName;

    /// TODO: docs.
    type Config: DeserializeOwned;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn api<P: Plugin<B>>(&self, ctx: ApiCtx<'_, '_, Self, P, B>);

    /// TODO: docs.
    fn on_config_changed(
        &mut self,
        new_config: Self::Config,
        ctx: &mut NeovimCtx<'_, B>,
    );

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

/// TODO: docs.
pub struct ApiCtx<'a, 'b, M: Module<B>, P: Plugin<B>, B: Backend> {
    module_api: &'a mut <B::Api<P> as Api<P, B>>::ModuleApi<'b, M>,
    command_builder: CommandBuilder<'a, B>,
    config_builder: &'a mut ConfigFnBuilder<B>,
    namespace: &'a mut notify::Namespace,
    backend: &'b BackendHandle<B>,
}

/// TODO: docs.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ModuleName(str);

pub(crate) struct ConfigFnBuilder<B: Backend> {
    module_name: &'static ModuleName,
    config_handler: Box<
        dyn FnMut(B::ApiValue, &mut notify::Namespace, &mut NeovimCtx<B>)
            + 'static,
    >,
    submodules: OrderedMap<&'static str, Self>,
}

impl<'a, 'b, M, P, B> ApiCtx<'a, 'b, M, P, B>
where
    M: Module<B>,
    P: Plugin<B>,
    B: Backend,
{
    /// TODO: docs.
    #[inline]
    pub fn with_command<Cmd>(mut self, command: Cmd) -> Self
    where
        Cmd: Command<B>,
    {
        self.command_builder.add_command(command);
        self
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_function<Fun>(self, mut function: Fun) -> Self
    where
        Fun: Function<B>,
    {
        let backend = self.backend.clone();
        let mut namespace = self.namespace.clone();
        namespace.set_action(Fun::NAME);
        let fun = move |value| {
            let fun = &mut function;
            let namespace = &namespace;
            backend.with_mut(move |mut backend| {
                let args = backend.deserialize::<Fun::Args>(value).map_err(
                    |err| {
                        backend.emit_err(namespace, &err);
                        FunctionError::Deserialize(err)
                    },
                )?;

                let ret = fun
                    .call(args, NeovimCtx::new(backend.as_mut()))
                    .into_result()
                    .map_err(|err| {
                        // Even though the error is bound to 'static, Rust
                        // thinks that the error captures some lifetime due to
                        // `Function::call()` returning an `impl MaybeResult`.
                        //
                        // Should be the same problem as
                        // https://github.com/rust-lang/rust/issues/42940
                        //
                        // FIXME: Is there a better way around this than boxing
                        // the error?
                        Box::new(err) as Box<dyn notify::Error>
                    })
                    .map_err(|err| {
                        backend.emit_err(namespace, &err);
                        FunctionError::Call(err)
                    })?;

                backend.serialize(&ret).map_err(|err| {
                    backend.emit_err(namespace, &err);
                    FunctionError::Serialize(err)
                })
            })
        };
        self.module_api.add_function(Fun::NAME, fun);
        self
    }

    /// TODO: docs.
    #[inline]
    pub fn with_module<Mod>(mut self, module: Mod) -> Self
    where
        Mod: Module<B>,
    {
        let mut module_api = self.module_api.as_module::<Mod>();
        self.namespace.push_module(Mod::NAME);
        let api_ctx = ApiCtx::new(
            &mut module_api,
            self.command_builder.add_module::<Mod>(),
            self.config_builder.add_module::<Mod>(),
            self.namespace,
            self.backend,
        );
        Module::api(&module, api_ctx);
        module_api.finish();
        self.namespace.pop();
        self.config_builder.finish(module);
        self
    }

    #[inline]
    pub(crate) fn new(
        module_api: &'a mut <B::Api<P> as Api<P, B>>::ModuleApi<'b, M>,
        command_builder: CommandBuilder<'a, B>,
        config_builder: &'a mut ConfigFnBuilder<B>,
        namespace: &'a mut notify::Namespace,
        backend: &'b BackendHandle<B>,
    ) -> Self {
        Self {
            module_api,
            command_builder,
            config_builder,
            namespace,
            backend,
        }
    }
}

impl ModuleName {
    /// TODO: docs.
    #[inline]
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    /// TODO: docs.
    #[inline]
    pub const fn new(name: &str) -> &Self {
        assert!(!name.is_empty());
        assert!(name.len() <= 24);
        // SAFETY: `ModuleName` is a `repr(transparent)` newtype around `str`.
        unsafe { &*(name as *const str as *const Self) }
    }

    /// TODO: docs.
    #[inline]
    pub const fn uppercase_first(&self) -> &Self {
        todo!();
    }
}

impl<B: Backend> ConfigFnBuilder<B> {
    #[inline]
    pub(crate) fn build(
        mut self,
        backend: BackendHandle<B>,
    ) -> impl FnMut(B::ApiValue) + 'static {
        move |value| {
            backend.with_mut(|backend| {
                self.handle(
                    value,
                    &mut notify::Namespace::default(),
                    &mut NeovimCtx::new(backend),
                )
            });
        }
    }

    #[inline]
    pub(crate) fn finish<M: Module<B>>(&mut self, mut module: M) {
        self.config_handler = Box::new(move |value, namespace, ctx| {
            let backend = ctx.backend_mut();
            match backend.deserialize(value) {
                Ok(config) => module.on_config_changed(config, ctx),
                Err(err) => {
                    // backend.emit_deserialize_config_error(namespace, err)
                    backend.emit_err(namespace, err)
                },
            }
        });
    }

    #[inline]
    pub(crate) fn new<M: Module<B>>() -> Self {
        Self {
            module_name: M::NAME,
            config_handler: Box::new(|_, _, _| {}),
            submodules: Default::default(),
        }
    }

    #[inline]
    fn add_module<M: Module<B>>(&mut self) -> &mut Self {
        self.submodules.insert(M::NAME.as_str(), ConfigFnBuilder::new::<M>())
    }

    #[inline]
    fn handle(
        &mut self,
        mut value: B::ApiValue,
        namespace: &mut notify::Namespace,
        ctx: &mut NeovimCtx<B>,
    ) {
        let mut map_access = match value.map_access() {
            Ok(map_access) => map_access,
            Err(err) => {
                // TODO: the namespace should just be the plugin and the
                // config fn name.
                ctx.backend_mut().emit_err(namespace, &err);
                return;
            },
        };
        namespace.push_module(self.module_name);
        loop {
            let Some(key) = map_access.next_key() else { break };
            let key_str = match key.as_str() {
                Ok(key) => key,
                Err(err) => {
                    // TODO: same as above.
                    ctx.backend_mut().emit_err(namespace, &err);
                    namespace.push_module(self.module_name);
                    return;
                },
            };
            let Some(submodule) = self.submodules.get_mut(key_str) else {
                continue;
            };
            drop(key);
            let value = map_access.take_next_value();
            submodule.handle(value, namespace, ctx);
        }
        drop(map_access);
        (self.config_handler)(value, namespace, ctx);
        namespace.pop();
    }
}

enum FunctionError<D, C, S> {
    Deserialize(D),
    Call(C),
    Serialize(S),
}

impl<D, C, S> notify::Error for FunctionError<D, C, S>
where
    D: notify::Error,
    C: notify::Error,
    S: notify::Error,
{
    #[inline]
    fn to_level(&self) -> Option<notify::Level> {
        match self {
            Self::Deserialize(err) => err.to_level(),
            Self::Call(err) => err.to_level(),
            Self::Serialize(err) => err.to_level(),
        }
    }

    #[inline]
    fn to_message(&self) -> notify::Message {
        match self {
            Self::Deserialize(err) => err.to_message(),
            Self::Call(err) => err.to_message(),
            Self::Serialize(err) => err.to_message(),
        }
    }
}
