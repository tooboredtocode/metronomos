use core::fmt;
use std::any::TypeId;
use std::ops::Deref;
use std::sync::Arc;

use metronomos_loom::DependencyGraph;
use metronomos_loom::dependency::ShallowDependency;
use metronomos_loom::graph_ref::DependencyAnyRef;

use crate::dependency::PulseDependency;

/// A DOT-format string representation of the dependency graph.
///
/// `DotString` is always present in every [`PulseContainer`](crate::PulseContainer) and provides
/// a textual Graphviz/DOT representation of the full dependency graph — including all dependencies,
/// groups, and finalizers.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DotString(Arc<str>);

impl fmt::Debug for DotString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DotString({})", self.inner())
    }
}

impl fmt::Display for DotString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner())
    }
}

impl Deref for DotString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl AsRef<str> for DotString {
    fn as_ref(&self) -> &str {
        self.inner()
    }
}

impl DotString {
    pub(crate) fn make(graph: &DependencyGraph<PulseDependency>) -> Self {
        let dot_string = graph
            .display_filtered(|n| match n {
                DependencyAnyRef::DependencyGroup(g) => {
                    g.descriptor().type_id != TypeId::of::<()>()
                }
                _ => true,
            })
            .with_format(|f, node| match node {
                DependencyAnyRef::Dependency(d) => f.write_str(d.inner().name()),
                DependencyAnyRef::DependencyGroup(g) => {
                    write!(f, "Group: {}", g.descriptor().name())
                }
                DependencyAnyRef::DependencyGroupItem(gi)
                    if gi.inner().info.type_id == TypeId::of::<()>() =>
                {
                    write!(f, "Finalizer {}", gi.index())
                }
                DependencyAnyRef::DependencyGroupItem(gi) => {
                    write!(f, "{}: {}", gi.inner().name(), gi.index())
                }
            })
            .to_string()
            .into_boxed_str();
        DotString(Arc::from(dot_string))
    }

    fn inner(&self) -> &str {
        self.0.deref()
    }
}
