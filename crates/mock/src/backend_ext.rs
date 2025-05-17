use core::mem;

use ed::executor::Executor;
use ed::{Backend, Context};
use futures_lite::future;

/// A [`Backend`] extension trait used to run async closures with an
/// `Context<Self>`.
pub trait BackendExt:
    Backend<Executor: Executor<Runner: AsMut<crate::executor::Runner>>>
{
    /// Same as [`run`](BackendExt::run), but it blocks the current thread
    /// until the future returned by it completes.
    #[inline]
    fn block_on<T>(self, fun: impl AsyncFnOnce(&mut Context<Self>) -> T) -> T {
        self.block_on_inner(fun, false)
    }

    /// Same as [`run_all`](BackendExt::run_all), but it blocks the
    /// current thread until the future returned by it completes.
    #[inline]
    fn block_on_all<T>(
        self,
        fun: impl AsyncFnOnce(&mut Context<Self>) -> T,
    ) -> T {
        self.block_on_inner(fun, true)
    }

    /// Turns the given async closure into a future that resolves to the
    /// closure's output.
    ///
    /// Unlike [`run_all`](BackendExt::run_all), the returned future will
    /// complete at the same time as the future obtained by calling the
    /// closure, without waiting for any detached task spawned in the closure's
    /// body.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::{Duration, Instant};
    /// # use mock::{BackendExt, Mock};
    /// # use mock::fs::MockFs;
    /// #
    /// # futures_lite::future::block_on(async {
    /// let start = Instant::now();
    ///
    /// Mock::<MockFs>::default()
    ///     .run(async |ctx| {
    ///         ctx.spawn_local(async |_| {
    ///             async_io::Timer::after(Duration::from_secs(2)).await;
    ///         })
    ///         .detach();
    ///     })
    ///     .await;
    ///
    /// // The future returned by `run()` completes immediately, without
    /// // waiting for the timer to expire.
    /// assert!(start.elapsed() < Duration::from_secs(2));
    /// # });
    /// ```
    #[inline]
    fn run<T: 'static>(
        self,
        fun: impl AsyncFnOnce(&mut Context<Self>) -> T + 'static,
    ) -> impl Future<Output = T> {
        self.run_inner(fun, false)
    }

    /// Turns the given async closure into a future that resolves to the
    /// closure's output.
    ///
    /// Unlike [`run`](BackendExt::run), the returned future will only complete
    /// once *all* the detached tasks spawned in the closure's body are done.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::{Duration, Instant};
    /// # use mock::{BackendExt, Mock};
    /// # use mock::fs::MockFs;
    /// #
    /// # futures_lite::future::block_on(async {
    /// let start = Instant::now();
    ///
    /// Mock::<MockFs>::default()
    ///     .run_all(async |ctx| {
    ///         ctx.spawn_local(async |_| {
    ///             async_io::Timer::after(Duration::from_secs(2)).await;
    ///         })
    ///         .detach();
    ///     })
    ///     .await;
    ///
    /// // Even though the async closure we passed to `run_all()` completes
    /// // immediately, the returned future will wait for the timer to expire.
    /// assert!(start.elapsed() >= Duration::from_secs(2));
    /// # });
    /// ```
    #[inline]
    fn run_all<T: 'static>(
        self,
        fun: impl AsyncFnOnce(&mut Context<Self>) -> T + 'static,
    ) -> impl Future<Output = T> {
        self.run_inner(fun, true)
    }

    #[inline]
    #[doc(hidden)]
    fn block_on_inner<T>(
        self,
        fun: impl AsyncFnOnce(&mut Context<Self>) -> T,
        block_on_all: bool,
    ) -> T {
        let fun = async move |ctx: &mut Context<Self>| {
            Box::into_raw(Box::new(fun(ctx).await)) as *mut ()
        };

        // SAFETY: we're blocking the current thread on the future, so we
        // can "extend" the lifetimes of the async function without the caller
        // being able to pull the rug out from under us.
        let extended = unsafe { extend_lifetime(fun) };

        let out = future::block_on(self.run_inner(extended, block_on_all));

        // SAFETY: the function is only called once and the pointer was created
        // by a call to `Box::into_raw`.
        *unsafe { Box::from_raw(out as *mut T) }
    }

    #[inline]
    #[doc(hidden)]
    fn run_inner<T: 'static>(
        self,
        fun: impl AsyncFnOnce(&mut Context<Self>) -> T + 'static,
        run_all: bool,
    ) -> impl Future<Output = T> {
        let (runner, task) = Backend::with_ctx(self, move |ctx| {
            let task = ctx.spawn_local_unprotected(fun);
            let runner =
                ctx.with_editor(|ed| ed.executor().runner().as_mut().clone());
            (runner, task)
        });

        async move { runner.run_inner(task, run_all).await }
    }
}

unsafe fn extend_lifetime<B: Backend, T: 'static>(
    fun: impl for<'a> AsyncFnOnce(&'a mut Context<B>) -> T,
) -> impl for<'a> AsyncFnOnce(&'a mut Context<B>) -> T + 'static {
    use core::marker::PhantomData;
    use core::pin::Pin;

    struct Boxed<'a, F>(F, PhantomData<&'a ()>);

    impl<'a, T, U, F> AsyncFnOnce<(T,)> for Boxed<'a, F>
    where
        F: AsyncFnOnce(T) -> U,
        F::CallOnceFuture: 'a,
    {
        type CallOnceFuture = Pin<Box<dyn Future<Output = U> + 'a>>;
        type Output = U;

        #[inline]
        extern "rust-call" fn async_call_once(
            self,
            args: (T,),
        ) -> Self::CallOnceFuture {
            Box::pin(self.0(args.0))
        }
    }

    #[allow(clippy::type_complexity)]
    struct TypeErased<'a, T, U>(
        Box<
            dyn AsyncFnOnce<
                    (T,),
                    CallOnceFuture = Pin<Box<dyn Future<Output = U> + 'a>>,
                    Output = U,
                > + 'a,
        >,
    );

    impl<'a, T, U> AsyncFnOnce<(T,)> for TypeErased<'a, T, U> {
        type CallOnceFuture = Pin<Box<dyn Future<Output = U> + 'a>>;
        type Output = U;

        #[inline]
        extern "rust-call" fn async_call_once(
            self,
            args: (T,),
        ) -> Self::CallOnceFuture {
            self.0(args.0)
        }
    }

    let boxed = Boxed(
        async move |args: *mut ()| {
            // SAFETY: the pointer points to a Context<T>, we just cast it
            // to `*mut ()` to type-erase the async closure's input.
            let args = unsafe { &mut *(args as *mut Context<B>) };
            fun(args).await
        },
        PhantomData,
    );

    // SAFETY: up to the caller.
    let erased = unsafe {
        mem::transmute::<
            TypeErased<'_, *mut (), T>,
            TypeErased<'static, *mut (), T>,
        >(TypeErased(Box::new(boxed)))
    };

    async move |args: &mut Context<B>| {
        erased(args as *mut Context<B> as *mut ()).await
    }
}

impl<B> BackendExt for B where
    B: Backend<Executor: Executor<Runner: AsMut<crate::executor::Runner>>>
{
}
