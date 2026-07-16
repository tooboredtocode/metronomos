use std::collections::{HashMap, HashSet, VecDeque};
use std::{fmt, mem};

use crate::dependency::Dependency;
use crate::graph::DependencyGraph;
use crate::graph_ref::{
    DepOrDepGroupItemRef, DepOrDepGroupRef, DependencyGroupItemRef, DependencyGroupRef,
    DependencyRef,
};
use crate::node_descriptors::RootNode;

pub(crate) struct AllDependenciesIter<'a, D: Dependency> {
    visited: HashSet<(D::Key, bool)>, // The bool indicates whether the key is a dependency (true) or a dependency group item (false)
    queue: VecDeque<DepOrDepGroupItemRef<'a, D>>,
}

impl<'a, D> AllDependenciesIter<'a, D>
where
    D: Dependency,
{
    pub(crate) fn new(start: DepOrDepGroupItemRef<'a, D>) -> Self {
        let mut res = Self {
            visited: HashSet::new(),
            queue: VecDeque::from([start]),
        };

        let _ = res.next(); // Skip the first element, as it is the starting dependency
        res
    }

    fn update_queue(&mut self, dep: DepOrDepGroupRef<'a, D>) {
        match dep {
            DepOrDepGroupRef::Dependency(dep) => {
                let key = (dep.key.clone(), true);
                if self.visited.contains(&key) {
                    return;
                }
                self.visited.insert(key);
                self.queue.push_back(DepOrDepGroupItemRef::Dependency(dep));
            }
            DepOrDepGroupRef::DependencyGroup(group) => {
                let key = (group.key.clone(), false);
                if self.visited.contains(&key) {
                    return;
                }
                self.visited.insert(key);
                self.queue
                    .extend(group.items().map(DepOrDepGroupItemRef::DependencyGroupItem));
            }
        }
    }
}

impl<'a, D> Iterator for AllDependenciesIter<'a, D>
where
    D: Dependency,
{
    type Item = DepOrDepGroupItemRef<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.queue.pop_front()?;

        for dep in next.direct_dependencies() {
            self.update_queue(dep);
        }

        Some(next)
    }
}

pub(crate) struct AllDependantsIter<'a, D: Dependency> {
    // The root node struct already contains the information about whether it is a dependency
    // or a dependency group item.
    visited: HashSet<RootNode<D::Key>>,
    queue: VecDeque<DepOrDepGroupItemRef<'a, D>>,
}

impl<'a, D> AllDependantsIter<'a, D>
where
    D: Dependency,
{
    pub(crate) fn new(start: DepOrDepGroupItemRef<'a, D>) -> Self {
        let mut res = Self {
            visited: HashSet::new(),
            queue: VecDeque::from([start]),
        };

        let _ = res.next(); // Skip the first element, as it is the starting dependency
        res
    }

    pub(crate) fn new_from_group(start: DependencyGroupRef<'a, D>) -> Self {
        // We just return an arbitrary item from the group, since that will just call
        // the same dependants as the group itself.
        let start = start
            .items()
            .next()
            .expect("Dependency group should have at least one item");

        Self::new(DepOrDepGroupItemRef::DependencyGroupItem(start))
    }

    fn update_queue(&mut self, dep: DepOrDepGroupItemRef<'a, D>) {
        match dep {
            DepOrDepGroupItemRef::Dependency(dep) => {
                let root_node = RootNode::new_dependency(dep.key.clone());
                if self.visited.contains(&root_node) {
                    return;
                }
                self.visited.insert(root_node);
                self.queue.push_back(DepOrDepGroupItemRef::Dependency(dep));
            }
            DepOrDepGroupItemRef::DependencyGroupItem(item) => {
                let root_node = RootNode::new_dependency_group_item(item.key.clone(), item.idx);
                if self.visited.contains(&root_node) {
                    return;
                }
                self.visited.insert(root_node);
                self.queue
                    .push_back(DepOrDepGroupItemRef::DependencyGroupItem(item));
            }
        }
    }
}

impl<'a, D> Iterator for AllDependantsIter<'a, D>
where
    D: Dependency,
{
    type Item = DepOrDepGroupItemRef<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.queue.pop_front()?;

        for dep in next.direct_dependants() {
            self.update_queue(dep);
        }

        Some(next)
    }
}

struct GroupMissingItems(Box<[u8]>);

impl GroupMissingItems {
    fn new(group_items: usize) -> Self {
        let mut bytes = vec![u8::MAX; group_items.div_ceil(8)];
        let remainder = group_items % 8;
        if remainder != 0 {
            let last_byte_index = bytes.len() - 1;
            let mask = (1u8 << remainder) - 1;
            bytes[last_byte_index] &= mask;
        }

        Self(bytes.into_boxed_slice())
    }

    fn is_item_yielded(&self, item_idx: usize) -> bool {
        let byte_index = item_idx / 8;
        let bit_index = item_idx % 8;
        (self.0[byte_index] & (1 << bit_index)) == 0
    }

    fn mark_item_as_yielded(&mut self, item_idx: usize) {
        let byte_index = item_idx / 8;
        let bit_index = item_idx % 8;
        self.0[byte_index] &= !(1 << bit_index);
    }

    fn has_missing_items(&self) -> bool {
        self.0.iter().any(|&byte| byte != 0)
    }
}

impl fmt::Debug for GroupMissingItems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GroupMissingItems(")?;
        for byte in self.0.iter().rev() {
            write!(f, "{:08b}", byte)?;
        }
        f.write_str(")")?;
        Ok(())
    }
}

pub(crate) struct InitChunksIter<'a, D: Dependency> {
    yielded: HashSet<D::Key>,
    yielded_group_items: HashMap<D::Key, GroupMissingItems>,
    next: Vec<DepOrDepGroupItemRef<'a, D>>,
}

impl<'a, D: Dependency> InitChunksIter<'a, D> {
    pub(crate) fn new(dep: &'a DependencyGraph<D>) -> Self {
        let mut res = Self {
            yielded: HashSet::new(),
            yielded_group_items: HashMap::new(),
            next: Vec::new(),
        };

        // Add all the root dependencies to the next queue, as they can be yielded first.
        for root_dep in dep.root_nodes() {
            res.add_to_yielded(&root_dep);
            res.next.push(root_dep);
        }

        res
    }

    fn add_to_yielded(&mut self, dep: &DepOrDepGroupItemRef<'a, D>) {
        match dep {
            DepOrDepGroupItemRef::Dependency(dep) => {
                self.yielded.insert(dep.key.clone());
            }
            DepOrDepGroupItemRef::DependencyGroupItem(item) => {
                let group_missing_items = self
                    .yielded_group_items
                    .entry(item.key.clone())
                    .or_insert_with(|| GroupMissingItems::new(item.group_entry.dependencies.len()));

                group_missing_items.mark_item_as_yielded(item.idx as usize);
                if !group_missing_items.has_missing_items() {
                    self.yielded.insert(item.key.clone());
                    self.yielded_group_items.remove(item.key);
                }
            }
        }
    }

    fn check_if_dependencies_yielded(
        &self,
        parents: impl Iterator<Item = DepOrDepGroupRef<'a, D>>,
    ) -> bool {
        for parent in parents {
            let key = match &parent {
                DepOrDepGroupRef::Dependency(dep) => dep.key,
                DepOrDepGroupRef::DependencyGroup(group) => group.key,
            };

            if !self.yielded.contains(key) {
                return false;
            }
        }
        true
    }

    fn dep_can_be_yielded(&self, dep: &DependencyRef<'a, D>) -> bool {
        // If the dependency has already been yielded, it cannot be yielded again.
        if self.yielded.contains(dep.key) {
            return false;
        }
        // Check if all the direct dependencies have been yielded. If any of them have not been yielded,
        // then this dependency cannot be yielded yet.
        self.check_if_dependencies_yielded(dep.direct_dependencies())
    }

    fn group_item_can_be_yielded(&self, item: &DependencyGroupItemRef<'a, D>) -> bool {
        // If the dependency group itself has already been yielded, all its items have been yielded,
        // so this item cannot be yielded again.
        if self.yielded.contains(item.key) {
            return false;
        }

        // Group has not been yielded yet, check if the group item has already been yielded. If it has, it cannot be yielded again.
        if let Some(group_missing_items) = self.yielded_group_items.get(item.key)
            && group_missing_items.is_item_yielded(item.idx as usize)
        {
            return false;
        }

        // Check if all the direct dependencies have been yielded. If any of them have not been yielded,
        // then this dependency group item cannot be yielded yet.
        self.check_if_dependencies_yielded(item.direct_dependencies())
    }

    /// Returns true if the given dependency can be yielded, meaning that all of its direct
    /// dependencies and thus all of its transitive dependencies have already been yielded.
    fn can_be_yielded(&self, dep: &DepOrDepGroupItemRef<'a, D>) -> bool {
        match dep {
            DepOrDepGroupItemRef::Dependency(dep) => self.dep_can_be_yielded(dep),
            DepOrDepGroupItemRef::DependencyGroupItem(item) => self.group_item_can_be_yielded(item),
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

        // Add all the direct dependants of the yielded dependencies to the next queue, if they can be yielded.
        result
            .iter()
            .flat_map(|dep| dep.direct_dependants())
            .for_each(|dep| {
                if self.can_be_yielded(&dep) {
                    self.add_to_yielded(&dep);
                    self.next.push(dep);
                }
            });

        Some(result)
    }
}
