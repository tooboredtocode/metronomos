//! Value traits for the DI container.
//!
//! This module defines three core traits that power how types interact with [`PulseContainer`](crate::PulseContainer):
//!
//! - [`PulseValue`] — Marker trait for types that can be stored and retrieved from the container.
//!   Derive it via `#[derive(PulseValue)]` from `metronomos-pulse-macros`.
//! - [`CustomPulseValue`] — For types that need manual registration (e.g., foreign or non-derived types).
//! - [`FromPulseValue`] — Used by the dependency system to extract values as function arguments.

use metronomos_loom::dependency::DependencyKeyKind;
pub use metronomos_pulse_macros::PulseValue;

use crate::container::PulseContext;

mod inner {
    pub trait PulseValueSealed: Sized + Clone + Send + Sync + 'static {}
}
pub(crate) use inner::PulseValueSealed;

/// Marker trait for types that can be used as values in a [`PulseContainer`](crate::PulseContainer).
///
/// The recommended way to implement this trait is via the derive macro:
///
/// ```
/// use metronomos_pulse::value::PulseValue;
///
/// #[derive(PulseValue, Clone)]
/// struct Config {
///     timeout_ms: u32,
/// }
/// ```
pub trait PulseValue: PulseValueSealed {
    #[doc(hidden)]
    const KIND: DependencyKeyKind = DependencyKeyKind::Unique;
    #[doc(hidden)]
    const IS_FINALIZER: bool = false;
    #[doc(hidden)]
    type StorageType: Sized + Clone + Send + Sync + 'static;

    /// Type returned by [`PulseContext::get_value`] for this value type.
    type GetValueType<'a>;

    #[doc(hidden)]
    fn name() -> &'static str;

    #[doc(hidden)]
    fn map_to_storage_type(value: Self) -> Self::StorageType;
    #[doc(hidden)]
    fn get_value_type(context: PulseContext<'_>) -> Self::GetValueType<'_>;
}

/// Marker trait for types to implement [`PulseValue`] manually, without using the derive macro.
pub trait CustomPulseValue: Sized + Clone + Send + Sync + 'static {
    /// Stable name for the type, used as its key in the dependency graph.
    const NAME: &'static str;
}
impl<T: CustomPulseValue> PulseValueSealed for T {}
impl<T: CustomPulseValue> PulseValue for T {
    const KIND: DependencyKeyKind = DependencyKeyKind::Unique;
    type StorageType = Self;

    type GetValueType<'a> = Option<&'a Self>;

    fn name() -> &'static str {
        T::NAME
    }

    fn map_to_storage_type(value: Self) -> Self::StorageType {
        value
    }

    fn get_value_type(context: PulseContext<'_>) -> Self::GetValueType<'_> {
        context.inner_get_value::<Self>()
    }
}

/// Trait for types that can be constructed from a value in a [`PulseContainer`](crate::PulseContainer).
///
/// Types implementing this trait can be used as arguments in function-based dependencies:
/// - [`FnDependency`](crate::dependency::FnDependency) — synchronous dependency functions
/// - [`AsyncFnDependency`](crate::dependency::AsyncFnDependency) — asynchronous dependency functions
///
/// The container uses this trait to inject values into dependency constructors automatically.
/// When you write a dependency like `|config: Config| Service { config }`, the `Config` type
/// is expected to implement `FromPulseValue`.
pub trait FromPulseValue: Sized {
    /// The [`PulseValue`] type this can be constructed from.
    type Value: PulseValue;

    /// Whether this dependency is required.
    ///
    /// `true` (the default) means the dependency must be present in the container,
    /// and an error will be returned if it cannot be resolved. Set to `false` for
    /// optional dependencies (e.g., when implementing for `Option<T>`).
    const REQUIRED: bool = true;

    /// Constructs an instance of the type from a value in a [`PulseContainer`](crate::PulseContainer).
    ///
    /// The value may be `None` when the dependency is optional. In that case, this function
    /// should return `None` to signal that the dependency could not be resolved.
    fn from_value(value: <Self::Value as PulseValue>::GetValueType<'_>) -> Option<Self>;
}

impl<T> FromPulseValue for T
where
    T: for<'a> PulseValue<GetValueType<'a> = Option<&'a T>>,
{
    type Value = T;

    fn from_value(value: Option<&T>) -> Option<Self> {
        value.cloned()
    }
}

impl<T> FromPulseValue for Option<T>
where
    T: for<'a> PulseValue<GetValueType<'a> = Option<&'a T>>,
{
    type Value = T;
    const REQUIRED: bool = false;

    fn from_value(value: Option<&T>) -> Option<Self> {
        Some(value.cloned())
    }
}
