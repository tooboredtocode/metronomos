//! A type-safe dependency injection (DI) container for Rust inspired by [Uber's `dig`][dig] Go library.
//!
//! `metronomos-pulse` provides a DI container that dynamically resolves dependencies during setup
//! and using compile-time optimiziable function-based constructors.
//! It is designed to be ergonomic, type-safe, and flexible,
//!
//! # Quick overview
//!
//! Dependencies are registered with [`PulseContainerBuilder`](crate::builder::PulseContainerBuilder)
//! using function-based constructors whose arguments are automatically resolved from the graph.
//! Types must derive [`PulseValue`](crate::value::PulseValue):
//!
//! ```
//! use metronomos_pulse::{PulseContainer, value::PulseValue};
//!
//! #[derive(PulseValue, Clone, Debug, PartialEq)]
//! struct Config(String);
//!
//! #[derive(PulseValue, Clone, Debug, PartialEq)]
//! struct Database { config: Config }
//!
//! impl Database {
//!     fn init(config: Config) -> Self {
//!         Database { config }
//!     }
//! }
//!
//! # tokio::runtime::Builder::new_current_thread()
//! #     .enable_all()
//! #     .build()
//! #     .unwrap()
//! #     .block_on(async {
//! let mut builder = PulseContainer::builder();
//! builder.provide_value(Config("postgres://localhost".into())).unwrap();
//! builder.provide(Database::init).unwrap();
//!
//! let container = builder.build().await.unwrap();
//!
//! let db = container.context().get_value::<Database>();
//! assert_eq!(db, Some(&Database { config: Config("postgres://localhost".into()) }));
//! # });
//! ```
//!
//! [dig]: https://pkg.go.dev/go.uber.org/dig

pub mod builder;
pub mod container;
pub mod dependency;
pub mod error;
mod ext;
#[cfg(test)]
mod test;
pub mod value;

pub use container::PulseContainer;
