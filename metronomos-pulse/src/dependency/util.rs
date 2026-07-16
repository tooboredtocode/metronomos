use crate::error::{BuildDependencyError, PulseError};
use crate::value::PulseValue;

/// Trait for converting a function's return type into a [`PulseError`]-based result.
///
/// This trait is used internally to normalize the return types of dependency functions.
/// It allows users to write dependency functions that either:
/// - Return a value directly (e.g., `|name: String| Greeting { message: name }`)
/// - Return a `Result` (e.g., `|name: String| -> Result<Greeting, BuildDependencyError> { ... }`)
pub trait IntoPulseResult: Sized + Send + Sync + 'static {
    /// The value type produced by the conversion.
    type Value: PulseValue;

    /// Convert this value or error into a `Result<Value, PulseError>`.
    fn into_pulse_result(self) -> Result<Self::Value, PulseError>;
}

impl<T> IntoPulseResult for T
where
    T: PulseValue,
{
    type Value = T;

    fn into_pulse_result(self) -> Result<Self::Value, PulseError> {
        Ok(self)
    }
}

impl<T> IntoPulseResult for Result<T, BuildDependencyError>
where
    T: PulseValue,
{
    type Value = T;

    fn into_pulse_result(self) -> Result<Self::Value, PulseError> {
        self.map_err(From::from)
    }
}
