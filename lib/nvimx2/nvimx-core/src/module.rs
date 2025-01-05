//! TODO: docs.

use serde::de::DeserializeOwned;

use crate::action_ctx::ModulePath;
use crate::api::{Api, ModuleApi};
use crate::backend::{Key, MapAccess, Value};
use crate::command::{Command, CommandBuilder};
use crate::util::OrderedMap;
use crate::{
    ActionCtx,
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
    fn api<P: Plugin<B>>(&self, ctx: &mut ApiCtx<Self, P, B>);

    /// TODO: docs.
    fn on_config_changed(
        &mut self,
        new_config: Self::Config,
        ctx: &mut NeovimCtx<B>,
    );

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

/// TODO: docs.
pub struct ApiCtx<'a, 'b, M: Module<B>, P: Plugin<B>, B: Backend> {
    module_api: &'a mut <B::Api<P> as Api<P, B>>::ModuleApi<'b, M>,
    command_builder: CommandBuilder<'a, B>,
    config_builder: &'a mut ConfigFnBuilder<B>,
    module_path: &'a mut ModulePath,
    backend: &'b BackendHandle<B>,
}

/// TODO: docs.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ModuleName(str);

pub(crate) struct ConfigFnBuilder<B: Backend> {
    module_name: &'static ModuleName,
    config_handler: Box<
        dyn FnMut(B::ApiValue, &mut ModulePath, &mut NeovimCtx<B>) + 'static,
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
    pub fn with_command<Cmd>(&mut self, command: Cmd) -> &mut Self
    where
        Cmd: Command<B>,
    {
        self.command_builder.add_command(command);
        self
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_function<Fun>(&mut self, mut function: Fun) -> &mut Self
    where
        Fun: Function<B>,
    {
        let backend = self.backend.clone();
        let module_path = self.module_path.clone();
        let fun = move |value| {
            let fun = &mut function;
            let module_path = &module_path;
            backend.with_mut(move |mut backend| {
                let args = backend.deserialize::<Fun::Args>(value).map_err(
                    |err| {
                        backend.emit_action_err(module_path, Fun::NAME, &err);
                        FunctionError::Deserialize(err)
                    },
                )?;

                let mut action_ctx = ActionCtx::new(
                    NeovimCtx::new(backend.as_mut()),
                    &module_path,
                );

                let ret = fun
                    .call(args, &mut action_ctx)
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
                        backend.emit_action_err(module_path, Fun::NAME, &err);
                        FunctionError::Call(err)
                    })?;

                backend.serialize(&ret).map_err(|err| {
                    backend.emit_action_err(module_path, Fun::NAME, &err);
                    FunctionError::Serialize(err)
                })
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
        let mut module_api = self.module_api.as_module::<Mod>();
        self.module_path.push(Mod::NAME);
        let mut api_ctx = ApiCtx::new(
            &mut module_api,
            self.command_builder.add_module::<Mod>(),
            self.config_builder.add_module::<Mod>(),
            self.module_path,
            self.backend,
        );
        Module::api(&module, &mut api_ctx);
        module_api.finish();
        self.module_path.pop();
        self.config_builder.finish(module);
        self
    }

    #[inline]
    pub(crate) fn new(
        module_api: &'a mut <B::Api<P> as Api<P, B>>::ModuleApi<'b, M>,
        command_builder: CommandBuilder<'a, B>,
        config_builder: &'a mut ConfigFnBuilder<B>,
        module_path: &'a mut ModulePath,
        backend: &'b BackendHandle<B>,
    ) -> Self {
        Self {
            module_api,
            command_builder,
            config_builder,
            module_path,
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
                let mut module_path = ModulePath::new(self.module_name);
                self.handle(
                    value,
                    &mut module_path,
                    &mut NeovimCtx::new(backend),
                )
            });
        }
    }

    #[inline]
    pub(crate) fn finish<M: Module<B>>(&mut self, mut module: M) {
        self.config_handler = Box::new(move |value, module_path, ctx| {
            let backend = ctx.backend_mut();
            match backend.deserialize(value) {
                Ok(config) => module.on_config_changed(config, ctx),
                Err(err) => {
                    // backend.emit_deserialize_config_error(namespace, err)
                    backend.emit_err(module_path, err)
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
        module_path: &mut ModulePath,
        ctx: &mut NeovimCtx<B>,
    ) {
        let mut map_access = match value.map_access() {
            Ok(map_access) => map_access,
            Err(err) => {
                // TODO: the namespace should just be the plugin and the
                // config fn name.
                ctx.backend_mut().emit_err(module_path, &err);
                return;
            },
        };
        module_path.push(self.module_name);
        loop {
            let Some(key) = map_access.next_key() else { break };
            let key_str = match key.as_str() {
                Ok(key) => key,
                Err(err) => {
                    // TODO: same as above.
                    ctx.backend_mut().emit_err(module_path, &err);
                    module_path.push(self.module_name);
                    return;
                },
            };
            let Some(submodule) = self.submodules.get_mut(key_str) else {
                continue;
            };
            drop(key);
            let value = map_access.take_next_value();
            submodule.handle(value, module_path, ctx);
        }
        drop(map_access);
        (self.config_handler)(value, module_path, ctx);
        module_path.pop();
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
