//! The lifecycle context and associated utilities used by application hooks.
//!
//! When the [`Runtime`](crate::Runtime) starts, it creates a `LifecycleContext` that is passed to
//! all lifecycle hooks. This context provides a way for hooks to poll the current state of the
//! application to prevent blocking the shutdown process.
//!
//! The following utilities are provided to said purpose:
//! - [`LifecycleContext::wait_for_shutdown`] and [`LifecycleContext::wait_for_shutdown_owned`] which
//!   allow hooks to wait for a shutdown signal (i.e. for axum server graceful shutdown).
//! - [`LifecycleContext::interval`] and [`LifecycleContext::interval_at`] which allow hooks to run
//!   periodic tasks while respecting the shutdown signal.
//! - [`LifecycleContext::wrap_stream`] which wraps a stream to terminate when a shutdown signal is
//!   received.
//!
//! Additionally, the [`LifecycleContext::notify_error`] method allows hooks to notify the runtime
//! of a fatal error, which will trigger an immediate graceful shutdown of the entire application.
//!

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::select;
use tokio::sync::Notify;
use tokio::task::{AbortHandle, JoinSet};
use tokio::time::{Instant, sleep_until};
use tracing::debug;

mod interval;
mod shutdown;
mod stream;

pub use interval::LifecycleInterval;
pub use shutdown::{Shutdown, ShutdownOwned};
pub use stream::LifecycleStream;

/// The context passed to lifecycle hooks.
///
/// Provides methods for spawning background tasks, receiving periodic intervals,
/// wrapping Tokio streams, and triggering a graceful shutdown of the application.
///
/// See [`LifecycleContext::notify_error`] to initiate an immediate shutdown from within a hook.
#[derive(Clone, Debug)]
pub struct LifecycleContext {
    notify: Arc<Notify>,
    shutdown: Arc<AtomicBool>,
}

pub(crate) struct LifecycleContextManager {
    ctx: LifecycleContext,
    join_set: JoinSet<()>,
    timeouts: BTreeMap<Duration, Vec<AbortHandle>>,
}

impl LifecycleContext {
    /// Notify the lifecycle context of a fatal error, triggering an immediate graceful shutdown
    /// of the entire application.
    ///
    /// This is typically called from within a lifecycle hook when an unrecoverable error occurs,
    /// such as a connection failure or a critical configuration issue. Once called, all background
    /// tasks are signalled to stop and the runtime waits for them to complete (or times out).
    ///
    /// # Behaviour
    ///
    /// This method is idempotent — calling it multiple times from different hooks has the same
    /// effect as a single call. The first caller wins.
    #[inline]
    pub fn notify_error(&self) {
        self.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Notify the lifecycle context of a shutdown request, which will initiate a graceful shutdown
    /// of the application. This operation is idempotent, and multiple calls to this method will
    /// have no effect after the first call.
    fn shutdown(&self) {
        let already_shutdown = self.shutdown.swap(true, Ordering::AcqRel);
        if already_shutdown {
            debug!("Shutdown already initiated, ignoring duplicate shutdown request");
            return;
        }
        self.notify.notify_waiters();
    }
}

impl LifecycleContextManager {
    pub(super) fn new() -> Self {
        let ctx = LifecycleContext {
            notify: Arc::new(Notify::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        Self {
            ctx,
            join_set: JoinSet::new(),
            timeouts: BTreeMap::new(),
        }
    }

    /// Spawn a new task within the lifecycle context.
    /// The task will be managed by the lifecycle context and will be awaited during shutdown.
    /// If a timeout is provided, the task will be aborted if it does not complete within the specified duration.
    pub(super) fn spawn<F, Fut>(&mut self, task: &F, timeout: Option<Duration>)
    where
        F: Fn(LifecycleContext) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let abort_handle = self.join_set.spawn(task(self.get_context()));
        if let Some(timeout) = timeout {
            self.timeouts.entry(timeout).or_default().push(abort_handle);
        }
    }

    pub(crate) fn get_context(&self) -> LifecycleContext {
        self.ctx.clone()
    }

    /// Wait for any panicked tasks to finish and return true if any panicked, false otherwise.
    pub(crate) async fn wait_for_panicked_task(&mut self) -> bool {
        while let Some(join_result) = self.join_set.join_next().await {
            let Err(join_error) = join_result else {
                continue;
            };

            if join_error.is_panic() {
                return true;
            }
        }

        false
    }

    /// Shutdown the lifecycle context, waiting for all tasks to finish, aborting any that exceed
    /// their timeout.
    pub(crate) async fn shutdown(&mut self) {
        let Self {
            join_set, timeouts, ..
        } = self;

        self.ctx.shutdown();
        debug!("Waiting for {} tasks to finish", join_set.len());

        // Wait for all tasks to finish or abort any that exceed their timeout.
        select! {
            _ = Self::join_tasks_task(join_set) => {},
            _ = Self::timeout_abort_task(timeouts) => {},
        }

        // Finally, wait for any remaining tasks to finish after aborting those that exceeded their timeout.
        Self::join_tasks_task(join_set).await;
    }

    /// Abort any tasks that exceed their timeout.
    async fn timeout_abort_task(timeouts: &BTreeMap<Duration, Vec<AbortHandle>>) {
        let now = Instant::now();

        for (timeout, abort_handles) in timeouts {
            sleep_until(now + *timeout).await;

            let abort_count = abort_handles
                .iter()
                .filter(|handle| !handle.is_finished()) // Only abort tasks that are still running
                .map(AbortHandle::abort)
                .count();
            if abort_count > 0 {
                debug!(
                    "{} tasks exceeded their timeout of {:?} and were aborted",
                    abort_count, timeout
                );
            }
        }
    }

    /// Wait for all tasks to finish, ignoring any panics.
    async fn join_tasks_task(join_set: &mut JoinSet<()>) {
        while join_set.join_next().await.is_some() {}
    }
}
