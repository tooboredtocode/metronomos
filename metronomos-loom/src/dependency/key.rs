use std::fmt;
use std::hash::{Hash, Hasher};

use crate::dependency::ShallowDependency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyKeyKind {
    /// A unique key for a dependency. This is used to identify the dependency in the dependency graph.
    Unique,
    /// A group key for a dependency. This is used to identify a group of dependencies in the
    /// dependency graph. Multiple dependencies can have the same group key.
    Group,
}

/// A key for a dependency. This is used to identify the dependency in the dependency graph.
pub struct DependencyKey<D: ShallowDependency + ?Sized> {
    pub key: D::Key,
    pub kind: DependencyKeyKind,
}

impl<D: ShallowDependency + ?Sized> DependencyKey<D> {
    /// Creates a new dependency key with the specified kind.
    pub fn new(key: D::Key, kind: DependencyKeyKind) -> Self {
        Self { key, kind }
    }

    /// Creates a new unique dependency key.
    pub fn new_unique(key: D::Key) -> Self {
        Self {
            key,
            kind: DependencyKeyKind::Unique,
        }
    }

    /// Creates a new dependency group key.
    pub fn new_group(key: D::Key) -> Self {
        Self {
            key,
            kind: DependencyKeyKind::Group,
        }
    }

    /// Converts this dependency key to a different dependency type with the same key type.
    pub fn cast<U>(self) -> DependencyKey<U>
    where
        U: ShallowDependency<Key = D::Key> + ?Sized,
    {
        DependencyKey {
            key: self.key,
            kind: self.kind,
        }
    }
}

impl<D: ShallowDependency + ?Sized> fmt::Debug for DependencyKey<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependencyKey")
            .field("key", &self.key)
            .field("kind", &self.kind)
            .finish()
    }
}

impl<D: ShallowDependency + ?Sized> Clone for DependencyKey<D> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            kind: self.kind,
        }
    }
}

impl<D: ShallowDependency + ?Sized> Copy for DependencyKey<D> where D::Key: Copy {}

impl<D: ShallowDependency + ?Sized> PartialEq for DependencyKey<D> {
    fn eq(&self, other: &Self) -> bool {
        // Compare the keys for equality. The kind is not considered for equality.
        self.key == other.key
    }
}

impl<D: ShallowDependency + ?Sized> Eq for DependencyKey<D> {}

impl<D: ShallowDependency + ?Sized> Hash for DependencyKey<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the key. The kind is not considered for hashing.
        self.key.hash(state);
    }
}
