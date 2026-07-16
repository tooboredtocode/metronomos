//! Value traits and types for the DI container.
//!
//! This module defines how types become storable values in a [`PulseContainer`][crate::PulseContainer].
//! The central trait is [`PulseValue`], which marks a type as eligible to be stored and retrieved
//! from the container.
//!
//! # Core Traits
//!
//! - [`PulseValue`] — Marker trait for storable types. Derive via `#[derive(PulseValue)]` or implement [`CustomPulseValue`] for custom behaviour.
//! - [`FromPulseValue`] — Used to extract values from the container as function arguments in dependencies.
//!
//! # Value Types
//!
//! - [`ArcValue<T>`] — Wraps `Arc<T>` so the inner type can be used as a DI value (cheap cloning).
//! - [`GroupValues<V>`] — Collects multiple instances of a grouped dependency into an iterable collection.
//! - [`ValueGroupEntry<V>`] — Wrapper for a single entry in a dependency group, derefs to the inner value.
//!

mod arc_value;
mod dot_string;
mod group;
mod traits;
mod unit;
pub(crate) mod utils;

pub use arc_value::ArcValue;
pub use dot_string::DotString;
pub use group::{GroupValues, ValueGroupEntry};
pub use traits::*;
