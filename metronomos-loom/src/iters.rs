use std::mem;

use daggy::petgraph::visit::{Bfs, Reversed, VisitMap, Visitable};
use daggy::{NodeIndex, Walker};
use fixedbitset::FixedBitSet;

use crate::DependencyGraph;
use crate::dependency::Dependency;
use crate::graph_ref::{DepOrDepGroupItemRef, DepOrDepGroupRef};
use crate::node_descriptors::GraphNodeKind;

fn raw_direct_dependencies_for_node<D: Dependency>(
    dep: &DependencyGraph<D>,
    node_idx: NodeIndex,
) -> impl Iterator<Item = NodeIndex> + '_ {
    dep.graph
        .parents(node_idx)
        .iter(&dep.graph)
        .map(|(_, node_idx)| node_idx)
}

pub(crate) fn direct_dependencies_for_node<D: Dependency>(
    dep: &DependencyGraph<D>,
    node_idx: NodeIndex,
) -> impl Iterator<Item = DepOrDepGroupRef<'_, D>> {
    raw_direct_dependencies_for_node(dep, node_idx).filter_map(move |node_idx| {
        let key = &dep.graph[node_idx];
        match key.kind {
            GraphNodeKind::Dependency => dep.get_dependency(&key.key).map(DepOrDepGroupRef::Dependency),
            GraphNodeKind::DependencyGroup => dep.get_dependency_group(&key.key).map(DepOrDepGroupRef::DependencyGroup),
            #[cfg(debug_assertions)]
            GraphNodeKind::DependencyGroupItem(_) => panic!("Dependencies should not be able to depend on DependencyGroupMember nodes directly!",),
            #[cfg(not(debug_assertions))]
            GraphNodeKind::DependencyGroupItem(_) => None,
        }
    })
}

pub(crate) fn direct_dependants_for_node<D: Dependency>(
    dep: &DependencyGraph<D>,
    node_idx: NodeIndex,
) -> impl Iterator<Item = DepOrDepGroupItemRef<'_, D>> {
    dep
        .graph
        .children(node_idx)
        .iter(&dep.graph)
        .filter_map(|(_, node_idx)| {
            let key = &dep.graph[node_idx];
            match key.kind {
                GraphNodeKind::Dependency => dep.get_dependency(&key.key).map(DepOrDepGroupItemRef::Dependency),
                GraphNodeKind::DependencyGroupItem(idx) => dep.get_dependency_group_item(&key.key, idx).map(DepOrDepGroupItemRef::DependencyGroupItem),
                #[cfg(debug_assertions)]
                GraphNodeKind::DependencyGroup => panic!("Dependency groups should not be able to depend on Dependencies or Dependency Groups directly!",),
                #[cfg(not(debug_assertions))]
                GraphNodeKind::DependencyGroup => None,
            }
        })
}

pub(crate) fn all_dependencies_for_node<D: Dependency>(
    dep: &DependencyGraph<D>,
    node_idx: NodeIndex,
) -> impl Iterator<Item = DepOrDepGroupRef<'_, D>> {
    let graph = Reversed(&dep.graph);
    let mut node_iter = Bfs::new(graph, node_idx).iter(graph);

    let first_node = node_iter.next(); // Skip the first node, which is the node itself.
    debug_assert_eq!(
        first_node,
        Some(node_idx),
        "The first node in the BFS iterator should be the node itself."
    );

    node_iter.filter_map(|node_idx| {
        let key = &dep.graph[node_idx];
        match key.kind {
            GraphNodeKind::Dependency => dep
                .get_dependency(&key.key)
                .map(DepOrDepGroupRef::Dependency),
            GraphNodeKind::DependencyGroup => dep
                .get_dependency_group(&key.key)
                .map(DepOrDepGroupRef::DependencyGroup),
            // We already yielded the parent DependencyGroup, so we don't want to yield the DependencyGroupItem nodes.
            GraphNodeKind::DependencyGroupItem(_) => None,
        }
    })
}

pub(crate) fn all_dependants_for_node<D: Dependency>(
    dep: &DependencyGraph<D>,
    node_idx: NodeIndex,
) -> impl Iterator<Item = DepOrDepGroupItemRef<'_, D>> {
    let mut node_iter = Bfs::new(&dep.graph, node_idx).iter(&dep.graph);

    let first_node = node_iter.next(); // Skip the first node, which is the node itself.
    debug_assert_eq!(
        first_node,
        Some(node_idx),
        "The first node in the BFS iterator should be the node itself."
    );

    node_iter.filter_map(|node_idx| {
        let key = &dep.graph[node_idx];
        match key.kind {
            GraphNodeKind::Dependency => dep
                .get_dependency(&key.key)
                .map(DepOrDepGroupItemRef::Dependency),
            GraphNodeKind::DependencyGroup => None, // The Group only depends on its members, so we skip it.
            GraphNodeKind::DependencyGroupItem(idx) => dep
                .get_dependency_group_item(&key.key, idx)
                .map(DepOrDepGroupItemRef::DependencyGroupItem),
        }
    })
}

pub(crate) struct InitChunksIter<'a, D: Dependency> {
    visited: FixedBitSet,
    available: FixedBitSet,
    next: Vec<DepOrDepGroupItemRef<'a, D>>,
}

impl<'a, D: Dependency> InitChunksIter<'a, D> {
    pub(crate) fn new(dep: &'a DependencyGraph<D>) -> Self {
        let mut res = Self {
            visited: dep.graph.visit_map(),
            available: dep.graph.visit_map(),
            next: dep.root_nodes().collect::<Vec<_>>(),
        };

        for root_dep in &res.next {
            res.visited.visit(root_dep.node_index());
        }

        res
    }

    fn mark_as_visited(&mut self, dep: &DepOrDepGroupItemRef<'a, D>) {
        self.visited.visit(dep.node_index());
    }

    fn mark_as_available(&mut self, dep: &DepOrDepGroupItemRef<'a, D>) {
        match dep {
            DepOrDepGroupItemRef::Dependency(dep) => {
                self.available.visit(dep.entry.graph_node);
            }
            DepOrDepGroupItemRef::DependencyGroupItem(group_item) => {
                self.available.visit(group_item.node_idx);
                // Check if all items in the group are available, and if so, mark the group as available as well.
                let group = group_item.group();
                if group
                    .items()
                    .all(|item| self.available.is_visited(&item.node_idx))
                {
                    self.available.visit(group.group_entry.group_node);
                }
            }
        }
    }

    fn try_visit_dep_or_group_item(
        &mut self,
        graph: &'a DependencyGraph<D>,
        dep: DepOrDepGroupItemRef<'a, D>,
    ) {
        if self.visited.is_visited(&dep.node_index()) {
            return;
        }

        if raw_direct_dependencies_for_node(graph, dep.node_index())
            .all(|direct_dep_node_idx| self.available.is_visited(&direct_dep_node_idx))
        {
            self.mark_as_visited(&dep);
            self.next.push(dep);
        }
    }
}

impl<'a, D: Dependency> Iterator for InitChunksIter<'a, D> {
    type Item = Vec<DepOrDepGroupItemRef<'a, D>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_empty() {
            return None;
        }

        let result = mem::take(&mut self.next);

        // Mark the yielded dependencies as available, so that their dependants can be yielded in the next iteration.
        for dep in &result {
            self.mark_as_available(dep);
        }

        // Add all the direct dependants of the yielded dependencies to the next queue, if they can be yielded.
        for dep in &result {
            match dep {
                DepOrDepGroupItemRef::Dependency(dep) => {
                    for dependant in dep.direct_dependants() {
                        self.try_visit_dep_or_group_item(dep.dep, dependant);
                    }
                }
                DepOrDepGroupItemRef::DependencyGroupItem(group_item) => {
                    let group = group_item.group();
                    if !self.available.is_visited(&group.group_entry.group_node) {
                        continue; // The group is not available yet, so we can't yield its dependants.
                    }
                    for dependant in group.direct_dependants() {
                        self.try_visit_dep_or_group_item(group.dep, dependant);
                    }
                }
            }
        }

        Some(result)
    }
}
