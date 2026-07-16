//! The dependency graph builder.
//!
//! This module provides [`DependencyGraphBuilder`], which is used to construct
//! [`DependencyGraph`] instances. The builder allows adding dependencies with their
//! relationships while maintaining the DAG invariant.

use std::collections::HashSet;
use std::collections::hash_map::{Entry, HashMap};
use std::ops::Deref;

use daggy::petgraph::Direction;
use daggy::{Dag, NodeIndex, Walker, WouldCycle};
use itertools::{Either, Itertools};

use crate::DependencyGraph;
use crate::dependency::{
    Dependency, DependencyItem, DependencyKeyKind, IntoDependency, ShallowDependency,
};
use crate::entry::{DependencyGroupEntry, MaybeDependency, MaybeDependencyEntry};
use crate::error::{AddDependencyError, MissingDependencyError};
use crate::node_descriptors::{GraphNode, GraphNodeKind, RootNode};

/// A builder for constructing dependency graphs.
///
/// [`DependencyGraphBuilder`] allows you to incrementally add dependencies and dependency groups
/// to a graph, maintaining a directed acyclic graph (DAG) structure. It tracks dependencies,
/// handles optional dependencies, and detects cycles before building the final graph.
pub struct DependencyGraphBuilder<D: Dependency> {
    pub(crate) dependencies: HashMap<D::Key, MaybeDependencyEntry<D>>,
    pub(crate) dependency_groups: HashMap<D::Key, DependencyGroupEntry<D>>,
    pub(crate) graph: Dag<GraphNode<D::Key>, ()>,
}

impl<D: Dependency> DependencyGraphBuilder<D> {
    /// Adds a new dependency to the graph.
    pub fn add_dependency<I>(&mut self, into_dependency: I) -> Result<(), AddDependencyError<I>>
    where
        I: IntoDependency<D>,
    {
        let key = into_dependency.key();
        match key.kind {
            DependencyKeyKind::Unique => self.add_unique_dependency(key.key, into_dependency),
            DependencyKeyKind::Group => self.add_group_dependency(key.key, into_dependency),
        }
    }

    fn add_unique_dependency<I>(
        &mut self,
        key: D::Key,
        into_dependency: I,
    ) -> Result<(), AddDependencyError<I>>
    where
        I: IntoDependency<D>,
    {
        #[cfg(debug_assertions)]
        if self.dependency_groups.contains_key(&key) {
            panic!(
                "Dependency key {:?} should not exist in the dependency groups map, but it does",
                key
            );
        }

        let (node_index, is_new) = match self.dependencies.get(&key) {
            Some(entry) => {
                if entry.is_present() {
                    return Err(AddDependencyError::new_already_provided(into_dependency));
                }

                (entry.graph_node, false)
            }
            None => {
                let node_index = self.graph.add_node(GraphNode::new_dependency(key.clone()));
                (node_index, true)
            }
        };

        if self
            .try_insert_node_with_deps(node_index, into_dependency.dependencies())
            .is_err()
        {
            if is_new {
                self.graph.remove_node(node_index);
            }
            return Err(AddDependencyError::new_cycle(into_dependency));
        }

        // Update the dependency entry for the current dependency.
        match self.dependencies.entry(key) {
            Entry::Occupied(mut entry) => {
                let old_entry = entry
                    .get_mut()
                    .set_dependency(into_dependency.into_dependency());
                // Check for logic errors in debug mode, but skip the check in release mode for performance reasons.
                debug_assert!(
                    old_entry.is_none(),
                    "Dependency entry should not have had a dependency before"
                );
            }
            Entry::Vacant(entry) => {
                entry.insert(MaybeDependencyEntry::new_present(
                    node_index,
                    into_dependency.into_dependency(),
                ));
            }
        }

        Ok(())
    }

    /// Adds a dependency group to the graph.
    fn add_group_dependency<I>(
        &mut self,
        key: D::Key,
        into_dependency: I,
    ) -> Result<(), AddDependencyError<I>>
    where
        I: IntoDependency<D>,
    {
        #[cfg(debug_assertions)]
        if self.dependencies.contains_key(&key) {
            panic!(
                "Dependency group key {:?} should not exist in the dependencies map, but it does",
                key
            );
        }

        let (group_index, existing_items, is_new) = match self.dependency_groups.get(&key) {
            Some(entry) => (entry.group_node, entry.dependencies.len() as u16, false),
            None => {
                let group_index = self
                    .graph
                    .add_node(GraphNode::new_dependency_group(key.clone()));
                (group_index, 0, true)
            }
        };

        let (_, node_index) = self.graph.add_parent(
            group_index,
            (),
            GraphNode::new_dependency_group_item(key.clone(), existing_items),
        );

        if self
            .try_insert_node_with_deps(node_index, into_dependency.dependencies())
            .is_err()
        {
            self.graph.remove_node(node_index);
            if is_new {
                self.graph.remove_node(group_index);
            }
            return Err(AddDependencyError::new_cycle(into_dependency));
        }

        // Finally, add the dependency to the group entry.
        let entry = self
            .dependency_groups
            .entry(key)
            .or_insert_with(|| DependencyGroupEntry::new(group_index, into_dependency.shallow()));

        entry.dependencies.push(into_dependency.into_dependency());
        entry.graph_nodes.push(node_index);

        Ok(())
    }

    /// Inserts a node with its dependencies into the graph.
    fn try_insert_node_with_deps(
        &mut self,
        node_index: NodeIndex,
        dependencies: impl Iterator<Item = DependencyItem<D::Shallow>>,
    ) -> Result<(), WouldCycle<Vec<()>>> {
        let mut optional_now_missing_deps = HashSet::new();

        let (existing_nodes, new_dependencies): (Vec<_>, Vec<_>) =
            dependencies.partition_map(|dep| {
                let key = dep.key();
                match key.kind {
                    DependencyKeyKind::Group => match self.dependency_groups.get(&key.key) {
                        Some(entry) => Either::Left(entry.group_node),
                        None => Either::Right(dep),
                    },
                    DependencyKeyKind::Unique => {
                        match self.dependencies.get(&key.key) {
                            Some(entry) => {
                                // Upgrade non-grouped optional dependencies to required if they are now required by the new dependency.
                                if entry.is_optional() && dep.is_required() {
                                    optional_now_missing_deps.insert(key.key);
                                }
                                Either::Left(entry.graph_node)
                            }
                            None => Either::Right(dep),
                        }
                    }
                }
            });

        // Add the edges to the existing dependencies.
        self.graph.add_edges(
            existing_nodes
                .into_iter()
                .map(|dep_node| (dep_node, node_index, ())),
        )?;

        // Mark any optional dependencies that are now required as missing.
        for dep_key in optional_now_missing_deps {
            if let Some(entry) = self.dependencies.get_mut(&dep_key) {
                entry.make_missing_required();
            } else if cfg!(debug_assertions) {
                panic!(
                    "Dependency entry for key {:?} should exist, but it does not",
                    dep_key
                );
            }
        }

        // Add the new dependencies to the graph.
        for dep in new_dependencies {
            let key = dep.key();
            match key.kind {
                DependencyKeyKind::Group => {
                    let (_, node) = self.graph.add_parent(
                        node_index,
                        (),
                        GraphNode::new_dependency_group(key.key.clone()),
                    );
                    let old_entry = self
                        .dependency_groups
                        .insert(key.key, DependencyGroupEntry::new(node, dep.into_inner()));
                    // Check for logic errors in debug mode, but skip the check in release mode for performance reasons.
                    debug_assert!(
                        old_entry.is_none(),
                        "Dependency group entry should not have existed before, but it did"
                    );
                }
                DependencyKeyKind::Unique => {
                    let (_, node) = self.graph.add_parent(
                        node_index,
                        (),
                        GraphNode::new_dependency(key.key.clone()),
                    );
                    let old_entry = self
                        .dependencies
                        .insert(key.key, MaybeDependencyEntry::new(node, dep));
                    // Check for logic errors in debug mode, but skip the check in release mode for performance reasons.
                    debug_assert!(
                        old_entry.is_none(),
                        "Dependency entry should not have existed before, but it did"
                    );
                }
            }
        }

        Ok(())
    }

    /// Returns an iterator over all dependencies in the builder.
    pub fn dependencies(&self) -> impl Iterator<Item = &D> {
        self.dependencies
            .values()
            .filter_map(|entry| match &entry.dependency {
                MaybeDependency::Present(dep) => Some(dep),
                _ => None,
            })
    }

    /// Returns an iterator over all dependency groups in the builder.
    pub fn dependency_groups(&self) -> impl Iterator<Item = &[D]> {
        self.dependency_groups
            .values()
            .map(|entry| &*entry.dependencies)
    }

    /// Returns an iterator over the missing dependencies in the builder.
    ///
    /// If `include_optional` is true, optional dependencies are also included.
    pub fn missing_dependencies(
        &self,
        include_optional: bool,
    ) -> impl Iterator<Item = &D::Shallow> {
        self.dependencies
            .values()
            .filter_map(move |entry| match &entry.dependency {
                MaybeDependency::Missing(dep) if include_optional || dep.is_required() => {
                    Some(dep.deref())
                }
                _ => None,
            })
    }

    /// Builds the dependency graph from the current state of the builder.
    #[allow(
        // The error type is large since we return the original builder back to the caller, to
        // allow them to inspect the missing dependencies and fix the graph.
        clippy::result_large_err)
    ]
    pub fn build(self) -> Result<DependencyGraph<D>, MissingDependencyError<D>> {
        if self.missing_dependencies(false).next().is_some() {
            return Err(MissingDependencyError(self));
        }

        let Self {
            dependencies: maybe_dependencies,
            mut dependency_groups,
            mut graph,
        } = self;

        let mut has_optional_dependencies = false;
        let mut dependencies = maybe_dependencies
            .into_iter()
            .filter_map(|(key, entry)| match entry.try_into_dependency_entry() {
                Ok(Some(dep_entry)) => Some((key, dep_entry)),
                Ok(None) => {
                    has_optional_dependencies = true;
                    None
                }
                Err(_) => {
                    unreachable!("Graph should not have missing dependencies at this point")
                }
            })
            .collect::<HashMap<_, _>>();

        // Optimize for graphs with no optional dependencies.
        if has_optional_dependencies {
            // Filter the graph to only include nodes that are present in the dependencies map.
            graph = graph.filter_map(
                |_, key| {
                    if dependencies.contains_key(&key.key) {
                        Some(key.clone())
                    } else {
                        None
                    }
                },
                |_, ()| Some(()),
            );

            // Update the graph_node indices in the dependencies map to match the new graph.
            for (node_idx, node) in graph.raw_nodes().iter().enumerate() {
                match node.weight.kind {
                    GraphNodeKind::Dependency => {
                        if let Some(dep_info) = dependencies.get_mut(&node.weight.key) {
                            dep_info.graph_node = NodeIndex::new(node_idx);
                        } else if cfg!(debug_assertions) {
                            panic!(
                                "All graph nodes should point to a dependency entry, but node {:?} does not",
                                node.weight
                            );
                        }
                    }
                    GraphNodeKind::DependencyGroup => {
                        if let Some(group_info) = dependency_groups.get_mut(&node.weight.key) {
                            group_info.group_node = NodeIndex::new(node_idx);
                        } else if cfg!(debug_assertions) {
                            panic!(
                                "All graph nodes should point to a dependency group entry, but node {:?} does not",
                                node.weight
                            );
                        }
                    }
                    GraphNodeKind::DependencyGroupItem(idx) => {
                        if let Some(group_info) = dependency_groups.get_mut(&node.weight.key)
                            && let Some(exiting_idx) = group_info.graph_nodes.get_mut(idx as usize)
                        {
                            *exiting_idx = NodeIndex::new(node_idx);
                        } else if cfg!(debug_assertions) {
                            panic!(
                                "All graph nodes should point to a dependency group entry, but node {:?} does not",
                                node.weight
                            );
                        }
                    }
                }
            }
        }

        let root_nodes_indices = graph
            .graph()
            .externals(Direction::Incoming)
            .collect::<Vec<_>>();

        let mut root_nodes = HashSet::new();
        let mut dep_group_roots = HashSet::new();

        for &node_index in &root_nodes_indices {
            let key = &graph[node_index];
            match key.kind {
                GraphNodeKind::Dependency => {
                    root_nodes.insert(RootNode::new_dependency(key.key.clone()));
                }
                GraphNodeKind::DependencyGroupItem(idx) => {
                    root_nodes.insert(RootNode::new_dependency_group_item(key.key.clone(), idx));
                }
                GraphNodeKind::DependencyGroup => {
                    dep_group_roots.insert(node_index);
                }
            }
        }

        graph.transitive_reduce(root_nodes_indices);

        for &dep_group_root in &dep_group_roots {
            graph
                .children(dep_group_root)
                .iter(&graph)
                .filter(|&(_, child_index)| {
                    graph
                        .parents(child_index)
                        .iter(&graph)
                        // Only include children that with empty parent groups.
                        .find(|(_, parent_index)| !dep_group_roots.contains(parent_index))
                        .is_none()
                })
                .for_each(|(_, child_index)| {
                    let key = &graph[child_index];
                    match key.kind {
                        GraphNodeKind::Dependency => {
                            root_nodes.insert(RootNode::new_dependency(key.key.clone()));
                        }
                        GraphNodeKind::DependencyGroupItem(idx) => {
                            root_nodes.insert(RootNode::new_dependency_group_item(
                                key.key.clone(),
                                idx,
                            ));
                        }
                        #[cfg(debug_assertions)]
                        GraphNodeKind::DependencyGroup => {
                            panic!("Dependency group should not be a child of another dependency group, but it is: {:?}", key);
                        }
                        #[cfg(not(debug_assertions))]
                        GraphNodeKind::DependencyGroup => {}
                    }
                })
        }

        Ok(DependencyGraph {
            dependencies,
            dependency_groups,
            graph,
            root_nodes,
        })
    }
}
