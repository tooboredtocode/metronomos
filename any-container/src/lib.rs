//! Type-erased containers
//!
//! This crate provides type-erased container types that allow storing values of different types
//! behind a common interface.
//!
//! # Overview
//!
//! The crate provides several type-erased container types:
//!
//! - [`AnyMap`] - A map from types to a single value of each type
//! - [`AnyMultiMap`] - A map from types to multiple values of each type
//! - [`AnyVec`] - A type-erased vector that stores values of a single type
//! - [`AnyCloneBox`] - A type-erased box that allows cloning the boxed value
//!

mod boxed;
mod map;
mod multimap;
mod utils;
pub mod vec;

pub use boxed::AnyCloneBox;
pub use map::AnyMap;
pub use multimap::AnyMultiMap;
pub use vec::AnyVec;
