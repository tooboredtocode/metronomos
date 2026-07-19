use std::collections::{HashMap, HashSet};

use crate::DependencyGraph;
use crate::dependency::{Dependency, DependencyItem, DependencyKey, ShallowDependency};
use crate::graph_ref::{DepOrDepGroupItemRef, DepOrDepGroupRef};

#[derive(Debug, Copy, Clone)]
pub(crate) struct TestShallowDependency {
    name: &'static str,
    group: bool,
}

#[derive(Debug)]
pub(crate) struct TestDependency {
    name: &'static str,
    group: bool,
    dependencies: Vec<DependencyItem<TestShallowDependency>>,
}

impl ShallowDependency for TestShallowDependency {
    type Key = &'static str;

    fn key(&self) -> DependencyKey<Self> {
        if self.group {
            DependencyKey::new_group(self.name)
        } else {
            DependencyKey::new_unique(self.name)
        }
    }
    fn name(&self) -> &str {
        self.name
    }
}

impl ShallowDependency for TestDependency {
    type Key = &'static str;

    fn key(&self) -> DependencyKey<Self> {
        if self.group {
            DependencyKey::new_group(self.name)
        } else {
            DependencyKey::new_unique(self.name)
        }
    }
    fn name(&self) -> &str {
        self.name
    }
}

impl Dependency for TestDependency {
    type Shallow = TestShallowDependency;

    fn shallow(&self) -> Self::Shallow {
        TestShallowDependency {
            name: self.name,
            group: self.group,
        }
    }

    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<Self::Shallow>> {
        self.dependencies.iter().copied()
    }
}

impl TestShallowDependency {
    pub fn new(name: &'static str, group: bool) -> Self {
        Self { name, group }
    }
}

impl TestDependency {
    pub fn new(
        name: &'static str,
        group: bool,
        dependencies: impl Iterator<Item = (&'static str, bool, bool)>,
    ) -> Self {
        Self {
            name,
            group,
            dependencies: dependencies
                .map(|(dep_name, required, group)| {
                    if required {
                        DependencyItem::required(TestShallowDependency::new(dep_name, group))
                    } else {
                        DependencyItem::optional(TestShallowDependency::new(dep_name, group))
                    }
                })
                .collect(),
        }
    }

    pub fn new_root(name: &'static str, group: bool) -> Self {
        Self {
            name,
            group,
            dependencies: Vec::new(),
        }
    }
}

pub(crate) fn create_dependency_graph_var_a() -> DependencyGraph<TestDependency> {
    let mut builder = DependencyGraph::builder();

    builder
        .add_dependency(TestDependency::new_root("Dep A", false))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep B", false))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep C",
            false,
            [("Dep A", true, false), ("Dep B", true, false)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep D",
            false,
            [("Dep A", true, false)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep F",
            false,
            [("Dep B", true, false), ("Dep C", true, false)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep G",
            false,
            [("Dep A", true, false), ("Dep F", true, false)].into_iter(),
        ))
        .unwrap();

    builder.build().expect("Graph should be buildable")
}

pub(crate) fn create_dependency_graph_var_b() -> DependencyGraph<TestDependency> {
    let mut builder = DependencyGraph::builder();

    builder
        .add_dependency(TestDependency::new_root("Dep A", true))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep A", true))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep B", false))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep C",
            false,
            [("Dep A", true, true), ("Dep B", true, false)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep D",
            true,
            [("Dep A", true, true)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep D", true))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep D",
            true,
            [("Dep B", true, false)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep F",
            false,
            [("Dep B", true, false), ("Dep C", true, false)].into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep G",
            false,
            [("Dep A", true, true), ("Dep F", true, false)].into_iter(),
        ))
        .unwrap();

    builder.build().expect("Graph should be buildable")
}

pub(crate) fn create_dependency_graph_var_c() -> DependencyGraph<TestDependency> {
    let mut builder = DependencyGraph::builder();

    builder
        .add_dependency(TestDependency::new_root("Dep A", false))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep B", false))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep C", false))
        .unwrap();

    builder
        .add_dependency(TestDependency::new(
            "Dep D",
            false,
            [("Dep B", true, false)].into_iter(),
        ))
        .unwrap();

    builder
        .add_dependency(TestDependency::new_root("Dep E", false))
        .unwrap();
    builder
        .add_dependency(TestDependency::new_root("Dep F", false))
        .unwrap();

    builder
        .add_dependency(TestDependency::new(
            "Dep G",
            false,
            [
                ("Dep C", true, false),
                ("Dep E", true, false),
                ("Dep F", true, false),
            ]
            .into_iter(),
        ))
        .unwrap();
    builder
        .add_dependency(TestDependency::new(
            "Dep H",
            false,
            [
                ("Dep A", true, false),
                ("Dep B", true, false),
                ("Dep E", true, false),
                ("Dep F", true, false),
            ]
            .into_iter(),
        ))
        .unwrap();

    builder
        .add_dependency(TestDependency::new(
            "Dep I",
            true,
            [("Dep F", true, false)].into_iter(),
        ))
        .unwrap();

    builder
        .add_dependency(TestDependency::new(
            "Dep J",
            true,
            [("Dep A", true, false), ("Dep B", true, false), ("Dep I", true, true)].into_iter(),
        ))
        .unwrap();

    builder.build().expect("Graph should be buildable")
}

pub(crate) fn ref_name(dep_ref: DepOrDepGroupItemRef<'_, TestDependency>) -> &str {
    dep_ref.inner().name()
}

pub(crate) fn ref_name_with_group(dep_ref: DepOrDepGroupRef<'_, TestDependency>) -> &str {
    match dep_ref {
        DepOrDepGroupRef::Dependency(dep) => dep.inner().name(),
        DepOrDepGroupRef::DependencyGroup(dep_group) => dep_group.descriptor().name(),
    }
}

pub(crate) fn ref_iter_set<'a>(
    deps: impl Iterator<Item = DepOrDepGroupItemRef<'a, TestDependency>>,
) -> HashSet<&'a str> {
    deps.map(ref_name).collect::<HashSet<_>>()
}

pub(crate) fn count_ref_iter<'a>(
    deps: impl Iterator<Item = DepOrDepGroupItemRef<'a, TestDependency>>,
) -> HashMap<&'a str, usize> {
    let mut counts = HashMap::new();
    for dep in deps {
        let name = ref_name(dep);
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

pub(crate) fn ref_iter_set_with_group<'a>(
    deps: impl Iterator<Item = DepOrDepGroupRef<'a, TestDependency>>,
) -> HashSet<&'a str> {
    deps.map(ref_name_with_group).collect::<HashSet<_>>()
}
