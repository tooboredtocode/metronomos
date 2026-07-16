//! Error types for the Pulse dependency injection container.
//!
//! This module defines the error hierarchy used throughout the crate's build and runtime lifecycle.
//! All public API methods that can fail return one of these types, either directly or wrapped in a
//! [`PulseError`] enum.
//!
//! # Error Hierarchy
//!
//! ```text
//! PulseError (top-level enum)
//! ├── DependencyNotProvided   — A dependency input was not found in the container
//! ├── MissingDependencies     — One or more declared dependencies are missing at build time
//! │       └── MissingDependencyError (lists which dependencies are absent)
//! ├── ProviderError           — Errors from adding providers to the builder
//! │       └── ProvideError    — DependencyCycle, AlreadyProvided, UnexpectedError
//! └── BuildDependencyError   — Errors originating inside a user-provided dependency function
//!         └── Any `Send + Sync + 'static` error (downcastable via `BuildDependencyError`)
//! ```
//!

use std::collections::HashSet;
use std::fmt;

use metronomos_loom::dependency::Dependency;
use metronomos_loom::error::MissingDependencyError as LoomMissingDependencyError;
use thiserror::Error;

use crate::builder::ProvideError;
use crate::dependency::PulseDependencyInfo;

/// Top-level error type for the Pulse container.
///
/// This enum captures the four categories of failures that can occur during the
/// container's build and runtime lifecycle.
#[derive(Debug, Error)]
pub enum PulseError {
    /// A dependency input was not found in the container.
    #[error("Dependency was not provided: {}", .0.type_name)]
    DependencyNotProvided(PulseDependencyInfo),

    /// One or more declared dependencies are missing at build time.
    #[error(transparent)]
    MissingDependencies(#[from] MissingDependencyError),

    /// A builder-level error occurred while registering providers.
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProvideError),

    /// An error originated inside a user-provided dependency function.
    #[error("Build dependency error: {0}")]
    BuildDependencyError(#[from] BuildDependencyError),
}

/// Error listing which dependencies are missing from the container.
///
/// This error is raised when one or more types that a dependency function requires
/// have no corresponding provider registered in the container. It holds a set of
/// [`PulseDependencyInfo`] values — one for each missing dependency.
#[derive(Debug, Error)]
#[error("Missing dependencies: {}", .dependencies.iter().map(|d| d.type_name).collect::<Vec<_>>().join(", "))]
pub struct MissingDependencyError {
    dependencies: HashSet<PulseDependencyInfo>,
}

/// A wrapper around a boxed error that implements `Send` and `Sync`.
///
/// When a dependency function returns or propagates an error, Pulse wraps it in this type
/// so the error can be stored inside a shared data structure across threads. The original
/// error type is preserved and accessible through downcasting.
pub struct BuildDependencyError(Box<dyn std::error::Error + Send + Sync>);

impl MissingDependencyError {
    /// Returns the set of missing dependency types.
    ///
    /// Each entry contains the type ID, kind (e.g., sync function or async function), and
    /// human-readable type name for the missing dependency.
    pub fn missing_dependencies(&self) -> &HashSet<PulseDependencyInfo> {
        &self.dependencies
    }
}

impl BuildDependencyError {
    fn as_dyn_error<'a>(&self) -> &(dyn std::error::Error + 'a) {
        &*self.0
    }

    /// Returns `true` if the underlying error is of type `T`.
    pub fn is<T: std::error::Error + 'static>(&self) -> bool {
        self.0.is::<T>()
    }

    /// Attempts to downcast the underlying error to a reference of type `T`.
    pub fn downcast_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }

    /// Attempts to downcast the underlying error to type `T`, consuming the [`BuildDependencyError`] if successful.
    pub fn downcast<T: std::error::Error + 'static>(self) -> Result<T, Self> {
        self.0
            .downcast::<T>()
            .map(|boxed| *boxed)
            .map_err(BuildDependencyError)
    }

    /// Returns the underlying cause of the error, if any.
    pub fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Debug for BuildDependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for BuildDependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<D> From<LoomMissingDependencyError<D>> for MissingDependencyError
where
    D: Dependency<Shallow = PulseDependencyInfo>,
{
    fn from(err: LoomMissingDependencyError<D>) -> Self {
        Self {
            dependencies: err.missing_dependencies().copied().collect(),
        }
    }
}

impl<D> From<LoomMissingDependencyError<D>> for PulseError
where
    D: Dependency<Shallow = PulseDependencyInfo>,
{
    fn from(err: LoomMissingDependencyError<D>) -> Self {
        PulseError::MissingDependencies(err.into())
    }
}

impl<E> From<E> for BuildDependencyError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        BuildDependencyError(Box::new(err))
    }
}
