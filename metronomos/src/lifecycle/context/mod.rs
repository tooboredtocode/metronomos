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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout_at};
use tracing::{debug, info};

mod interval;
mod shutdown;
mod stream;

pub use interval::LifecycleInterval;
pub use shutdown::{Shutdown, ShutdownOwned};
pub use stream::LifecycleStream;

/// A context handle passed to lifecycle hooks for managing background tasks and shutdown.
#[derive(Clone, Debug)]
pub struct LifecycleContext {
    notify: Arc<Notify>,
    shutdown: Arc<AtomicBool>,
}

pub(crate) struct LifecycleContextManager {
    ctx: LifecycleContext,
    join_set: JoinSet<()>,
}

impl LifecycleContext {
    /// Notify the lifecycle context of a fatal error, triggering an immediate graceful shutdown
    /// of the entire application.
    ///
    /// This is typically called from within a lifecycle hook when an unrecoverable error occurs,
    /// such as a connection failure or a critical configuration issue. Once called, all background
    /// tasks are signaled to stop and the runtime waits for them to complete (or times out).
    ///
    /// # Behavior
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
        }
    }

    /// Spawn a new task within the lifecycle context.
    /// The task will be managed by the lifecycle context and will be awaited during shutdown.
    pub(super) fn spawn<F, Fut>(&mut self, task: &F)
    where
        F: Fn(LifecycleContext) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.join_set.spawn(task(self.get_context()));
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

    /// Shutdown the lifecycle context, waiting for all tasks to finish.
    pub(crate) async fn shutdown(&mut self, timeout: Option<Duration>) {
        self.ctx.shutdown();
        match timeout {
            Some(duration) => self.shutdown_wait_with_timeout(duration).await,
            None => self.shutdown_wait_no_timeout().await,
        }
    }

    async fn shutdown_wait_no_timeout(&mut self) {
        info!("Waiting for {} tasks to finish", self.join_set.len());
        // We don't care about any panics in the tasks, we just want to wait for them to finish.
        while self.join_set.join_next().await.is_some() {}
    }

    async fn shutdown_wait_with_timeout(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        info!(
            "Waiting for {} tasks to finish with timeout of {:?}",
            self.join_set.len(),
            timeout
        );

        while let Ok(Some(_)) = timeout_at(deadline, self.join_set.join_next()).await {}

        // After the timeout, we will abort any remaining tasks.
        let remaining_tasks = self.join_set.len();
        if remaining_tasks > 0 {
            self.join_set.abort_all();
            info!("Timeout elapsed. Aborted {} tasks.", remaining_tasks);
        }
    }
}
