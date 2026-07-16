//! Traits and types for defining dependencies in the graph.
//!
//! This module provides the core traits and types used to define what a "dependency" is
//! within the dependency graph system. The main traits are:
//!
//! - [`ShallowDependency`] - A lightweight trait for identifying dependencies by key and name
//! - [`Dependency`] - A full trait for dependencies with their transitive dependencies
//! - [`IntoDependency`] - A trait for converting types into dependencies
//!
//! # Relationship Between Traits
//!
//! [`Dependency`] extends [`ShallowDependency`], providing additional functionality for
//! describing the full dependency relationships of a type.
//!

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

mod key;

pub use key::{DependencyKey, DependencyKeyKind};

/// A lightweight trait for identifying dependencies.
///
/// [`ShallowDependency`] provides the minimal information needed to identify a dependency
/// in the graph: a unique key and a name. It is used when you only need to track
/// dependencies without their full transitive relationships.
pub trait ShallowDependency {
    /// The type of key used to identify this dependency.
    ///
    /// The key must be unique for each dependency and should be a copyable or cheaply clonable type.
    type Key: Hash + Eq + Clone + Debug;

    /// Returns the unique key for this dependency.
    fn key(&self) -> DependencyKey<Self>;

    /// Returns the name of this dependency.
    ///
    /// The name is used for display purposes and does not need to be unique, although using unique
    /// names is recommended.
    fn name(&self) -> &str;
}

/// A trait for types that represent full dependencies with transitive relationships.
///
/// [`Dependency`] extends [`ShallowDependency`] by adding methods for describing the full
/// dependency relationships of a type. This trait is required for types that want to be
/// inserted into the dependency graph.
pub trait Dependency: ShallowDependency {
    /// The shallow version of this dependency.
    ///
    /// This is used to build the dependency graph without needing to know the full details
    /// of the dependency.
    type Shallow: ShallowDependency<Key = Self::Key>;

    /// Returns the shallow version of this dependency.
    fn shallow(&self) -> Self::Shallow;

    /// Returns an iterator of this dependency's dependencies.
    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<Self::Shallow>>;
}

/// A trait for converting a type into a dependency.
///
/// [`IntoDependency`] allows any type to be converted into a dependency, enabling type-safe
/// insertion into the dependency graph. Types implementing this trait can be added directly
/// to a [`DependencyGraphBuilder`](crate::builder::DependencyGraphBuilder) without first being converted.
pub trait IntoDependency<D: Dependency> {
    /// Returns the unique key of the dependency.
    fn key(&self) -> DependencyKey<D>;

    /// Returns the shallow version of the dependency.
    fn shallow(&self) -> D::Shallow;

    /// Returns an iterator of this type's dependencies.
    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<D::Shallow>>;

    /// Converts this type into a dependency.
    fn into_dependency(self) -> D;
}

/// A dependency item representing a dependency in the graph.
///
/// [`DependencyItem`] wraps a dependency with a flag indicating whether it is required or optional.
/// Required dependencies must be satisfied for the graph to be considered valid, while optional
/// dependencies may be missing.
#[derive(Debug, Copy, Clone)]
pub struct DependencyItem<D: ShallowDependency> {
    shallow: D,
    required: bool,
}

impl<D: ShallowDependency> DependencyItem<D> {
    /// Creates a new optional dependency item.
    pub fn optional(shallow: D) -> Self {
        Self {
            shallow,
            required: false,
        }
    }

    /// Creates a new required dependency item.
    pub fn required(shallow: D) -> Self {
        Self {
            shallow,
            required: true,
        }
    }

    /// Returns true if the dependency is required.
    pub fn is_required(&self) -> bool {
        self.required
    }

    pub(crate) fn set_required(&mut self, required: bool) {
        self.required = required;
    }

    /// Returns the shallow version of the dependency.
    pub fn into_inner(self) -> D {
        self.shallow
    }
}

impl<D: ShallowDependency> Deref for DependencyItem<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.shallow
    }
}

impl<D: Dependency> IntoDependency<D> for D {
    fn key(&self) -> DependencyKey<D> {
        self.key()
    }

    fn shallow(&self) -> D::Shallow {
        self.shallow()
    }

    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<<D as Dependency>::Shallow>> {
        self.dependencies()
    }

    fn into_dependency(self) -> D {
        self
    }
}
