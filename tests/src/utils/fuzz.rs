use core::cell::Cell;
use core::convert::Infallible;
use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::{env, panic, sync};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;

#[track_caller]
pub(crate) fn run<T: FuzzResult>(fun: impl FnOnce(&mut ChaChaRng) -> T) {
    let seed = seed();
    set_panic_hook(seed);
    let mut rng = ChaChaRng::seed_from_u64(seed);
    if let Err(err) = fun(&mut rng).into_result() {
        panic!("{err}");
    }
}

pub(crate) fn run_async<T: FuzzResult>(
    fun: impl AsyncFnOnce(&mut ChaChaRng) -> T,
) -> impl Future<Output = ()> {
    TrackCaller::new(async move {
        let seed = seed();
        set_panic_hook(seed);
        let mut rng = ChaChaRng::seed_from_u64(seed);
        if let Err(err) = fun(&mut rng).await.into_result() {
            panic!("{err}");
        }
    })
}

#[track_caller]
fn seed() -> u64 {
    match env::var("SEED") {
        Ok(seed) => seed.parse().expect("couldn't parse $SEED"),
        Err(env::VarError::NotPresent) => rand::random(),
        Err(env::VarError::NotUnicode(seed)) => {
            panic!("$SEED contained invalid unicode: {seed:?}")
        },
    }
}

fn set_panic_hook(seed: u64) {
    thread_local! {
        static FUZZ_SEED: Cell<Option<u64>> = const { Cell::new(None) };
    }

    FUZZ_SEED.with(|s| s.replace(Some(seed)));

    // Make sure to only set the hook once, even if multiple tests are run in
    // parallel.
    static SET_HOOK: sync::Once = sync::Once::new();
    SET_HOOK.call_once(|| {
        let prev_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let seed = FUZZ_SEED.with(|s| s.get().expect("seed has been set"));
            eprintln!("fuzz run failed with seed {seed}");
            prev_hook(info);
        }));
    });
}

/// A trait for the result of a fuzz run.
///
/// It's only implemented for `()` and `Result<(), E>` where `E: Display`.
pub(crate) trait FuzzResult {
    #[doc(hidden)]
    type Error: fmt::Display;

    #[doc(hidden)]
    fn into_result(self) -> Result<(), Self::Error>;
}

impl FuzzResult for () {
    type Error = Infallible;

    fn into_result(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<E: fmt::Display> FuzzResult for Result<(), E> {
    type Error = E;

    fn into_result(self) -> Result<(), Self::Error> {
        self
    }
}

impl FuzzResult for never::Never {
    type Error = Infallible;

    fn into_result(self) -> Result<(), Self::Error> {
        unreachable!()
    }
}

mod never {
    pub(crate) type Never = <fn() -> ! as FnRet>::Output;

    pub(crate) trait FnRet {
        type Output;
    }

    impl<R> FnRet for fn() -> R {
        type Output = R;
    }
}

pin_project_lite::pin_project! {
    /// A [`Future`] wrapper that makes it possible to track the caller of an
    /// async function on stable.
    ///
    /// See https://github.com/rust-lang/rust/issues/110011 for more infos.
    struct TrackCaller<T> {
        #[pin]
        inner: T,
    }
}

impl<T> TrackCaller<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<Fut: Future> Future for TrackCaller<Fut> {
    type Output = Fut::Output;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}
