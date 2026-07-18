//! Application startup and shutdown lifecycle management.
//!
//! The [`Lifecycle`] handle allows dependencies registered in the DI container to
//! register hooks that are invoked at application startup and cleaned up on shutdown.
//! Each hook receives a [`LifecycleContext`] that provides utilities for spawning
//! long-lived tasks, receiving periodic intervals, wrapping streams, and triggering
//! a graceful shutdown from anywhere within a hook.

use std::mem::ManuallyDrop;
use std::time::Duration;

use metronomos_pulse::value::CustomPulseValue;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{self};
use tracing::error;

pub mod context;
mod hook;
mod inner;

pub use context::LifecycleContext;
use hook::LifecycleHook;
pub(crate) use inner::LifecycleInner;

/// A cloneable handle for registering application lifecycle hooks.
///
/// This handle is provided as a dependency by the DI container and is used to
/// register async callbacks that run at application startup and run until shutdown.
#[derive(Clone)]
#[repr(transparent)]
pub struct Lifecycle {
    sink: mpsc::Sender<LifecycleHook>,
}

/// A builder for registering a lifecycle hook.
///
/// This builder is returned by the [`Lifecycle::hook`] method and allows for additional
/// configuration of the hook before it is registered with the runtime.
pub struct LifecycleHookBuilder<'a> {
    lifecycle: &'a Lifecycle,
    hook: ManuallyDrop<LifecycleHook>,
}

impl CustomPulseValue for Lifecycle {
    const NAME: &'static str = "metronomos::Lifecycle";
}

impl Lifecycle {
    /// Register an async lifecycle hook to be executed at application startup, to be run during the
    /// lifecycle of the application until shutdown.
    ///
    /// The `run` closure is invoked with a [`LifecycleContext`] when the runtime starts.
    /// Use the context to gracefully stop on shutdown.
    pub fn hook<F, Fut>(&self, run: F) -> LifecycleHookBuilder<'_>
    where
        F: Fn(LifecycleContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        LifecycleHookBuilder {
            lifecycle: self,
            hook: ManuallyDrop::new(LifecycleHook::new(run)),
        }
    }

    fn register_hook_inner(&self, hook: LifecycleHook) {
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

impl<'a> LifecycleHookBuilder<'a> {
    /// Set a custom timeout for the lifecycle hook.
    ///
    /// If the hook does not complete within the specified duration, it will be forcefully terminated.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.hook.set_timeout(timeout);
        self
    }

    /// Disable the default timeout for the lifecycle hook.
    ///
    /// <div class="warning">
    ///
    /// Disabling the timeout can lead to the application hanging indefinitely if the hook does not complete.
    /// You should only disable the timeout if your hook respects the shutdown signal and will
    /// complete in a reasonable amount of time.
    ///
    /// </div>
    pub fn disable_timeout(mut self) -> Self {
        self.hook.disable_timeout();
        self
    }

    /// Register the lifecycle hook with the runtime.
    /// Alternatively, you can simply drop the builder to register the hook.
    pub fn register(self) {
        drop(self);
    }
}

impl Drop for LifecycleHookBuilder<'_> {
    fn drop(&mut self) {
        let hook = unsafe {
            // SAFETY: The builder is being dropped, so we can safely take ownership of the hook.
            ManuallyDrop::take(&mut self.hook)
        };
        self.lifecycle.register_hook_inner(hook);
    }
}
