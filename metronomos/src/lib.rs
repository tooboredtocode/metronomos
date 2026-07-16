//! A dependency-injected runtime for building async Rust applications.
//!
//! `metronomos` wraps [`PulseContainer`], a type-safe dependency injection
//! container from `metronomos-pulse`, and adds **lifecycle management** — starting and stopping async hooks
//! with graceful shutdown support.
//!

use std::time::Duration;

use metronomos_pulse::PulseContainer;
use metronomos_pulse::container::PulseContext;
use tokio::signal::ctrl_c;
use tracing::{Span, error, info};

pub mod builder;
pub mod lifecycle;

use lifecycle::LifecycleInner;

/// The main application runtime that manages dependency injection and lifecycle hooks.
///
/// Create a `Runtime` using [`Runtime::builder()`], configure dependencies via the builder,
/// then call [`run`][Self::run] to start the application.
pub struct Runtime {
    pub(crate) container: PulseContainer,
    pub(crate) lifecycle: LifecycleInner,
    pub(crate) span: Span,
}

impl Runtime {
    /// Returns the dependency injection [`PulseContext`] for resolving registered dependencies.
    ///
    /// Use this method to resolve values that were registered with the builder at runtime:
    pub fn context(&self) -> PulseContext<'_> {
        self.container.context()
    }

    async fn wait_for_shutdown_signal() {
        ctrl_c().await.expect("Failed to listen for Ctrl+C");
    }

    /// Run the runtime: start all registered lifecycle hooks and wait for a shutdown signal.
    ///
    /// This method performs the following steps:
    ///
    /// 1. Starts all lifecycle hooks registered via the builder in dependency order.
    /// 2. Waits for either a shutdown signal (Ctrl+C) or a panic in any hook task.
    /// 3. Gracefully shuts down all hooks, waiting up to `shutdown_timeout` for each to complete.
    pub async fn run(&mut self, shutdown_timeout: Option<Duration>) {
        let Self {
            lifecycle, span, ..
        } = self;

        info!(
            parent: span.id(),
            "Runtime started, starting all lifecycle hooks..."
        );

        let mut ctx = lifecycle.start_hooks();

        info!(
            parent: span.id(),
            "All lifecycle hooks started, waiting for signal to stop..."
        );
        tokio::select!(
            _ = Self::wait_for_shutdown_signal() => {
                info!(
                    parent: span.id(),
                    "Signal received, stopping all lifecycle hooks..."
                );
                ctx.shutdown(shutdown_timeout).await;
            },
            panicked = ctx.wait_for_panicked_task() => {
                if panicked {
                    error!(
                        parent: span.id(),
                        "One or more lifecycle hooks panicked, shutting down..."
                    );
                    ctx.shutdown(shutdown_timeout).await;
                } else {
                    info!(
                        parent: span.id(),
                        "All lifecycle hooks completed successfully, shutting down..."
                    );
                }
            }
        );
    }
}
