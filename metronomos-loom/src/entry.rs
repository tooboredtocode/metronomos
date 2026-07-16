use daggy::NodeIndex;

use crate::dependency::{Dependency, DependencyItem};

pub(crate) enum MaybeDependency<D: Dependency> {
    Missing(DependencyItem<D::Shallow>),
    Present(D),
}

pub(crate) struct MaybeDependencyEntry<D: Dependency> {
    pub(crate) dependency: MaybeDependency<D>,
    pub(crate) graph_node: NodeIndex,
}

pub(crate) struct DependencyEntry<D: Dependency> {
    pub(crate) dependency: D,
    pub(crate) graph_node: NodeIndex,
}

pub(crate) struct DependencyGroupEntry<D: Dependency> {
    pub(crate) shallow: D::Shallow,
    pub(crate) dependencies: Vec<D>,
    pub(crate) graph_nodes: Vec<NodeIndex>,
    pub(crate) group_node: NodeIndex,
}

impl<D: Dependency> MaybeDependencyEntry<D> {
    pub(crate) fn new(graph_node: NodeIndex, dependency: DependencyItem<D::Shallow>) -> Self {
        Self {
            dependency: MaybeDependency::Missing(dependency),
            graph_node,
        }
    }

    pub(crate) fn new_present(graph_node: NodeIndex, dependency: D) -> Self {
        Self {
            dependency: MaybeDependency::Present(dependency),
            graph_node,
        }
    }

    pub(crate) fn is_optional(&self) -> bool {
        matches!(&self.dependency, MaybeDependency::Missing(dep) if !dep.is_required())
    }

    pub(crate) fn is_present(&self) -> bool {
        matches!(self.dependency, MaybeDependency::Present(_))
    }

    pub(crate) fn make_missing_required(&mut self) {
        if let MaybeDependency::Missing(shallow) = &mut self.dependency {
            shallow.set_required(true);
        }
    }

    pub(crate) fn set_dependency(&mut self, dependency: D) -> Option<D> {
        match &mut self.dependency {
            MaybeDependency::Present(existing) => {
                let old_dependency = std::mem::replace(existing, dependency);
                Some(old_dependency)
            }
            MaybeDependency::Missing(_) => {
                self.dependency = MaybeDependency::Present(dependency);
                None
            }
        }
    }

    pub(crate) fn try_into_dependency_entry(
        self,
    ) -> Result<Option<DependencyEntry<D>>, D::Shallow> {
        match self.dependency {
            MaybeDependency::Present(dependency) => Ok(Some(DependencyEntry {
                dependency,
                graph_node: self.graph_node,
            })),
            MaybeDependency::Missing(shallow) => {
                if shallow.is_required() {
                    Err(shallow.into_inner())
                } else {
                    Ok(None)
                }
            }
        }
    }
}

impl<D: Dependency> DependencyGroupEntry<D> {
    pub(crate) fn new(group_node: NodeIndex, shallow: D::Shallow) -> Self {
        Self {
            shallow,
            dependencies: Vec::new(),
            graph_nodes: Vec::new(),
            group_node,
        }
    }
}
