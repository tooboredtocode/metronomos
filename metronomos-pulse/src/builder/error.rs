//! Error types returned during dependency registration and building.

use std::fmt;

use metronomos_loom::error::{AddDependencyError, AddDependencyErrorKind};
use thiserror::Error;

use crate::dependency::{ArcValueDependency, PulseDependency, ValueDependency};
use crate::value::PulseValue;

/// Error returned when a dependency provider cannot be registered with the builder.
#[derive(Debug, Error)]
pub enum ProvideError {
    /// A provider for this type would create a dependency cycle.
    #[error("Provider would create a dependency cycle")]
    DependencyCycle,

    /// A provider for this type has already been provided.
    #[error("A provider for this type has already been provided")]
    AlreadyProvided,

    /// An unexpected error occurred while adding the provider.
    #[error("Unexpected error occurred while adding the provider")]
    UnexpectedError,
}

impl ProvideError {
    fn from_add_dependency_error_kind(kind: AddDependencyErrorKind) -> Self {
        match kind {
            AddDependencyErrorKind::DependencyCycle => ProvideError::DependencyCycle,
            AddDependencyErrorKind::AlreadyProvided => ProvideError::AlreadyProvided,
            _ => ProvideError::UnexpectedError,
        }
    }
}

/// Error returned when a value fails to be provided to the builder.
///
/// This error is returned by [`PulseContainerBuilder::provide_value`](crate::builder::PulseContainerBuilder::provide_value)
/// and [`PulseContainerBuilder::provide_arc_value`](crate::builder::PulseContainerBuilder::provide_arc_value) when the
/// provider conflicts with an existing registration. It carries both the original value
/// (via [`into_value`](Self::into_value)) and the underlying [`ProvideError`](ProvideError).
pub struct ProvideValueError<T> {
    value: T,
    error: ProvideError,
}

impl<T> ProvideValueError<T> {
    pub fn error(&self) -> &ProvideError {
        &self.error
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn into_inner(self) -> (T, ProvideError) {
        (self.value, self.error)
    }
}

impl<T> fmt::Debug for ProvideValueError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProvideValueError")
            .field("value", &"<value>")
            .field("error", &self.error)
            .finish()
    }
}

impl<T> fmt::Display for ProvideValueError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

impl<T> std::error::Error for ProvideValueError<T> {}

impl<V> From<ProvideValueError<V>> for ProvideError {
    fn from(err: ProvideValueError<V>) -> Self {
        let (_, err) = err.into_inner();
        err
    }
}

impl From<AddDependencyError<PulseDependency>> for ProvideError {
    fn from(err: AddDependencyError<PulseDependency>) -> Self {
        let (_, kind) = err.into_parts();
        ProvideError::from_add_dependency_error_kind(kind)
    }
}

impl<V> From<AddDependencyError<ValueDependency<V>>> for ProvideValueError<V>
where
    V: PulseValue,
{
    fn from(err: AddDependencyError<ValueDependency<V>>) -> Self {
        let (val, kind) = err.into_parts();
        ProvideValueError {
            value: val.0,
            error: ProvideError::from_add_dependency_error_kind(kind),
        }
    }
}

impl<T> From<AddDependencyError<ArcValueDependency<T>>> for ProvideValueError<T>
where
    T: Send + Sync + 'static,
{
    fn from(err: AddDependencyError<ArcValueDependency<T>>) -> Self {
        let (val, kind) = err.into_parts();
        ProvideValueError {
            value: val.0,
            error: ProvideError::from_add_dependency_error_kind(kind),
        }
    }
}
