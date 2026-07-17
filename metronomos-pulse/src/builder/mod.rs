//! Types for building a [`PulseContainer`] with dependencies.
//!

use futures::future;
use metronomos_loom::DependencyGraph;
use metronomos_loom::builder::DependencyGraphBuilder;
use metronomos_loom::dependency::DependencyKeyKind;
use tracing::{debug, instrument};

use crate::PulseContainer;
use crate::dependency::{
    ArcValueDependency, AsyncFnDependency, FnDependency, PulseDependency, ValueDependency,
};
use crate::error::PulseError;
use crate::value::{ArcValue, DotString, PulseValue};

mod error;

pub use error::{ProvideError, ProvideValueError};

/// Builder for constructing a [`PulseContainer`].
///
/// Use this struct to register dependencies in the form of sync functions, async functions, or concrete values.
/// Once all dependencies are registered, call [`build`](Self::build) to resolve the dependency graph
/// and instantiate every value in the correct initialization order.
pub struct PulseContainerBuilder {
    dep: DependencyGraphBuilder<PulseDependency>,
}

impl PulseContainer {
    /// Creates a new empty builder for constructing a [`PulseContainer`].
    ///
    /// The builder collects dependency registrations and is then used via
    /// [`build`](PulseContainerBuilder::build) to produce the final container.
    pub fn builder() -> PulseContainerBuilder {
        PulseContainerBuilder {
            dep: DependencyGraph::builder(),
        }
    }
}

macro_rules! log_provide {
    ($value:ty, sync $provide_fn_sync:ident) => {
        log_provide!($value, sync $provide_fn_sync, async);
    };
    ($value:ty, async $provide_fn_async:ident) => {
        log_provide!($value, sync, async $provide_fn_async);
    };
    ($value:ty, sync $($provide_fn_sync:ident)?, async $($provide_fn_async:ident)?) => {
        if <$value as PulseValue>::IS_FINALIZER {
            debug!(
                $( finalizer = std::any::type_name::<$provide_fn_sync>(), )?
                $( async_finalizer = std::any::type_name::<$provide_fn_async>(), )?
                "provide",
            );
        } else {
            debug!(
                value = <$value as PulseValue>::name(),
                $( constructor = std::any::type_name::<$provide_fn_sync>(), )?
                $( async_constructor = std::any::type_name::<$provide_fn_async>(), )?
                "provide"
            );
        }
    };
}

impl PulseContainerBuilder {
    /// Provides a sync function dependency to the container builder.
    ///
    /// The provided function can take other dependencies as arguments and returns a value that will be
    /// stored in the container for later retrieval. Input types are resolved from the graph automatically,
    /// so you only need to provide the function itself.
    pub fn provide<F, Dep>(&mut self, fun: F) -> Result<(), ProvideError>
    where
        F: FnDependency<Dep>,
    {
        log_provide!(F::Value, sync F);
        self.dep.add_dependency(PulseDependency::new_fun(fun))?;
        Ok(())
    }

    /// Provides an async function dependency to the container builder.
    ///
    /// The provided async function can take other dependencies as arguments and returns a value that will be
    /// stored in the container for later retrieval. Use this when construction requires asynchronous I/O
    /// or awaiting other futures.
    pub fn provide_async<F, Dep>(&mut self, fun: F) -> Result<(), ProvideError>
    where
        F: AsyncFnDependency<Dep>,
        Dep: Send + Sync + 'static,
    {
        log_provide!(F::Value, async F);
        self.dep
            .add_dependency(PulseDependency::new_async_fun(fun))?;
        Ok(())
    }

    /// Provides a concrete value to the container.
    ///
    /// Use this method when the value does not depend on other dependencies and requires no initialization.
    /// The type must implement [`PulseValue`], which is typically derived via `#[derive(PulseValue)]`.
    pub fn provide_value<V: PulseValue>(&mut self, value: V) -> Result<(), ProvideValueError<V>> {
        debug!(value = V::name(), "provide_value");
        self.dep.add_dependency(ValueDependency(value))?;
        Ok(())
    }

    /// Provides a concrete value that will be stored as an [`ArcValue`] in the container.
    ///
    /// Use this method when the value does not depend on other dependencies and cannot implement
    /// [`PulseValue`] (e.g., types that do not implement `Clone`). The value is wrapped in an `Arc`
    /// internally, so it can be cloned cheaply.
    ///
    /// # Note
    ///
    /// If your type implements [`PulseValue`] (or can implement it), you should prefer
    /// [`provide_value`](Self::provide_value) instead for better ergonomics.
    pub fn provide_arc_value<T: Send + Sync + 'static>(
        &mut self,
        value: T,
    ) -> Result<(), ProvideValueError<T>> {
        debug!(value = ArcValue::<T>::name(), "provide_value");
        self.dep.add_dependency(ArcValueDependency(value))?;
        Ok(())
    }

    /// Builds the container and initializes all dependencies.
    #[instrument(skip(self), name = "build_container")]
    pub async fn build(self) -> Result<PulseContainer, PulseError> {
        let Self { dep } = self;

        debug!("Building dependency graph");
        let graph = dep.build()?;
        let mut res = PulseContainer::new(DotString::make(&graph));

        for (num, chunk) in graph.init_chunks().enumerate() {
            debug!("Initializing dependency chunk {}", num + 1);

            let chunk_values = future::try_join_all(
                chunk
                    .into_iter()
                    .map(|dep| dep.inner().build(res.context())),
            )
            .await?;

            for (kind, value) in chunk_values {
                match kind {
                    DependencyKeyKind::Unique => {
                        let success = res.insert(value).is_ok();
                        debug_assert!(
                            success,
                            "There should not be a value with the same type_id already in the container"
                        );
                    }
                    DependencyKeyKind::Group => {
                        res.insert_group(value);
                    }
                }
            }
        }

        debug!("Dependency graph built and initialized successfully");

        Ok(res)
    }
}
