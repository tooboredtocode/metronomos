//! Error types for the dependency graph builder.
//!
//! This module provides error types used when building the dependency graph:
//!
//! - [`AddDependencyErrorKind`] - The kind of error that occurred during dependency addition
//! - [`AddDependencyError`] - Error returned when adding a dependency fails
//! - [`MissingDependencyError`] - Error returned when building with missing dependencies

use std::fmt;

use crate::builder::DependencyGraphBuilder;
use crate::dependency::Dependency;

/// The kind of error that occurred when adding a dependency.
#[derive(Debug)]
#[non_exhaustive]
pub enum AddDependencyErrorKind {
    /// Adding the dependency created a cycle in the graph.
    DependencyCycle,
    /// The dependency was already provided.
    AlreadyProvided,
}

/// Error returned when adding a dependency to the builder fails.
///
/// This error occurs when:
/// - Adding the dependency creates a cycle in the graph ([`AddDependencyErrorKind::DependencyCycle`])
/// - The dependency was already provided ([`AddDependencyErrorKind::AlreadyProvided`])
///
/// The error includes the original value that was being added, allowing inspection or recovery.
pub struct AddDependencyError<I> {
    into_dependency: I,
    kind: AddDependencyErrorKind,
}

/// Error returned when building a dependency graph with missing required dependencies.
///
/// This error contains the original `DependencyGraphBuilder`, allowing inspection of the
/// missing dependencies and potentially recovery by adding the missing dependencies.
pub struct MissingDependencyError<D: Dependency>(pub DependencyGraphBuilder<D>);

impl<I> AddDependencyError<I> {
    pub(crate) fn new_cycle(into_dependency: I) -> Self {
        Self {
            into_dependency,
            kind: AddDependencyErrorKind::DependencyCycle,
        }
    }

    pub(crate) fn new_already_provided(into_dependency: I) -> Self {
        Self {
            into_dependency,
            kind: AddDependencyErrorKind::AlreadyProvided,
        }
    }
}

impl<I> AddDependencyError<I> {
    /// Returns the kind of error that occurred.
    pub fn kind(&self) -> &AddDependencyErrorKind {
        &self.kind
    }

    /// Consumes the error and returns the original value that was being added.
    pub fn into_inner(self) -> I {
        self.into_dependency
    }

    /// Consumes the error and returns both the original value and the error kind.
    pub fn into_parts(self) -> (I, AddDependencyErrorKind) {
        (self.into_dependency, self.kind)
    }
}

impl<D: Dependency> MissingDependencyError<D> {
    /// Returns an iterator over the missing dependencies.
    pub fn missing_dependencies(&self) -> impl Iterator<Item = &D::Shallow> {
        self.0.missing_dependencies(false)
    }
}

impl<I> fmt::Debug for AddDependencyError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddDependencyError")
            .field("into_dependency", &"<into_dependency>")
            .field("kind", &self.kind)
            .finish()
    }
}

impl<D: Dependency> fmt::Debug for MissingDependencyError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MissingDependencyError")
            .field(&"<graph_builder>")
            .finish()
    }
}

impl<I> fmt::Display for AddDependencyError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            AddDependencyErrorKind::DependencyCycle => {
                write!(
                    f,
                    "Adding dependency would create a cycle in the dependency graph"
                )
            }
            AddDependencyErrorKind::AlreadyProvided => {
                write!(f, "The dependency was already provided")
            }
        }
    }
}

impl<D: Dependency> fmt::Display for MissingDependencyError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot build Dependency Graph due to a missing dependency"
        )
    }
}

impl<I> std::error::Error for AddDependencyError<I> {}

impl<D: Dependency> std::error::Error for MissingDependencyError<D> {}
