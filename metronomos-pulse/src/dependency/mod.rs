//! Dependency traits, types, and registration helpers.
//!
//! This module provides the core abstractions for how dependencies are described,
//! erased, and resolved within the [`PulseContainer`](crate::PulseContainer).
//!
//! # Overview
//!
//! Dependencies in Pulse come in two categories:
//!
//! - **Function dependencies** — Registered via [`PulseContainerBuilder::provide`](crate::builder::PulseContainerBuilder::provide) (sync) or
//!   [`provide_async`](crate::builder::PulseContainerBuilder::provide_async) (async). These are functions whose inputs
//!   are other resolved values and whose output is a new value. The traits
//!   [`FnDependency`] and [`AsyncFnDependency`] describe this pattern.
//! - **Value dependencies** — Registered via [`provide_value`](crate::builder::PulseContainerBuilder::provide_value) or
//!   [`provide_arc_value`](crate::builder::PulseContainerBuilder::provide_arc_value). These are concrete values stored directly.
//!

use std::any::TypeId;
use std::fmt;

use any_container::AnyCloneBox;
use metronomos_loom::dependency::{
    Dependency, DependencyItem, DependencyKey, DependencyKeyKind, ShallowDependency,
};

use crate::container::PulseContext;
use crate::error::PulseError;
use crate::value::PulseValue;

mod async_fn_dep;
mod fn_dep;
mod info;
mod util;
mod value_dep;

pub use async_fn_dep::*;
pub use fn_dep::*;
pub use info::PulseDependencyInfo;
pub(crate) use value_dep::{ArcValueDependency, ValueDependency};

/// The kind of a dependency: function-registered or direct value.
enum PulseDependencyKind {
    /// A synchronous function provider.
    Fun(Box<dyn ErasedFnDep>),
    /// An asynchronous function provider.
    AsyncFun(Box<dyn ErasedAsyncFnDep>),
    /// A concrete value stored directly.
    Value(AnyCloneBox),
}

/// A complete dependency: its type info, kind, and transitive dependencies.
///
/// This is the internal representation of a single node in the dependency graph.
/// It combines [`PulseDependencyInfo`] (the type-level identity) with a concrete
/// provider (`PulseDependencyKind`) and the list of other dependencies it requires.
pub(crate) struct PulseDependency {
    pub(crate) info: PulseDependencyInfo,
    kind: PulseDependencyKind,
    deps: Vec<DependencyItem<PulseDependencyInfo>>,
}

impl PulseDependency {
    pub(crate) fn new_fun<F, Dep>(fun: F) -> Self
    where
        F: FnDependency<Dep>,
    {
        Self {
            info: PulseDependencyInfo::new::<F::Value>(),
            kind: PulseDependencyKind::Fun(erase_fn_dep(fun)),
            deps: F::dependencies().collect(),
        }
    }

    pub(crate) fn new_async_fun<F, Dep>(fun: F) -> Self
    where
        F: AsyncFnDependency<Dep>,
        Dep: Send + Sync + 'static,
    {
        Self {
            info: PulseDependencyInfo::new::<F::Value>(),
            kind: PulseDependencyKind::AsyncFun(erase_async_fn_dep(fun)),
            deps: F::dependencies().collect(),
        }
    }

    pub(crate) fn new_value<V: PulseValue>(value: V) -> Self {
        Self {
            info: PulseDependencyInfo::new::<V>(),
            kind: PulseDependencyKind::Value(AnyCloneBox::new(V::map_to_storage_type(value))),
            deps: Vec::new(),
        }
    }

    pub(crate) async fn build(
        &self,
        context: PulseContext<'_>,
    ) -> Result<(DependencyKeyKind, AnyCloneBox), PulseError> {
        let boxed = match &self.kind {
            PulseDependencyKind::Fun(f) => f.provide(context)?,
            PulseDependencyKind::AsyncFun(f) => f.provide(context).await?,
            PulseDependencyKind::Value(value) => value.clone(),
        };

        Ok((self.info.kind, boxed))
    }
}

impl fmt::Debug for PulseDependencyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PulseDependencyKind::Fun(_) => write!(f, "Fun(...)"),
            PulseDependencyKind::AsyncFun(_) => write!(f, "AsyncFun(...)"),
            PulseDependencyKind::Value(_) => write!(f, "Value(...)"),
        }
    }
}

impl ShallowDependency for PulseDependency {
    type Key = TypeId;

    fn key(&self) -> DependencyKey<Self> {
        self.info.key().cast()
    }

    fn name(&self) -> &str {
        self.info.name()
    }
}

impl Dependency for PulseDependency {
    type Shallow = PulseDependencyInfo;

    fn shallow(&self) -> Self::Shallow {
        self.info
    }

    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<Self::Shallow>> {
        self.deps.iter().copied()
    }
}
