use std::future::poll_fn;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::time::{self, Instant, Interval, MissedTickBehavior};

use crate::lifecycle::context::{LifecycleContext, ShutdownOwned};

impl LifecycleContext {
    /// Creates a new [`LifecycleInterval`] that will tick at the specified duration,
    /// and will terminate on shutdown. See [`tokio::time::interval`] for more details.
    #[inline]
    pub fn interval(&self, duration: Duration) -> LifecycleInterval {
        self.wrap_interval(time::interval(duration))
    }

    /// Creates a new [`LifecycleInterval`] that will tick at the specified start time and duration,
    /// and will terminate on shutdown. See [`tokio::time::interval_at`] for more details.
    #[inline]
    pub fn interval_at(&self, start: Instant, duration: Duration) -> LifecycleInterval {
        self.wrap_interval(time::interval_at(start, duration))
    }

    fn wrap_interval(&self, interval: Interval) -> LifecycleInterval {
        LifecycleInterval {
            shutdown: Box::pin(self.wait_for_shutdown_owned()),
            inner: interval,
        }
    }
}

/// A [`tokio::time::Interval`] that terminates on lifecycle shutdown.
///
/// Like a regular interval, this type produces ticks at regular time intervals. Unlike
/// a regular interval, it automatically terminates when the associated lifecycle context
/// shuts down (via [`notify_error`][crate::lifecycle::LifecycleContext::notify_error]).
pub struct LifecycleInterval {
    shutdown: Pin<Box<ShutdownOwned>>,
    inner: Interval,
}

impl LifecycleInterval {
    /// Completes when the next tick of the interval is reached, or when the context is shut down.
    /// Returns the instant at which the tick occurred, or `None` if the context was shut down
    /// before the tick could be completed. See [`tokio::time::Interval::tick`] for more details.
    pub async fn tick(&mut self) -> Option<Instant> {
        let instant = poll_fn(|cx| self.poll_tick(cx));

        instant.await
    }

    /// Polls for the next instant in the interval to be reached.
    /// Returns
    /// - `Poll::Ready(Some(instant))` when the next tick is reached,
    /// - `Poll::Ready(None)` if the context is shut down before the tick can be completed, or
    /// - `Poll::Pending` if the next tick has not yet been reached.
    ///
    /// See [`tokio::time::Interval::poll_tick`] for more details.
    pub fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Option<Instant>> {
        if self.shutdown.as_mut().poll(cx).is_ready() {
            return Poll::Ready(None);
        }

        self.inner.poll_tick(cx).map(Some)
    }

    /// Resets the interval to complete one period after the current time.
    /// See [`tokio::time::Interval::reset`] for more details.
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Resets the interval immediately. See [`tokio::time::Interval::reset_immediately`] for more
    /// details.
    #[inline]
    pub fn reset_immediately(&mut self) {
        self.inner.reset_immediately();
    }

    /// Resets the interval after the specified [`std::time::Duration`].
    /// See [`tokio::time::Interval::reset_after`] for more details.
    #[inline]
    pub fn reset_after(&mut self, duration: Duration) {
        self.inner.reset_after(duration);
    }

    /// Resets the interval to a [`tokio::time::Instant`] deadline.
    /// See [`tokio::time::Interval::reset_at`] for more details.
    #[inline]
    pub fn reset_at(&mut self, deadline: Instant) {
        self.inner.reset_at(deadline);
    }

    /// Returns the [`MissedTickBehavior`] strategy currently being used.
    pub fn missed_tick_behavior(&self) -> MissedTickBehavior {
        self.inner.missed_tick_behavior()
    }

    /// Sets the [`MissedTickBehavior`] strategy that should be used.
    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
        self.inner.set_missed_tick_behavior(behavior);
    }

    /// Returns the period of the interval.
    pub fn period(&self) -> Duration {
        self.inner.period()
    }
}
