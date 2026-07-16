//! Internal runtime storage for resolved dependency values.
//!
//! This module provides the two core types that power the DI container at runtime:
//!
//! - [`PulseContainer`] — The backing store that holds all type-erased, resolved dependency
//!   values and their groups, keyed by `TypeId`. It is created by the builder during graph
//!   construction.
//! - [`PulseContext`] — A read-only, copyable handle to a [`PulseContainer`] that provides
//!   typed access to stored values via [`PulseContext::get_value`]. Use this to retrieve
//!   dependencies after the container is built.
//!

use std::any::Any;
use std::fmt;

use any_container::{AnyCloneBox, AnyMap, AnyMultiMap};

use crate::value::{DotString, PulseValue};

/// Internal runtime storage for resolved dependency values.
///
/// This struct holds all type-erased, fully-resolved values keyed by their `TypeId`, along
/// with grouped values (multiple instances of the same type) and a dot-string representation
/// of the dependency graph.
///
/// The container is constructed exclusively through [`PulseContainerBuilder`](crate::builder::PulseContainerBuilder).
/// After construction, interact with it via its [`context`](Self::context) method, which returns
/// a lightweight, copyable [`PulseContext`] handle.
pub struct PulseContainer {
    values: AnyMap,
    value_groups: AnyMultiMap,
    dot_string: DotString,
}

/// A read-only handle to a [`PulseContainer`] for retrieving typed values.
///
/// `PulseContext` is a lightweight, `Copy` struct that borrows a `PulseContainer`.
/// It provides access to all stored dependency values via type-safe retrieval methods.
#[derive(Copy, Clone)]
pub struct PulseContext<'a> {
    inner: &'a PulseContainer,
}

impl PulseContainer {
    /// Creates a new `PulseContainer` with the given dependency graph visualization.
    ///
    /// This constructor is internal — use [`PulseContainerBuilder`](crate::builder::PulseContainerBuilder)
    /// to construct a container in practice.
    pub(crate) fn new(dot_string: DotString) -> Self {
        Self {
            values: AnyMap::new(),
            value_groups: AnyMultiMap::new(),
            dot_string,
        }
    }

    /// Internal method to insert a single typed value into the container.
    pub(crate) fn insert(&mut self, value: AnyCloneBox) -> Result<(), AnyCloneBox> {
        self.values.try_insert_boxed(value)
    }

    /// Internal method to insert a value into a group of the same type.
    pub(crate) fn insert_group(&mut self, value: AnyCloneBox) {
        self.value_groups.insert_boxed(value)
    }

    /// Returns a read-only, copyable [`PulseContext`] handle to this container.
    pub fn context(&self) -> PulseContext<'_> {
        PulseContext::new(self)
    }
}

impl fmt::Debug for PulseContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PulseContainer")
            .field("values", &self.values)
            .field("value_groups", &self.value_groups)
            .field("dot_string", &"<DotString>")
            .finish()
    }
}

impl<'a> PulseContext<'a> {
    pub(crate) fn new(container: &'a PulseContainer) -> PulseContext<'a> {
        PulseContext { inner: container }
    }

    /// Retrieves a registered dependency value by type.
    ///
    /// Returns the stored value if it exists, or `None` if no value of the requested type
    /// was registered during construction. The exact return type depends on the type parameter:
    ///
    /// - For regular types (e.g., custom structs derived with `#[derive(PulseValue)]`): returns `Option<&T>`
    /// - For grouped types (e.g., `GroupValues<T>`): returns `&[T]` (an empty slice if no values were registered)
    pub fn get_value<V: PulseValue>(&self) -> V::GetValueType<'a> {
        V::get_value_type(*self)
    }

    pub(crate) fn inner_get_value<T: Any + Send + Sync>(&self) -> Option<&'a T> {
        self.inner.values.get::<T>()
    }

    pub(crate) fn inner_get_value_group<T: Any + Send + Sync>(&self) -> &'a [T] {
        self.inner.value_groups.get::<T>()
    }

    /// Returns a Graphviz-formatted dot string of the dependency graph.
    ///
    /// The returned [`DotString`] can be visualized with tools that support Graphviz/DOT format
    /// to inspect the dependency structure between container values and groups.
    pub fn dot_string(&self) -> &'a DotString {
        &self.inner.dot_string
    }
}
