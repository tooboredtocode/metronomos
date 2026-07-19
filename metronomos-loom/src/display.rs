//! A display implementation for the dependency graph.
//!
//! This module provides [`DependencyGraphDisplay`], which outputs the dependency graph in DOT
//! format, suitable for visualization with tools like Graphviz.

use std::fmt;

use daggy::petgraph::prelude::EdgeRef;
use daggy::petgraph::visit::{IntoEdgeReferences, IntoNodeReferences};

use crate::dependency::{Dependency, ShallowDependency};
use crate::graph::DependencyGraph;
use crate::graph_ref::DependencyAnyRef;
use crate::node_descriptors::{GraphNode, GraphNodeKind};

/// A display type for the dependency graph.
///
/// [`DependencyGraphDisplay`] provides a way to display the dependency graph in DOT format,
/// which can be used with visualization tools like Graphviz.
pub struct DependencyGraphDisplay<'a, D: Dependency> {
    nodes: Vec<Option<DependencyAnyRef<'a, D>>>,
    edges: Vec<(usize, usize)>,
}

struct DependencyGraphDisplayWithFormat<'a, D: Dependency, Fmt> {
    parent: DependencyGraphDisplay<'a, D>,
    format_node: Fmt,
}

static TYPE: &str = "digraph";
static INDENT: &str = "    ";

fn graph_node_to_ref<'a, D: Dependency>(
    graph: &'a DependencyGraph<D>,
    key: &GraphNode<D::Key>,
) -> Option<DependencyAnyRef<'a, D>> {
    match key.kind {
        GraphNodeKind::Dependency => graph
            .get_dependency(&key.key)
            .map(DependencyAnyRef::Dependency),
        GraphNodeKind::DependencyGroup => graph
            .get_dependency_group(&key.key)
            .map(DependencyAnyRef::DependencyGroup),
        GraphNodeKind::DependencyGroupItem(idx) => graph
            .get_dependency_group_item(&key.key, idx)
            .map(DependencyAnyRef::DependencyGroupItem),
    }
}

impl<'a, D> DependencyGraphDisplay<'a, D>
where
    D: Dependency,
{
    /// Returns a display with the default node format.
    pub(crate) fn new(graph: &'a DependencyGraph<D>) -> Self {
        Self::new_filtered(graph, |_| true)
    }

    /// Returns a display with a filtered set of nodes.
    pub(crate) fn new_filtered<F>(graph: &'a DependencyGraph<D>, filter_node: F) -> Self
    where
        F: Fn(DependencyAnyRef<'a, D>) -> bool,
    {
        let nodes = graph
            .graph
            .node_references()
            .map(|(_, key)| {
                let node_ref = graph_node_to_ref(graph, key)?;
                if filter_node(node_ref) {
                    Some(node_ref)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let edges = graph
            .graph
            .edge_references()
            .filter_map(|edge| {
                let source = edge.source().index();
                let target = edge.target().index();
                if nodes[source].is_some() && nodes[target].is_some() {
                    Some((source, target))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Self { nodes, edges }
    }

    /// Formats the graph with the given node formatter.
    fn format_graph_with<Fmt>(&self, f: &mut fmt::Formatter<'_>, format_node: Fmt) -> fmt::Result
    where
        Fmt: Fn(&mut fmt::Formatter<'_>, DependencyAnyRef<'a, D>) -> fmt::Result,
    {
        writeln!(f, "{} {{", TYPE)?;

        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node_ref)| node_ref.map(|node| (index, node)))
            .try_for_each(|(idx, node_ref)| {
                f.write_str(INDENT)?;
                write!(f, "{} [ label=\"", idx)?;
                format_node(f, node_ref)?;
                writeln!(f, "\" ]")
            })?;

        self.edges
            .iter()
            .copied()
            .try_for_each(|(source, target)| {
                f.write_str(INDENT)?;
                writeln!(f, "{} -> {}", source, target)
            })?;

        writeln!(f, "}}")
    }

    /// Returns a display with a custom node formatter.
    pub fn with_format<Fmt>(self, format_node: Fmt) -> impl fmt::Display + 'a
    where
        Fmt: for<'b> Fn(&mut fmt::Formatter<'b>, DependencyAnyRef<'a, D>) -> fmt::Result + 'a,
    {
        DependencyGraphDisplayWithFormat {
            parent: self,
            format_node,
        }
    }
}

/// Formats a node using its name.
fn default_format_node<'a, D: Dependency>(
    f: &mut fmt::Formatter<'_>,
    node_ref: DependencyAnyRef<'a, D>,
) -> fmt::Result {
    match node_ref {
        DependencyAnyRef::Dependency(dep) => f.write_str(dep.inner().name()),
        DependencyAnyRef::DependencyGroup(group) => f.write_str(group.descriptor().name()),
        DependencyAnyRef::DependencyGroupItem(item) => {
            write!(
                f,
                "{} (item {})",
                item.group().descriptor().name(),
                item.index()
            )
        }
    }
}

impl<'a, D> fmt::Display for DependencyGraphDisplay<'a, D>
where
    D: Dependency,
{
    /// Formats the graph using the default node formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_graph_with(f, default_format_node)
    }
}

impl<'a, D, Fmt> fmt::Display for DependencyGraphDisplayWithFormat<'a, D, Fmt>
where
    D: Dependency,
    Fmt: for<'b> Fn(&mut fmt::Formatter<'b>, DependencyAnyRef<'a, D>) -> fmt::Result,
{
    /// Formats the graph using the custom node formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.parent.format_graph_with(f, &self.format_node)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use crate::dependency::ShallowDependency;
    use crate::graph_ref::DependencyAnyRef;
    use crate::test_utils::*;

    #[test]
    fn test_display_var_a() {
        let graph = create_dependency_graph_var_a();

        let dot_string = format!("{}", graph.display());
        let compare_string = r#"digraph {
    0 [ label="Dep A" ]
    1 [ label="Dep B" ]
    2 [ label="Dep C" ]
    3 [ label="Dep D" ]
    4 [ label="Dep F" ]
    5 [ label="Dep G" ]
    0 -> 2
    1 -> 2
    0 -> 3
    4 -> 5
    2 -> 4
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    #[test]
    fn test_display_var_a_filtered() {
        let graph = create_dependency_graph_var_a();

        let dot_string = format!("{}", graph.display_filtered(test_filter));
        let compare_string = r#"digraph {
    0 [ label="Dep A" ]
    2 [ label="Dep C" ]
    3 [ label="Dep D" ]
    4 [ label="Dep F" ]
    5 [ label="Dep G" ]
    0 -> 2
    0 -> 3
    4 -> 5
    2 -> 4
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    #[test]
    fn test_display_var_a_with_format() {
        let graph = create_dependency_graph_var_a();

        let dot_string = format!("{}", graph.display().with_format(test_format_node));
        let compare_string = r#"digraph {
    0 [ label="Dep: Dep A" ]
    1 [ label="Dep: Dep B" ]
    2 [ label="Dep: Dep C" ]
    3 [ label="Dep: Dep D" ]
    4 [ label="Dep: Dep F" ]
    5 [ label="Dep: Dep G" ]
    0 -> 2
    1 -> 2
    0 -> 3
    4 -> 5
    2 -> 4
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    #[test]
    fn test_display_var_b() {
        let graph = create_dependency_graph_var_b();

        let dot_string = format!("{}", graph.display());
        let compare_string = r#"digraph {
    0 [ label="Dep A" ]
    1 [ label="Dep A (item 0)" ]
    2 [ label="Dep A (item 1)" ]
    3 [ label="Dep B" ]
    4 [ label="Dep C" ]
    5 [ label="Dep D" ]
    6 [ label="Dep D (item 0)" ]
    7 [ label="Dep D (item 1)" ]
    8 [ label="Dep D (item 2)" ]
    9 [ label="Dep F" ]
    10 [ label="Dep G" ]
    1 -> 0
    2 -> 0
    0 -> 4
    3 -> 4
    6 -> 5
    0 -> 6
    7 -> 5
    8 -> 5
    3 -> 8
    9 -> 10
    4 -> 9
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    #[test]
    fn test_display_var_b_filtered() {
        let graph = create_dependency_graph_var_b();

        let dot_string = format!("{}", graph.display_filtered(test_filter));
        let compare_string = r#"digraph {
    0 [ label="Dep A" ]
    1 [ label="Dep A (item 0)" ]
    2 [ label="Dep A (item 1)" ]
    4 [ label="Dep C" ]
    5 [ label="Dep D" ]
    6 [ label="Dep D (item 0)" ]
    7 [ label="Dep D (item 1)" ]
    8 [ label="Dep D (item 2)" ]
    9 [ label="Dep F" ]
    10 [ label="Dep G" ]
    1 -> 0
    2 -> 0
    0 -> 4
    6 -> 5
    0 -> 6
    7 -> 5
    8 -> 5
    9 -> 10
    4 -> 9
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    #[test]
    fn test_display_var_b_with_format() {
        let graph = create_dependency_graph_var_b();

        let dot_string = format!("{}", graph.display().with_format(test_format_node));
        let compare_string = r#"digraph {
    0 [ label="Group: Dep A" ]
    1 [ label="Group Item 0: Dep A" ]
    2 [ label="Group Item 1: Dep A" ]
    3 [ label="Dep: Dep B" ]
    4 [ label="Dep: Dep C" ]
    5 [ label="Group: Dep D" ]
    6 [ label="Group Item 0: Dep D" ]
    7 [ label="Group Item 1: Dep D" ]
    8 [ label="Group Item 2: Dep D" ]
    9 [ label="Dep: Dep F" ]
    10 [ label="Dep: Dep G" ]
    1 -> 0
    2 -> 0
    0 -> 4
    3 -> 4
    6 -> 5
    0 -> 6
    7 -> 5
    8 -> 5
    3 -> 8
    9 -> 10
    4 -> 9
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    #[test]
    fn test_display_var_c() {
        let graph = create_dependency_graph_var_c();

        let dot_string = format!("{}", graph.display());
        let compare_string = r#"digraph {
    0 [ label="Dep A" ]
    1 [ label="Dep B" ]
    2 [ label="Dep C" ]
    3 [ label="Dep D" ]
    4 [ label="Dep E" ]
    5 [ label="Dep F" ]
    6 [ label="Dep G" ]
    7 [ label="Dep H" ]
    8 [ label="Dep I" ]
    9 [ label="Dep I (item 0)" ]
    10 [ label="Dep J" ]
    11 [ label="Dep J (item 0)" ]
    1 -> 3
    2 -> 6
    4 -> 6
    5 -> 6
    0 -> 7
    1 -> 7
    4 -> 7
    5 -> 7
    9 -> 8
    5 -> 9
    11 -> 10
    0 -> 11
    1 -> 11
    8 -> 11
}
"#;

        assert_eq!(dot_string, compare_string);
    }

    fn test_filter(node_ref: DependencyAnyRef<TestDependency>) -> bool {
        match node_ref {
            DependencyAnyRef::Dependency(dep) => dep.inner().name() != "Dep B",
            _ => true,
        }
    }

    fn test_format_node(
        f: &mut fmt::Formatter<'_>,
        node_ref: DependencyAnyRef<TestDependency>,
    ) -> fmt::Result {
        match node_ref {
            DependencyAnyRef::Dependency(dep) => write!(f, "Dep: {}", dep.inner().name()),
            DependencyAnyRef::DependencyGroup(group) => {
                write!(f, "Group: {}", group.descriptor().name())
            }
            DependencyAnyRef::DependencyGroupItem(item) => write!(
                f,
                "Group Item {}: {}",
                item.index(),
                item.group().descriptor().name()
            ),
        }
    }
}
