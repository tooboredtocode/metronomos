//! Application startup and shutdown lifecycle management.
//!
//! The [`Lifecycle`] handle allows dependencies registered in the DI container to
//! register hooks that are invoked at application startup and cleaned up on shutdown.
//! Each hook receives a [`LifecycleContext`] that provides utilities for spawning
//! long-lived tasks, receiving periodic intervals, wrapping streams, and triggering
//! a graceful shutdown from anywhere within a hook.

use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{self};
use tracing::error;

pub mod context;
mod hook;
mod inner;

/// The context passed to lifecycle hooks.
///
/// Provides methods for spawning background tasks, receiving periodic intervals,
/// wrapping Tokio streams, and triggering a graceful shutdown of the application.
///
/// See [`LifecycleContext::notify_error`] to initiate an immediate shutdown from within a hook.
pub use context::LifecycleContext;
use hook::LifeCycleHook;
pub(crate) use inner::LifecycleInner;
use metronomos_pulse::value::CustomPulseValue;

/// A cloneable handle for registering application lifecycle hooks.
///
/// This handle is provided as a dependency by the DI container and is used to
/// register async callbacks that run at application startup and run until shutdown.
#[derive(Clone)]
#[repr(transparent)]
pub struct Lifecycle {
    sink: mpsc::Sender<LifeCycleHook>,
}

impl CustomPulseValue for Lifecycle {
    const NAME: &'static str = "metronomos::Lifecycle";
}

impl Lifecycle {
    /// Register an async lifecycle hook to be executed at application startup.
    ///
    /// The `run` closure is invoked with a [`LifecycleContext`] when the runtime starts.
    /// Use the context to spawn long-running background tasks, register periodic intervals,
    /// or wrap Tokio streams. All spawned tasks are tracked and awaited during graceful shutdown.
    pub fn register_hook<F, Fut>(&self, run: F)
    where
        F: Fn(LifecycleContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let hook = LifeCycleHook::new(run);

        if let Err(err) = self.sink.try_send(hook) {
            match err {
                TrySendError::Full(_) => {
                    error!(
                        "Lifecycle hook queue is full. Providing this many hooks is not supported!"
                    );
                }
                TrySendError::Closed(_) => {
                    unreachable!("Lifecycle hook queue is closed. This should never happen!");
                }
            }
        }
    }
}
