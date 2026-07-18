//! Builder for constructing a [`Runtime`].
//!
//! This module provides the [`RuntimeBuilder`] which collects dependency registrations
//! and lifecycle configuration before building the final runtime. See
//! [`Runtime::builder()`](crate::Runtime::builder) and [`Runtime::new_with()`](crate::Runtime::new_with)
//! for the primary entry points.

use std::time::Duration;

use metronomos_pulse::PulseContainer;
use metronomos_pulse::builder::{ProvideError, ProvideValueError, PulseContainerBuilder};
use metronomos_pulse::dependency::{AsyncFnDependency, FnDependency};
use metronomos_pulse::error::PulseError;
use metronomos_pulse::value::PulseValue;

use crate::Runtime;
use crate::lifecycle::LifecycleInner;

/// Builder for constructing a [`Runtime`].
///
/// Use this struct to register dependencies via [`provide`][Self::provide],
/// [`provide_async`][Self::provide_async], or value registration methods, then call
/// [`build`](Self::build) to resolve the dependency graph and produce the runtime.
pub struct RuntimeBuilder {
    pub(crate) container_builder: PulseContainerBuilder,
    pub(crate) lifecycle: LifecycleInner,
}

impl Runtime {
    /// Creates a new [`RuntimeBuilder`] for constructing a [`Runtime`].
    ///
    /// This is the primary entry point for building a runtime. The builder collects
    /// dependency registrations and lifecycle configuration, which are then finalized
    /// by calling [`RuntimeBuilder::build`](crate::builder::RuntimeBuilder::build).
    pub fn builder() -> RuntimeBuilder {
        let mut container_builder = PulseContainer::builder();
        let lifecycle = LifecycleInner::new();
        _ = container_builder.provide(lifecycle.as_provide());

        RuntimeBuilder {
            container_builder,
            lifecycle,
        }
    }

    /// Creates a new [`Runtime`] using an initializer closure.
    ///
    /// This is a convenience constructor that combines builder creation, initialization,
    /// and building into a single call. The initializer receives a mutable reference to the
    /// builder and can register dependencies or configure lifecycle hooks.
    pub async fn new_with(
        initializer: impl FnOnce(&mut RuntimeBuilder) -> Result<(), ProvideError>,
    ) -> Result<Self, PulseError> {
        let mut builder = Runtime::builder();
        initializer(&mut builder)?;
        builder.build().await
    }
}

impl RuntimeBuilder {
    /// Sets the default timeout for lifecycle hooks.
    ///
    /// By default, lifecycle hooks which do not specify a timeout will be immediately aborted on
    /// shutdown. This method allows you to extend the default timeout for all hooks that do not
    /// specify a timeout explicitly.
    #[inline]
    pub fn with_lifecycle_timeout(mut self, timeout: Duration) -> Self {
        self.lifecycle.set_default_timeout(timeout);
        self
    }

    /// Applies a function to the runtime builder for registering multiple dependencies at once.
    ///
    /// This method is useful when you need to register several related dependencies and want
    /// to handle errors collectively. The closure receives a mutable reference to the builder,
    /// allowing it to call any [`RuntimeBuilder`] method.
    #[inline]
    pub fn provide_with<F>(&mut self, fun: F) -> Result<(), ProvideError>
    where
        F: FnOnce(&mut RuntimeBuilder) -> Result<(), ProvideError>,
    {
        fun(self)
    }

    /// Provides a sync function dependency to the runtime builder.
    ///
    /// The provided function can take other dependencies as arguments and returns a value that will be
    /// stored in the container for later retrieval. Input types are resolved from the dependency graph
    /// automatically, so you only need to provide the function itself.
    pub fn provide<F, Dep>(&mut self, fun: F) -> Result<(), ProvideError>
    where
        F: FnDependency<Dep>,
    {
        self.container_builder.provide(fun)
    }

    /// Provides an async function dependency to the runtime builder.
    ///
    /// The provided async function can take other dependencies as arguments and returns a value that will be
    /// stored in the container for later retrieval. Use this when construction requires asynchronous I/O
    /// or awaiting other futures.
    pub fn provide_async<F, Dep>(&mut self, fun: F) -> Result<(), ProvideError>
    where
        F: AsyncFnDependency<Dep>,
        Dep: Send + Sync + 'static,
    {
        self.container_builder.provide_async(fun)
    }

    /// Provides a concrete value to the runtime.
    ///
    /// Use this method when the value does not depend on other dependencies and requires no initialization.
    /// The type must implement [`PulseValue`], which is typically derived via `#[derive(PulseValue)]`.
    pub fn provide_value<V: PulseValue>(&mut self, value: V) -> Result<(), ProvideValueError<V>> {
        self.container_builder.provide_value(value)
    }

    /// Provides a value that will be stored as an [`ArcValue`](metronomos_pulse::value::ArcValue) in the
    /// runtime. Useful for dependencies that do not require any other dependencies and/or
    /// initialization but do not implement [`PulseValue`] (e.g. types that do not implement `Clone`).
    ///
    /// ## Note
    /// If your type implements [`PulseValue`] (or can implement it), you should use
    /// [`provide_value`](Self::provide_value) instead.
    pub fn provide_arc_value<T: Send + Sync + 'static>(
        &mut self,
        value: T,
    ) -> Result<(), ProvideValueError<T>> {
        self.container_builder.provide_arc_value(value)
    }

    /// Builds the runtime and initializes all registered dependencies.
    ///
    /// This method resolves the dependency graph, initializes each chunk of dependencies
    /// in the correct order, and produces the final [`Runtime`]. After calling this,
    /// the builder is consumed and cannot be reused.
    pub async fn build(self) -> Result<Runtime, PulseError> {
        let container = self.container_builder.build().await?;
        Ok(Runtime {
            container,
            lifecycle: self.lifecycle,
        })
    }
}
