use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use std::time::Instant;

use nvim::libuv;
use pin_project_lite::pin_project;

/// TODO: docs.
#[inline]
pub fn sleep(duration: Duration) -> Sleep {
    let sleep_until = match Instant::now().checked_add(duration) {
        Some(instant) => instant,
        None => far_future(),
    };
    Sleep::new(sleep_until)
}

/// Roughly 30 years from now.
///
/// This was lifted directly from tokio's `Instant::far_future()` impl.
#[inline(always)]
fn far_future() -> Instant {
    Instant::now() + Duration::from_secs(86400 * 365 * 30)
}

pin_project! {
    /// TODO: docs
    pub struct Sleep {
        has_completed: bool,
        sleep_until: Instant,
    }
}

impl Sleep {
    #[inline]
    fn new(sleep_until: Instant) -> Self {
        Self { has_completed: false, sleep_until }
    }
}

impl Future for Sleep {
    type Output = ();

    #[inline]
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<()> {
        let this = self.project();

        if *this.has_completed {
            panic!("Sleep future polled after completion");
        }

        let Some(left_to_sleep) =
            this.sleep_until.checked_duration_since(Instant::now())
        else {
            *this.has_completed = true;
            return Poll::Ready(());
        };

        let waker = ctx.waker().clone();

        let _ = libuv::TimerHandle::once(left_to_sleep, || {
            waker.wake();
            Ok::<_, core::convert::Infallible>(())
        })
        .unwrap();

        Poll::Pending
    }
}
