//! Reference types for accessing dependencies in the graph.

use daggy::{NodeIndex, Walker};
use itertools::Either;

use crate::dependency::Dependency;
use crate::entry::{DependencyEntry, DependencyGroupEntry};
use crate::graph::DependencyGraph;
use crate::iters::{AllDependantsIter, AllDependenciesIter};
use crate::node_descriptors::{GraphNodeKind, RootNode};

/// A reference to either a dependency or a dependency group.
pub enum DepOrDepGroupRef<'a, D: Dependency> {
    /// A reference to a single dependency.
    Dependency(DependencyRef<'a, D>),
    /// A reference to a dependency group.
    DependencyGroup(DependencyGroupRef<'a, D>),
}

/// A reference to either a dependency or a dependency group item.
pub enum DepOrDepGroupItemRef<'a, D: Dependency> {
    /// A reference to a single dependency.
    Dependency(DependencyRef<'a, D>),
    /// A reference to an item in a dependency group.
    DependencyGroupItem(DependencyGroupItemRef<'a, D>),
}

/// An enum representing any type of graph reference.
pub enum DependencyAnyRef<'a, D: Dependency> {
    /// A reference to a single dependency.
    Dependency(DependencyRef<'a, D>),
    /// A reference to a dependency group.
    DependencyGroup(DependencyGroupRef<'a, D>),
    /// A reference to an item in a dependency group.
    DependencyGroupItem(DependencyGroupItemRef<'a, D>),
}

/// An immutable reference to a dependency in the graph.
pub struct DependencyRef<'a, D: Dependency> {
    pub(crate) dep: &'a DependencyGraph<D>,
    pub(crate) key: &'a D::Key,
    pub(crate) entry: &'a DependencyEntry<D>,
}

/// An immutable reference to a dependency group in the graph.
pub struct DependencyGroupRef<'a, D: Dependency> {
    pub(crate) dep: &'a DependencyGraph<D>,
    pub(crate) key: &'a D::Key,
    pub(crate) group_entry: &'a DependencyGroupEntry<D>,
}

/// An immutable reference to an item in a dependency group.
pub struct DependencyGroupItemRef<'a, D: Dependency> {
    pub(crate) dep: &'a DependencyGraph<D>,
    pub(crate) key: &'a D::Key,
    pub(crate) group_entry: &'a DependencyGroupEntry<D>,
    pub(crate) dependency: &'a D,
    pub(crate) idx: u16,
    pub(crate) node_idx: NodeIndex,
}

macro_rules! impl_copy_clone {
    ($t:ty) => {
        impl<'a, D: Dependency> Clone for $t {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<'a, D: Dependency> Copy for $t {}
    };
}

impl_copy_clone!(DepOrDepGroupRef<'a, D>);
impl_copy_clone!(DepOrDepGroupItemRef<'a, D>);
impl_copy_clone!(DependencyAnyRef<'a, D>);
impl_copy_clone!(DependencyRef<'a, D>);
impl_copy_clone!(DependencyGroupRef<'a, D>);
impl_copy_clone!(DependencyGroupItemRef<'a, D>);

impl<'a, D: Dependency> DepOrDepGroupItemRef<'a, D> {
    /// Returns a reference to the inner dependency.
    pub fn inner(&self) -> &'a D {
        match self {
            DepOrDepGroupItemRef::Dependency(dep_ref) => dep_ref.inner(),
            DepOrDepGroupItemRef::DependencyGroupItem(group_item_ref) => group_item_ref.inner(),
        }
    }

    /// Returns true if this item is a root node in the graph.
    pub fn is_root(&self) -> bool {
        match self {
            DepOrDepGroupItemRef::Dependency(dep_ref) => dep_ref.is_root(),
            DepOrDepGroupItemRef::DependencyGroupItem(group_item_ref) => group_item_ref.is_root(),
        }
    }
}

impl<'a, D: Dependency> DependencyRef<'a, D> {
    /// Returns a reference to the underlying dependency.
    pub fn inner(&self) -> &'a D {
        &self.entry.dependency
    }

    /// Returns true if this dependency is a root node in the graph.
    pub fn is_root(&self) -> bool {
        self.dep
            .root_nodes
            .contains(&RootNode::new_dependency(self.key.clone()))
    }
}

impl<'a, D: Dependency> DependencyGroupRef<'a, D> {
    /// Returns the descriptor of the dependency group.
    pub fn descriptor(&self) -> &'a D::Shallow {
        &self.group_entry.shallow
    }

    /// Returns a slice of all dependencies in the group.
    pub fn inner(&self) -> &'a [D] {
        &self.group_entry.dependencies
    }

    /// Returns a reference to the item at the given index in the group.
    pub fn get_item(&self, idx: u16) -> Option<DependencyGroupItemRef<'a, D>> {
        self.group_entry.dependencies
            .get(idx as usize)
            .and_then(|dependency| {
                match self.group_entry.graph_nodes.get(idx as usize) {
                    Some(node_idx) => Some((dependency, node_idx)),
                    None if cfg!(debug_assertions) => panic!("DependencyGroupEntry has a dependency at index {} but no corresponding graph node!", idx),
                    None => None,
                }
            })
            .map(|(dependency, node_idx)| DependencyGroupItemRef {
                dep: self.dep,
                key: self.key,
                group_entry: self.group_entry,
                dependency,
                idx,
                node_idx: *node_idx,
            })
    }

    /// Returns an iterator over all items in the dependency group.
    pub fn items(&self) -> impl Iterator<Item = DependencyGroupItemRef<'a, D>> {
        self.group_entry
            .dependencies
            .iter()
            .zip(self.group_entry.graph_nodes.iter())
            .enumerate()
            .map(
                move |(idx, (dependency, &node_idx))| DependencyGroupItemRef {
                    dep: self.dep,
                    key: self.key,
                    group_entry: self.group_entry,
                    dependency,
                    idx: idx as u16,
                    node_idx,
                },
            )
    }
}

impl<'a, D: Dependency> DependencyGroupItemRef<'a, D> {
    /// Returns a reference to the underlying dependency.
    pub fn inner(&self) -> &'a D {
        self.dependency
    }

    /// Returns true if this item is a root node in the graph.
    pub fn is_root(&self) -> bool {
        self.dep
            .root_nodes
            .contains(&RootNode::new_dependency_group_item(
                self.key.clone(),
                self.idx,
            ))
    }

    /// Returns a reference to the parent DependencyGroupRef of this DependencyGroupItemRef.
    pub fn group(&self) -> DependencyGroupRef<'a, D> {
        DependencyGroupRef {
            dep: self.dep,
            key: self.key,
            group_entry: self.group_entry,
        }
    }

    /// Returns the index of this DependencyGroupItemRef within its parent DependencyGroupRef.
    pub fn index(&self) -> u16 {
        self.idx
    }
}

fn direct_dependencies_for_node<D: Dependency>(
    dep: &DependencyGraph<D>,
    node_idx: NodeIndex,
) -> impl Iterator<Item = DepOrDepGroupRef<'_, D>> {
    dep.graph
        .parents(node_idx)
        .iter(&dep.graph)
        .filter_map(|(_, node_idx)| {
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

macro_rules! impl_direct_dependencies {
    ($($struct_name:ident ( $self:ident ) => $node_expr:expr ),* $(,)?) => {
        $(
            impl<'a, D: Dependency> $struct_name<'a, D> {
                /// An iterator of all the direct dependencies. Transitive dependencies are not returned.
                ///
                /// ### Example
                /// If the Dependency graph is built with the following Dependencies:
                /// - A
                /// - B
                /// - C depends on A, B
                /// - D depends on A
                /// - F depends on B, C
                ///
                /// Then this method would return the following:
                /// - A: ()
                /// - B: ()
                /// - C: (A, B)
                /// - D: (A)
                /// - F: (C)
                pub fn direct_dependencies($self) -> impl Iterator<Item= DepOrDepGroupRef<'a, D>> {
                    direct_dependencies_for_node($self.dep, $node_expr)
                }
            }
        )*
    };
}

impl_direct_dependencies! {
    DependencyRef(self) => self.entry.graph_node,
    DependencyGroupItemRef(self) => self.node_idx,
}

impl<'a, D: Dependency> DepOrDepGroupItemRef<'a, D> {
    /// An iterator of all the direct dependencies. Transitive dependencies are not returned.
    ///
    /// ### Note
    /// This is just a convenience method that calls `direct_dependencies` on the inner `DependencyRef` or `DependencyGroupItemRef`.
    pub fn direct_dependencies(self) -> impl Iterator<Item = DepOrDepGroupRef<'a, D>> {
        match self {
            DepOrDepGroupItemRef::Dependency(dep_ref) => {
                Either::Left(dep_ref.direct_dependencies())
            }
            DepOrDepGroupItemRef::DependencyGroupItem(group_item_ref) => {
                Either::Right(group_item_ref.direct_dependencies())
            }
        }
    }
}

fn direct_dependants_for_node<D: Dependency>(
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

macro_rules! impl_direct_dependants {
    ($($struct_name:ident ( $self:ident ) => $node_expr:expr ),* $(,)?) => {
        $(
            impl<'a, D: Dependency> $struct_name<'a, D> {
                /// An iterator of all the direct dependants. Transitive dependants are not returned.
                ///
                /// ### Example
                /// If the Dependency graph is built with the following Dependencies:
                /// - A
                /// - B
                /// - C depends on A, B
                /// - D depends on A
                /// - F depends on B, C
                ///
                /// Then this method would return the following:
                /// - A: (C, D)
                /// - B: (C, F)
                /// - C: (F)
                /// - D: ()
                /// - F: ()
                                pub fn direct_dependants($self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
                    direct_dependants_for_node($self.dep, $node_expr)
                }
            }
        )*
    };
}

impl_direct_dependants! {
    DependencyRef(self) => self.entry.graph_node,
    DependencyGroupRef(self) => self.group_entry.group_node,
}

impl<'a, D: Dependency> DependencyGroupItemRef<'a, D> {
    /// An iterator of all the direct dependants. Transitive dependants are not returned.
    ///
    /// ### Note
    /// This is just a convenience method that calls `direct_dependants` on the parent `DependencyGroupRef`.
    pub fn direct_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        self.group().direct_dependants()
    }
}

impl<'a, D: Dependency> DepOrDepGroupRef<'a, D> {
    /// An iterator of all the direct dependants. Transitive dependants are not returned.
    ///
    /// ### Note
    /// This is just a convenience method that calls `direct_dependants` on the inner `DependencyRef` or `DependencyGroupRef`.
    pub fn direct_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        match self {
            DepOrDepGroupRef::Dependency(dep_ref) => Either::Left(dep_ref.direct_dependants()),
            DepOrDepGroupRef::DependencyGroup(group_ref) => {
                Either::Right(group_ref.direct_dependants())
            }
        }
    }
}

impl<'a, D: Dependency> DepOrDepGroupItemRef<'a, D> {
    /// An iterator of all the direct dependants. Transitive dependants are not returned.
    ///
    /// ### Note
    /// This is just a convenience method that calls `direct_dependants` on the inner `DependencyRef` or `DependencyGroupItemRef`.
    pub fn direct_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        match self {
            DepOrDepGroupItemRef::Dependency(dep_ref) => Either::Left(dep_ref.direct_dependants()),
            DepOrDepGroupItemRef::DependencyGroupItem(group_item_ref) => {
                Either::Right(group_item_ref.direct_dependants())
            }
        }
    }
}

impl<'a, D: Dependency> DependencyRef<'a, D> {
    /// An iterator of all the dependencies, including transitive dependencies.
    ///
    /// ### Note
    /// Groups are skipped, so if a dependency depends on a group, the group will not be returned,
    /// but the dependencies in the group will be returned.
    pub fn all_dependencies(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        self.all_dependencies_inner()
    }

    fn all_dependencies_inner(self) -> AllDependenciesIter<'a, D> {
        AllDependenciesIter::new(DepOrDepGroupItemRef::Dependency(self))
    }

    /// An iterator of all the dependants, including transitive dependants.
    pub fn all_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        self.all_dependants_inner()
    }

    fn all_dependants_inner(self) -> AllDependantsIter<'a, D> {
        AllDependantsIter::new(DepOrDepGroupItemRef::Dependency(self))
    }
}

impl<'a, D: Dependency> DependencyGroupRef<'a, D> {
    /// An iterator of aller the dependants, including transitive dependants.
    pub fn all_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        self.all_dependants_inner()
    }

    fn all_dependants_inner(self) -> AllDependantsIter<'a, D> {
        AllDependantsIter::new_from_group(self)
    }
}

impl<'a, D: Dependency> DepOrDepGroupRef<'a, D> {
    /// An iterator of all the dependants, including transitive dependants.
    ///
    /// ### Note
    /// This is just a convenience method that calls `all_dependants` on the inner `DependencyRef`
    /// or `DependencyGroupRef`.
    pub fn all_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        match self {
            DepOrDepGroupRef::Dependency(dep_ref) => dep_ref.all_dependants_inner(),
            DepOrDepGroupRef::DependencyGroup(group_ref) => group_ref.all_dependants_inner(),
        }
    }
}

impl<'a, D: Dependency> DependencyGroupItemRef<'a, D> {
    /// An iterator of all the dependencies, including transitive dependencies.
    ///
    /// ### Note
    /// Groups are skipped, so if a dependency depends on a group, the group will not be returned,
    /// but the dependencies in the group will be returned.
    pub fn all_dependencies(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        self.all_dependencies_inner()
    }

    fn all_dependencies_inner(self) -> AllDependenciesIter<'a, D> {
        AllDependenciesIter::new(DepOrDepGroupItemRef::DependencyGroupItem(self))
    }

    /// An iterator of all the dependants, including transitive dependants.
    pub fn all_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        self.all_dependants_inner()
    }

    fn all_dependants_inner(self) -> AllDependantsIter<'a, D> {
        AllDependantsIter::new(DepOrDepGroupItemRef::DependencyGroupItem(self))
    }
}

impl<'a, D: Dependency> DepOrDepGroupItemRef<'a, D> {
    /// An iterator of all the dependencies, including transitive dependencies.
    ///
    /// ### Note
    /// This is just a convenience method that calls `all_dependencies` on the inner `DependencyRef`
    /// or `DependencyGroupItemRef`.
    pub fn all_dependencies(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        match self {
            DepOrDepGroupItemRef::Dependency(dep_ref) => dep_ref.all_dependencies_inner(),
            DepOrDepGroupItemRef::DependencyGroupItem(group_item_ref) => {
                group_item_ref.all_dependencies_inner()
            }
        }
    }

    /// An iterator of all the dependants, including transitive dependants.
    ///
    /// ### Note
    /// This is just a convenience method that calls `all_dependants` on the inner `DependencyRef`
    /// or `DependencyGroupItemRef`.
    pub fn all_dependants(self) -> impl Iterator<Item = DepOrDepGroupItemRef<'a, D>> {
        match self {
            DepOrDepGroupItemRef::Dependency(dep_ref) => dep_ref.all_dependants_inner(),
            DepOrDepGroupItemRef::DependencyGroupItem(group_item_ref) => {
                group_item_ref.all_dependants_inner()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::test_utils::*;

    #[test]
    fn test_direct_dependencies_var_a() {
        let graph = create_dependency_graph_var_a();

        let dep_a = graph.get_dependency(&"Dep A").unwrap();
        let direct_deps_a = ref_iter_set_with_group(dep_a.direct_dependencies());
        assert_eq!(direct_deps_a, HashSet::new());

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let direct_deps_b = ref_iter_set_with_group(dep_b.direct_dependencies());
        assert_eq!(direct_deps_b, HashSet::new());

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let direct_deps = ref_iter_set_with_group(dep_c.direct_dependencies());
        assert_eq!(direct_deps, HashSet::from_iter(["Dep A", "Dep B"]));

        let dep_d = graph.get_dependency(&"Dep D").unwrap();
        let direct_deps_d = ref_iter_set_with_group(dep_d.direct_dependencies());
        assert_eq!(direct_deps_d, HashSet::from_iter(["Dep A"]));

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let direct_deps_f = ref_iter_set_with_group(dep_f.direct_dependencies());
        assert_eq!(direct_deps_f, HashSet::from_iter(["Dep C"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let direct_deps_g = ref_iter_set_with_group(dep_g.direct_dependencies());
        assert_eq!(direct_deps_g, HashSet::from_iter(["Dep F"]));
    }

    #[test]
    fn test_direct_dependencies_var_b() {
        let graph = create_dependency_graph_var_b();

        let dep_a = graph.get_dependency_group(&"Dep A").unwrap();
        let direct_deps_a =
            ref_iter_set_with_group(dep_a.items().flat_map(|item| item.direct_dependencies()));
        assert_eq!(direct_deps_a, HashSet::new());

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let direct_deps_b = ref_iter_set_with_group(dep_b.direct_dependencies());
        assert_eq!(direct_deps_b, HashSet::new());

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let direct_deps_c = ref_iter_set_with_group(dep_c.direct_dependencies());
        assert_eq!(direct_deps_c, HashSet::from_iter(["Dep A", "Dep B"]));

        let dep_d_0 = graph.get_dependency_group_item(&"Dep D", 0).unwrap();
        let direct_deps_d_0 = ref_iter_set_with_group(dep_d_0.direct_dependencies());
        assert_eq!(direct_deps_d_0, HashSet::from_iter(["Dep A"]));

        let dep_d_1 = graph.get_dependency_group_item(&"Dep D", 1).unwrap();
        let direct_deps_d_1 = ref_iter_set_with_group(dep_d_1.direct_dependencies());
        assert_eq!(direct_deps_d_1, HashSet::new());

        let dep_d_2 = graph.get_dependency_group_item(&"Dep D", 2).unwrap();
        let direct_deps_d_2 = ref_iter_set_with_group(dep_d_2.direct_dependencies());
        assert_eq!(direct_deps_d_2, HashSet::from_iter(["Dep B"]));

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let direct_deps_f = ref_iter_set_with_group(dep_f.direct_dependencies());
        assert_eq!(direct_deps_f, HashSet::from_iter(["Dep C"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let direct_deps_g = ref_iter_set_with_group(dep_g.direct_dependencies());
        assert_eq!(direct_deps_g, HashSet::from_iter(["Dep F"]));
    }

    #[test]
    fn test_all_dependencies_var_a() {
        let graph = create_dependency_graph_var_a();

        let dep_a = graph.get_dependency(&"Dep A").unwrap();
        let all_deps_a = ref_iter_set(dep_a.all_dependencies());
        assert_eq!(all_deps_a, HashSet::new());

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let all_deps_b = ref_iter_set(dep_b.all_dependencies());
        assert_eq!(all_deps_b, HashSet::new());

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let all_deps_c = ref_iter_set(dep_c.all_dependencies());
        assert_eq!(all_deps_c, HashSet::from_iter(["Dep A", "Dep B"]));

        let dep_d = graph.get_dependency(&"Dep D").unwrap();
        let all_deps_d = ref_iter_set(dep_d.all_dependencies());
        assert_eq!(all_deps_d, HashSet::from_iter(["Dep A"]));

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let all_deps_f = ref_iter_set(dep_f.all_dependencies());
        assert_eq!(all_deps_f, HashSet::from_iter(["Dep A", "Dep B", "Dep C"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let all_deps_g = ref_iter_set(dep_g.all_dependencies());
        assert_eq!(
            all_deps_g,
            HashSet::from_iter(["Dep A", "Dep B", "Dep C", "Dep F"])
        );
    }

    #[test]
    fn test_all_dependencies_var_b() {
        let graph = create_dependency_graph_var_b();

        let dep_a = graph.get_dependency_group(&"Dep A").unwrap();
        let all_deps_a = ref_iter_set(dep_a.items().flat_map(|item| item.all_dependencies()));
        assert_eq!(all_deps_a, HashSet::new());

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let all_deps_b = ref_iter_set(dep_b.all_dependencies());
        assert_eq!(all_deps_b, HashSet::new());

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let all_deps_c = ref_iter_set(dep_c.all_dependencies());
        assert_eq!(all_deps_c, HashSet::from_iter(["Dep A", "Dep B"]));

        let dep_d_0 = graph.get_dependency_group_item(&"Dep D", 0).unwrap();
        let all_deps_d_0 = ref_iter_set(dep_d_0.all_dependencies());
        assert_eq!(all_deps_d_0, HashSet::from_iter(["Dep A"]));

        let dep_d_1 = graph.get_dependency_group_item(&"Dep D", 1).unwrap();
        let all_deps_d_1 = ref_iter_set(dep_d_1.all_dependencies());
        assert_eq!(all_deps_d_1, HashSet::new());

        let dep_d_2 = graph.get_dependency_group_item(&"Dep D", 2).unwrap();
        let all_deps_d_2 = ref_iter_set(dep_d_2.all_dependencies());
        assert_eq!(all_deps_d_2, HashSet::from_iter(["Dep B"]));

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let all_deps_f = ref_iter_set(dep_f.all_dependencies());
        assert_eq!(all_deps_f, HashSet::from_iter(["Dep A", "Dep B", "Dep C"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let all_deps_g = ref_iter_set(dep_g.all_dependencies());
        assert_eq!(
            all_deps_g,
            HashSet::from_iter(["Dep A", "Dep B", "Dep C", "Dep F"])
        );
    }

    #[test]
    fn test_direct_dependants_var_a() {
        let graph = create_dependency_graph_var_a();

        let dep_a = graph.get_dependency(&"Dep A").unwrap();
        let direct_dependants_a = ref_iter_set(dep_a.direct_dependants());
        assert_eq!(direct_dependants_a, HashSet::from_iter(["Dep C", "Dep D"]));

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let direct_dependants_b = ref_iter_set(dep_b.direct_dependants());
        assert_eq!(direct_dependants_b, HashSet::from_iter(["Dep C"]));

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let direct_dependants_c = ref_iter_set(dep_c.direct_dependants());
        assert_eq!(direct_dependants_c, HashSet::from_iter(["Dep F"]));

        let dep_d = graph.get_dependency(&"Dep D").unwrap();
        let direct_dependants_d = ref_iter_set(dep_d.direct_dependants());
        assert_eq!(direct_dependants_d, HashSet::new());

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let direct_dependants_f = ref_iter_set(dep_f.direct_dependants());
        assert_eq!(direct_dependants_f, HashSet::from_iter(["Dep G"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let direct_dependants_g = ref_iter_set(dep_g.direct_dependants());
        assert_eq!(direct_dependants_g, HashSet::new());
    }

    #[test]
    fn test_direct_dependants_var_b() {
        let graph = create_dependency_graph_var_b();

        let dep_a = graph.get_dependency_group(&"Dep A").unwrap();
        let direct_dependants_a = ref_iter_set(dep_a.direct_dependants());
        assert_eq!(direct_dependants_a, HashSet::from_iter(["Dep C", "Dep D"]));

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let direct_dependants_b = ref_iter_set(dep_b.direct_dependants());
        assert_eq!(direct_dependants_b, HashSet::from_iter(["Dep C", "Dep D"]));

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let direct_dependants_c = ref_iter_set(dep_c.direct_dependants());
        assert_eq!(direct_dependants_c, HashSet::from_iter(["Dep F"]));

        let dep_d = graph.get_dependency_group(&"Dep D").unwrap();
        let direct_dependants_d = ref_iter_set(dep_d.direct_dependants());
        assert_eq!(direct_dependants_d, HashSet::new());

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let direct_dependants_f = ref_iter_set(dep_f.direct_dependants());
        assert_eq!(direct_dependants_f, HashSet::from_iter(["Dep G"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let direct_dependants_g = ref_iter_set(dep_g.direct_dependants());
        assert_eq!(direct_dependants_g, HashSet::new());
    }

    #[test]
    fn test_all_dependants_var_a() {
        let graph = create_dependency_graph_var_a();

        let dep_a = graph.get_dependency(&"Dep A").unwrap();
        let all_dependants_a = ref_iter_set(dep_a.all_dependants());
        assert_eq!(
            all_dependants_a,
            HashSet::from_iter(["Dep C", "Dep D", "Dep F", "Dep G"])
        );

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let all_dependants_b = ref_iter_set(dep_b.all_dependants());
        assert_eq!(
            all_dependants_b,
            HashSet::from_iter(["Dep C", "Dep F", "Dep G"])
        );

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let all_dependants_c = ref_iter_set(dep_c.all_dependants());
        assert_eq!(all_dependants_c, HashSet::from_iter(["Dep F", "Dep G"]));

        let dep_d = graph.get_dependency(&"Dep D").unwrap();
        let all_dependants_d = ref_iter_set(dep_d.all_dependants());
        assert_eq!(all_dependants_d, HashSet::new());

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let all_dependants_f = ref_iter_set(dep_f.all_dependants());
        assert_eq!(all_dependants_f, HashSet::from_iter(["Dep G"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let all_dependants_g = ref_iter_set(dep_g.all_dependants());
        assert_eq!(all_dependants_g, HashSet::new());
    }

    #[test]
    fn test_all_dependants_var_b() {
        let graph = create_dependency_graph_var_b();

        let dep_a = graph.get_dependency_group(&"Dep A").unwrap();
        let all_dependants_a = ref_iter_set(dep_a.all_dependants());
        assert_eq!(
            all_dependants_a,
            HashSet::from_iter(["Dep C", "Dep D", "Dep F", "Dep G"])
        );

        let dep_b = graph.get_dependency(&"Dep B").unwrap();
        let all_dependants_b = ref_iter_set(dep_b.all_dependants());
        assert_eq!(
            all_dependants_b,
            HashSet::from_iter(["Dep C", "Dep D", "Dep F", "Dep G"])
        );

        let dep_c = graph.get_dependency(&"Dep C").unwrap();
        let all_dependants_c = ref_iter_set(dep_c.all_dependants());
        assert_eq!(all_dependants_c, HashSet::from_iter(["Dep F", "Dep G"]));

        let dep_d = graph.get_dependency_group(&"Dep D").unwrap();
        let all_dependants_d = ref_iter_set(dep_d.all_dependants());
        assert_eq!(all_dependants_d, HashSet::new());

        let dep_f = graph.get_dependency(&"Dep F").unwrap();
        let all_dependants_f = ref_iter_set(dep_f.all_dependants());
        assert_eq!(all_dependants_f, HashSet::from_iter(["Dep G"]));

        let dep_g = graph.get_dependency(&"Dep G").unwrap();
        let all_dependants_g = ref_iter_set(dep_g.all_dependants());
        assert_eq!(all_dependants_g, HashSet::new());
    }
}
