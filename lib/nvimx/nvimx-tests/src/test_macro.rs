use core::fmt;
use core::future::Future;
use std::path::PathBuf;

use nvim_oxi::{IntoResult, TestTerminator};

/// Returns the path where [`build`](crate::build) places the compiled dynamic
/// library for the given crate.
///
/// This function is not part of the crate's public API and should only be used
/// by the `#[nvimx::test]` macro.
pub fn library_path(crate_name: &str) -> PathBuf {
    library_path::path(crate_name)
}

/// Runs an async function annotated with `#[nvimx::test]` to completion.
///
/// This function is not part of the crate's public API and should only be used
/// by the `#[nvimx::test]` macro.
pub fn run_async_test<F, R>(terminator: TestTerminator, test_body: F)
where
    F: Future<Output = R> + 'static,
    R: IntoResult<(), Error: fmt::Display>,
{
    run_async_test::run(terminator, test_body)
}

mod library_path {
    use std::env::consts;

    use super::*;
    use crate::build;

    pub(super) fn path(crate_name: &str) -> PathBuf {
        let library_name = format!(
            "{prefix}{crate_name}{suffix}",
            prefix = consts::DLL_PREFIX,
            suffix = consts::DLL_SUFFIX,
        );

        build::target_dir()
            .join(build::BuildProfile::current().as_str())
            .join(library_name)
    }
}

mod run_async_test {
    use std::cell::OnceCell;
    use std::convert::Infallible;
    use std::panic;
    use std::sync::{Arc, Mutex};

    use nvim_oxi::TestFailure;
    use nvimx_executor::Executor;

    use super::*;

    thread_local! {
        static EXECUTOR: OnceCell<Executor> = const { OnceCell::new() };
    }

    pub(super) fn run<F, R>(terminator: TestTerminator, test_body: F)
    where
        F: Future<Output = R> + 'static,
        R: IntoResult<(), Error: fmt::Display>,
    {
        let terminator = Terminator::new(terminator);

        {
            let terminator = terminator.clone();

            panic::set_hook(Box::new(move |infos| {
                let terminator = terminator
                    .into_inner()
                    .expect("terminate has not been called");

                terminator
                    .terminate::<Infallible>(Err(TestFailure::Panic(infos)));
            }));
        }

        let future = async move {
            let res = test_body.await.into_result();

            let terminator = terminator
                .into_inner()
                .expect("terminate has not been called");

            match res {
                Ok(()) => terminator.terminate::<Infallible>(Ok(())),
                Err(err) => terminator.terminate(Err(TestFailure::Error(err))),
            }
        };

        EXECUTOR.with(|ex| {
            ex.get_or_init(Executor::register).spawn(future).detach()
        });
    }

    #[derive(Clone)]
    struct Terminator {
        inner: Arc<Mutex<*mut TestTerminator>>,
    }

    unsafe impl Send for Terminator {}
    unsafe impl Sync for Terminator {}

    impl Terminator {
        #[allow(clippy::unwrap_used, clippy::wrong_self_convention)]
        fn into_inner(&self) -> Option<TestTerminator> {
            let mut ptr = self.inner.lock().unwrap();
            let ptr = std::mem::replace(&mut *ptr, std::ptr::null_mut());
            if ptr.is_null() {
                None
            } else {
                Some(unsafe { *Box::from_raw(ptr) })
            }
        }

        #[allow(clippy::arc_with_non_send_sync)]
        fn new(test_terminator: TestTerminator) -> Self {
            let terminator = Box::into_raw(Box::new(test_terminator));
            let inner = Arc::new(Mutex::new(terminator));
            Self { inner }
        }
    }
}
