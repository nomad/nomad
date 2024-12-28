use core::mem::ManuallyDrop;

use serde::de::{Deserialize, DeserializeOwned};
use serde::ser::Serialize;

use crate::api::{Api, ModuleApi};
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
    type Plugin: Plugin<B>;

    /// TODO: docs.
    type Config: DeserializeOwned;

    /// TODO: docs.
    type Docs;

    /// TODO: docs.
    fn api(&self, ctx: ModuleApiCtx<'_, Self, B>);

    /// TODO: docs.
    fn on_config_changed(
        &mut self,
        new_config: Self::Config,
        ctx: NeovimCtx<'_, B>,
    );

    /// TODO: docs.
    fn docs() -> Self::Docs;
}

/// TODO: docs.
pub struct ModuleApiCtx<'a, M: Module<B>, B: Backend> {
    #[allow(clippy::type_complexity)]
    api: ManuallyDrop<
        <B::Api<M::Plugin> as Api<M::Plugin, B>>::ModuleApi<'a, M>,
    >,
    backend: &'a BackendHandle<B>,
}

/// TODO: docs.
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ModuleName(str);

impl<'a, M, B> ModuleApiCtx<'a, M, B>
where
    M: Module<B>,
    B: Backend,
{
    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_function<Fun>(mut self, mut fun: Fun) -> Self
    where
        Fun: Function<B, Module = M>,
    {
        let backend = self.backend.clone();

        let fun = move |value| {
            let fun = &mut fun;
            backend.with_mut(move |mut backend| {
                let args = Fun::Args::deserialize(backend.deserializer(value))
                    .map_err(|err| {
                        backend.emit_err(&err);
                        FunctionError::Deserialize(err)
                    })?;

                let ret = fun
                    .call(args, NeovimCtx::new(backend.reborrow()))
                    .into_result()
                    .map_err(|err| {
                        // Even though the error is bound to `notify::Error`
                        // (which itself is bound to `'static`), Rust thinks
                        // that the error captures some lifetime due to
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
                        backend.emit_err(&err);
                        FunctionError::Call(err)
                    })?;

                ret.serialize(backend.serializer()).map_err(|err| {
                    backend.emit_err(&err);
                    FunctionError::Serialize(err)
                })
            })
        };
        self.api.add_function(Fun::NAME, fun);
        self
    }

    #[inline]
    pub(crate) fn new(
        api: &'a mut B::Api<M::Plugin>,
        backend: &'a BackendHandle<B>,
    ) -> Self {
        Self { api: ManuallyDrop::new(api.with_module::<M>()), backend }
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
}

impl<M, B> Drop for ModuleApiCtx<'_, M, B>
where
    M: Module<B>,
    B: Backend,
{
    #[inline]
    fn drop(&mut self) {
        // SAFETY: We never use the `ManuallyDrop` again.
        let api = unsafe { ManuallyDrop::take(&mut self.api) };
        api.finish();
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
