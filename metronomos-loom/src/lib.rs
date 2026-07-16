//! A crate for building and managing dependency graphs.
//!
//! This crate provides a flexible dependency graph system that supports both unique dependencies
//! and dependency groups. It is built on top of a directed acyclic graph (DAG) structure,
//! ensuring that dependencies are properly ordered and cycles are detected.
//!
//! # Key Concepts
//!
//! ## Dependencies
//!
//! Dependencies are types that implement the [`Dependency`](dependency::Dependency) trait.
//! Each dependency has:
//! - A unique key that identifies it in the graph
//! - A set of dependencies it depends on
//!
//! ## Dependency Groups
//!
//! Dependency groups allow multiple instances of the same dependency type to be added to the graph.
//! They are useful when you need to manage several similar dependencies that share a common identity.
//!
//! # Examples
//!
//! ```
//! use metronomos_loom::DependencyGraph;
//! use metronomos_loom::dependency::{Dependency, DependencyItem, DependencyKey, ShallowDependency};
//! use metronomos_loom::error::AddDependencyErrorKind;
//!
//! // Define a custom dependency type
//! #[derive(Debug)]
//! struct MyDependency {
//!     name: String,
//!     deps: Vec<MyShallowDependency>,
//! }
//!
//! #[derive(Debug, Clone)]
//! struct MyShallowDependency {
//!    name: String,
//! }
//!
//! impl ShallowDependency for MyDependency {
//!     type Key = String;
//!
//!     fn key(&self) -> DependencyKey<Self> {
//!         DependencyKey::new_unique(self.name.clone())
//!     }
//!
//!     fn name(&self) -> &str {
//!         &self.name
//!     }
//! }
//!
//! impl ShallowDependency for MyShallowDependency {
//!    type Key = String;
//!
//!    fn key(&self) -> DependencyKey<Self> {
//!        DependencyKey::new_unique(self.name.clone())
//!    }
//!
//!    fn name(&self) -> &str {
//!        &self.name
//!    }
//! }
//!
//! impl Dependency for MyDependency {
//!     type Shallow = MyShallowDependency;
//!
//!     fn shallow(&self) -> Self::Shallow {
//!         MyShallowDependency {
//!            name: self.name.clone(),
//!         }
//!     }
//!
//!     fn dependencies(&self) -> impl Iterator<Item = DependencyItem<Self::Shallow>> {
//!         self.deps.iter().map(|d| DependencyItem::required(d.clone()))
//!     }
//! }
//!
//! // Build a dependency graph
//! let mut builder = DependencyGraph::builder();
//! let dep_a = MyDependency {
//!     name: "A".to_string(),
//!     deps: vec![],
//! };
//! builder.add_dependency(dep_a).unwrap();
//!
//! let graph = builder.build().unwrap();
//! assert_eq!(graph.len(), 1);
//! ```
//!

pub mod builder;
pub mod dependency;
pub mod display;
mod entry;
pub mod error;
mod graph;
pub mod graph_ref;
mod iters;
mod node_descriptors;
#[cfg(test)]
mod test_utils;

pub use graph::DependencyGraph;
