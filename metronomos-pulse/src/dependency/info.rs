use std::any::TypeId;
use std::hash::{Hash, Hasher};

use metronomos_loom::dependency::{DependencyKey, DependencyKeyKind, ShallowDependency};

use crate::value::PulseValue;

/// Lightweight, copyable identifier for a dependency type in the graph.
///
/// This struct carries three pieces of metadata about a dependency:
/// - [`type_id`](Self::type_id) — unique per-type discriminator
/// - [`kind`](Self::kind) — the [`DependencyKeyKind`] (e.g., function vs. value)
/// - [`type_name`](Self::type_name) — human-readable type name for error messages
///
/// `PulseDependencyInfo` implements equality based solely on `TypeId`, so two infos
/// compare equal when they refer to the same Rust type regardless of their kind or name.
#[derive(Debug, Copy, Clone)]
pub struct PulseDependencyInfo {
    /// Unique identifier for the stored type.
    pub type_id: TypeId,
    /// The dependency key kind (e.g., function-registered vs. value).
    pub kind: DependencyKeyKind,
    /// Human-readable name of the stored type.
    pub type_name: &'static str,
}

impl PulseDependencyInfo {
    /// Construct info for a given `PulseValue` type.
    ///
    /// Extracts the type ID, kind, and name from the type parameter `T`.
    pub fn new<T: PulseValue>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            kind: T::KIND,
            type_name: T::name(),
        }
    }

    /// Return a dependency key for the given value type.
    pub(crate) fn key_for<D, T>() -> DependencyKey<D>
    where
        D: ShallowDependency<Key = TypeId>,
        T: PulseValue,
    {
        DependencyKey::new(TypeId::of::<T>(), T::KIND)
    }
}

impl PartialEq for PulseDependencyInfo {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl PartialEq<TypeId> for PulseDependencyInfo {
    fn eq(&self, other: &TypeId) -> bool {
        self.type_id == *other
    }
}

impl PartialEq<PulseDependencyInfo> for TypeId {
    fn eq(&self, other: &PulseDependencyInfo) -> bool {
        *self == other.type_id
    }
}

impl Eq for PulseDependencyInfo {}

impl Hash for PulseDependencyInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

impl ShallowDependency for PulseDependencyInfo {
    type Key = TypeId;

    fn key(&self) -> DependencyKey<PulseDependencyInfo> {
        DependencyKey::new(self.type_id, self.kind)
    }

    fn name(&self) -> &str {
        self.type_name
    }
}
