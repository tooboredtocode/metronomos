use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Clone, Copy)]
pub(crate) enum GraphNodeKind {
    Dependency,
    DependencyGroup,
    DependencyGroupItem(u16), // The index of the member in the group
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RootNodeKind {
    Dependency,
    DependencyGroupItem(u16), // The index of the member in the group
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GraphNode<K: Debug + Clone + Eq + Hash> {
    pub(crate) key: K,
    pub(crate) kind: GraphNodeKind,
}

impl<K: Debug + Clone + Eq + Hash> GraphNode<K> {
    pub(crate) fn new_dependency(key: K) -> Self {
        Self {
            key,
            kind: GraphNodeKind::Dependency,
        }
    }

    pub(crate) fn new_dependency_group(key: K) -> Self {
        Self {
            key,
            kind: GraphNodeKind::DependencyGroup,
        }
    }

    pub(crate) fn new_dependency_group_item(key: K, index: u16) -> Self {
        Self {
            key,
            kind: GraphNodeKind::DependencyGroupItem(index),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RootNode<K: Debug + Clone + Eq + Hash> {
    pub(crate) key: K,
    pub(crate) kind: RootNodeKind,
}

impl<K: Debug + Clone + Eq + Hash> RootNode<K> {
    pub(crate) fn new_dependency(key: K) -> Self {
        Self {
            key,
            kind: RootNodeKind::Dependency,
        }
    }

    pub(crate) fn new_dependency_group_item(key: K, index: u16) -> Self {
        Self {
            key,
            kind: RootNodeKind::DependencyGroupItem(index),
        }
    }
}
