use std::collections::{HashMap, HashSet};

use daggy::Dag;

use crate::builder::DependencyGraphBuilder;
use crate::dependency::Dependency;
use crate::display::DependencyGraphDisplay;
use crate::entry::{DependencyEntry, DependencyGroupEntry};
use crate::graph_ref::{
    DepOrDepGroupItemRef, DependencyAnyRef, DependencyGroupItemRef, DependencyGroupRef,
    DependencyRef,
};
use crate::iters::InitChunksIter;
use crate::node_descriptors::{GraphNode, RootNode, RootNodeKind};

/// A directed acyclic graph (DAG) representing dependencies and their relationships.
pub struct DependencyGraph<D: Dependency> {
    pub(crate) dependencies: HashMap<D::Key, DependencyEntry<D>>,
    pub(crate) dependency_groups: HashMap<D::Key, DependencyGroupEntry<D>>,
    pub(crate) graph: Dag<GraphNode<D::Key>, ()>,
    pub(crate) root_nodes: HashSet<RootNode<D::Key>>,
}

impl<D: Dependency> DependencyGraph<D> {
    /// Creates a new dependency graph builder.
    pub fn builder() -> DependencyGraphBuilder<D> {
        DependencyGraphBuilder {
            dependencies: HashMap::new(),
            dependency_groups: HashMap::new(),
            graph: Dag::new(),
        }
    }

    /// Creates a new empty dependency graph.
    #[doc(hidden)]
    pub fn new_empty() -> Self {
        DependencyGraph {
            dependencies: HashMap::new(),
            dependency_groups: HashMap::new(),
            graph: Dag::new(),
            root_nodes: HashSet::new(),
        }
    }

    /// Returns the number of dependencies in the graph.
    pub fn len(&self) -> usize {
        self.dependencies.len() + self.dependency_groups.len()
    }

    /// Returns true if the graph has no dependencies.
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty() && self.dependency_groups.is_empty()
    }

    /// Returns a reference to the dependency with the given key, if it exists.
    pub fn get_dependency(&self, key: &D::Key) -> Option<DependencyRef<'_, D>> {
        self.dependencies
            .get_key_value(key)
            .map(|(key, entry)| DependencyRef {
                dep: self,
                key,
                entry,
            })
    }

    /// Returns a reference to the dependency group with the given key, if it exists.
    pub fn get_dependency_group(&self, key: &D::Key) -> Option<DependencyGroupRef<'_, D>> {
        self.dependency_groups
            .get_key_value(key)
            .map(|(key, group_entry)| DependencyGroupRef {
                dep: self,
                key,
                group_entry,
            })
    }

    /// Returns a reference to the dependency group item with the given key and index, if it exists.
    pub fn get_dependency_group_item(
        &self,
        key: &D::Key,
        idx: u16,
    ) -> Option<DependencyGroupItemRef<'_, D>> {
        self.dependency_groups
            .get_key_value(key)
            .and_then(|(key, group_entry)| {
                let dependency = group_entry.dependencies.get(idx as usize)?;
                let node_idx = *group_entry.graph_nodes.get(idx as usize)?;

                Some(DependencyGroupItemRef {
                    dep: self,
                    key,
                    group_entry,
                    dependency,
                    idx,
                    node_idx,
                })
            })
    }

    /// Returns an iterator over the root nodes of the dependency graph.
    pub fn root_nodes(&self) -> impl Iterator<Item = DepOrDepGroupItemRef<'_, D>> {
        self.root_nodes
            .iter()
            .filter_map(|root_node| match root_node.kind {
                RootNodeKind::Dependency => self
                    .get_dependency(&root_node.key)
                    .map(DepOrDepGroupItemRef::Dependency),
                RootNodeKind::DependencyGroupItem(idx) => self
                    .get_dependency_group_item(&root_node.key, idx)
                    .map(DepOrDepGroupItemRef::DependencyGroupItem),
            })
    }

    /// Returns an iterator over all dependencies in the graph.
    pub fn dependencies(&self) -> impl Iterator<Item = DependencyRef<'_, D>> {
        self.dependencies
            .keys()
            .filter_map(|key| self.get_dependency(key))
    }

    /// Returns an iterator over all dependency groups in the graph.
    pub fn dependency_groups(&self) -> impl Iterator<Item = DependencyGroupRef<'_, D>> {
        self.dependency_groups
            .keys()
            .filter_map(|key| self.get_dependency_group(key))
    }

    /// Returns an iterator that chunks over the dependencies in the graph, which can be
    /// initialized in parallel. The order is guaranteed to be topological, meaning that if
    /// dependency A depends on a dependency B, then B will be yielded before A.
    /// Useful for initializing the dependencies to ensure that all dependencies are initialized
    /// before their dependents.
    pub fn init_chunks(&self) -> impl Iterator<Item = Vec<DepOrDepGroupItemRef<'_, D>>> {
        InitChunksIter::new(self)
    }

    /// Maps the dependencies in the graph to a new type, returning a new dependency graph with the
    /// mapped dependencies.
    ///
    /// ### Note
    /// If you do not need to keep the original graph, consider using [`Self::map_owned`] instead,
    /// which consumes the original graph and avoids unnecessary cloning of the graph structure.
    pub fn map<F, D2>(&self, mut f: F) -> DependencyGraph<D2>
    where
        F: FnMut(&D) -> D2,
        D2: Dependency<Key = D::Key, Shallow = D::Shallow>,
        D::Shallow: Clone,
    {
        let dependencies = self
            .dependencies
            .iter()
            .map(|(key, entry)| {
                let new_entry = DependencyEntry {
                    dependency: f(&entry.dependency),
                    graph_node: entry.graph_node,
                };
                (key.clone(), new_entry)
            })
            .collect();

        let dependency_groups = self
            .dependency_groups
            .iter()
            .map(|(key, group_entry)| {
                let dependencies = group_entry.dependencies.iter().map(&mut f).collect();

                let new_group_entry = DependencyGroupEntry {
                    dependencies,
                    shallow: group_entry.shallow.clone(),
                    graph_nodes: group_entry.graph_nodes.clone(),
                    group_node: group_entry.group_node,
                };
                (key.clone(), new_group_entry)
            })
            .collect();

        DependencyGraph {
            dependencies,
            dependency_groups,
            graph: self.graph.clone(),
            root_nodes: self.root_nodes.clone(),
        }
    }

    /// Maps the dependencies in the graph to a new type, returning a new dependency graph with the
    /// mapped dependencies. This method consumes the original graph, avoiding unnecessary cloning
    /// of the graph structure.
    ///
    /// ### Note
    /// If you need to keep the original graph, consider using [`Self::map`] instead of
    /// `.clone().map_owned()`, which avoids unnecessary cloning of the dependency map.
    pub fn map_owned<F, D2>(self, mut f: F) -> DependencyGraph<D2>
    where
        F: FnMut(D) -> D2,
        D2: Dependency<Key = D::Key, Shallow = D::Shallow>,
    {
        let dependencies = self
            .dependencies
            .into_iter()
            .map(|(key, entry)| {
                let new_entry = DependencyEntry {
                    dependency: f(entry.dependency),
                    graph_node: entry.graph_node,
                };
                (key, new_entry)
            })
            .collect();

        let dependency_groups = self
            .dependency_groups
            .into_iter()
            .map(|(key, group_entry)| {
                let dependencies = group_entry.dependencies.into_iter().map(&mut f).collect();

                let new_group_entry = DependencyGroupEntry {
                    dependencies,
                    shallow: group_entry.shallow,
                    graph_nodes: group_entry.graph_nodes,
                    group_node: group_entry.group_node,
                };
                (key, new_group_entry)
            })
            .collect();

        DependencyGraph {
            dependencies,
            dependency_groups,
            graph: self.graph,
            root_nodes: self.root_nodes,
        }
    }

    /// Shrinks the capacity of the graph, dependencies, and dependency groups to fit their
    /// current length.
    pub fn shrink_to_fit(&mut self) {
        self.graph.shrink_to_fit();
        self.dependencies.shrink_to_fit();
        self.dependency_groups.shrink_to_fit();
        for group_entry in self.dependency_groups.values_mut() {
            group_entry.dependencies.shrink_to_fit();
            group_entry.graph_nodes.shrink_to_fit();
        }
        self.root_nodes.shrink_to_fit();
    }

    /// Returns a struct that displays the dependency graph in DOT format.
    pub fn display(&self) -> DependencyGraphDisplay<'_, D> {
        DependencyGraphDisplay::new(self)
    }

    /// Returns a struct that displays the dependency graph in DOT format, with the dependencies
    /// filtered by a predicate function.
    pub fn display_filtered<'a, F>(&'a self, filter: F) -> DependencyGraphDisplay<'a, D>
    where
        F: Fn(DependencyAnyRef<'a, D>) -> bool,
    {
        DependencyGraphDisplay::new_filtered(self, filter)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::error::AddDependencyErrorKind;
    use crate::test_utils::*;

    #[test]
    fn test_init_chunks_var_a() {
        let graph = create_dependency_graph_var_a();

        let mut chunks = graph.init_chunks();

        let chunk1 = ref_iter_set(chunks.next().unwrap().into_iter());
        assert_eq!(chunk1, HashSet::from(["Dep A", "Dep B"]));

        let chunk2 = ref_iter_set(chunks.next().unwrap().into_iter());
        assert_eq!(chunk2, HashSet::from(["Dep C", "Dep D"]));

        let chunk3 = ref_iter_set(chunks.next().unwrap().into_iter());
        assert_eq!(chunk3, HashSet::from(["Dep F"]));

        let chunk4 = ref_iter_set(chunks.next().unwrap().into_iter());
        assert_eq!(chunk4, HashSet::from(["Dep G"]));

        assert!(chunks.next().is_none());
    }

    #[test]
    fn test_init_chunks_var_b() {
        let graph = create_dependency_graph_var_b();

        let mut chunks = graph.init_chunks();

        let chunk1 = count_ref_iter(chunks.next().unwrap().into_iter());
        assert_eq!(
            chunk1,
            HashMap::from([("Dep A", 2), ("Dep B", 1), ("Dep D", 1)])
        );

        let chunk2 = count_ref_iter(chunks.next().unwrap().into_iter());
        assert_eq!(chunk2, HashMap::from([("Dep C", 1), ("Dep D", 2)]));

        let chunk3 = count_ref_iter(chunks.next().unwrap().into_iter());
        assert_eq!(chunk3, HashMap::from([("Dep F", 1)]));

        let chunk4 = count_ref_iter(chunks.next().unwrap().into_iter());
        assert_eq!(chunk4, HashMap::from([("Dep G", 1)]));

        assert!(chunks.next().is_none());
    }

    #[test]
    fn test_cyclic_dependency_detection() {
        let mut builder = DependencyGraph::builder();

        builder
            .add_dependency(TestDependency::new(
                "Dep A",
                false,
                [("Dep C", true, false)].into_iter(),
            ))
            .unwrap();
        builder
            .add_dependency(TestDependency::new(
                "Dep B",
                false,
                [("Dep A", true, false)].into_iter(),
            ))
            .unwrap();

        // Introduce a cycle: Dep A depends on Dep C
        let result = builder
            .add_dependency(TestDependency::new(
                "Dep C",
                false,
                [("Dep B", true, false)].into_iter(),
            ))
            .map_err(|e| e.into_parts().1);

        assert_matches!(
            result,
            Err(AddDependencyErrorKind::DependencyCycle),
            "Expected a dependency cycle error"
        );
    }
}
