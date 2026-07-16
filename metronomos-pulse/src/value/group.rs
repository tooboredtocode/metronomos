use std::fmt;
use std::ops::Deref;

use metronomos_loom::dependency::DependencyKeyKind;

use crate::container::PulseContext;
use crate::value::{FromPulseValue, PulseValue, PulseValueSealed};

/// A single entry in a dependency group.
///
/// This type wraps a value of type `V` that belongs to a group dependency. Use it in an
/// [`FnDependency`](crate::dependency::FnDependency) or [`AsyncFnDependency`](crate::dependency::AsyncFnDependency)
/// to add a value to a group dependency.
#[derive(Debug, Clone, Copy)]
pub struct ValueGroupEntry<V: PulseValue>(pub V);

/// A collection of values from a group dependency.
///
/// When multiple instances of the same dependency type are registered using the
/// [`ValueGroupEntry`] type, they can be accessed using this type.
#[derive(Clone)]
pub struct GroupValues<V: PulseValue>(Box<[V]>);

impl<V: PulseValue> GroupValues<V> {
    fn new(values: &[V]) -> Self {
        Self(Box::from(values))
    }
}

impl<V: PulseValue<StorageType = V>> PulseValueSealed for ValueGroupEntry<V> {}

impl<V> PulseValue for ValueGroupEntry<V>
where
    V: PulseValue<StorageType = V>,
{
    const KIND: DependencyKeyKind = DependencyKeyKind::Group;
    type StorageType = V;

    type GetValueType<'a> = &'a [V];

    #[inline]
    fn name() -> &'static str {
        V::name()
    }

    fn map_to_storage_type(value: Self) -> Self::StorageType {
        value.0
    }

    fn get_value_type(context: PulseContext<'_>) -> Self::GetValueType<'_> {
        context.inner_get_value_group::<V>()
    }
}

impl<V> FromPulseValue for GroupValues<V>
where
    V: PulseValue,
    ValueGroupEntry<V>: for<'a> PulseValue<GetValueType<'a> = &'a [V]>,
{
    type Value = ValueGroupEntry<V>;

    fn from_value(values: &[V]) -> Option<Self> {
        Some(GroupValues::new(values))
    }
}

impl<V: PulseValue> Deref for ValueGroupEntry<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: PulseValue> Deref for GroupValues<V> {
    type Target = [V];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: PulseValue> IntoIterator for GroupValues<V> {
    type Item = V;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<V: PulseValue + fmt::Debug> fmt::Debug for GroupValues<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debg = f.debug_tuple("GroupValues");
        for value in self.0.iter() {
            debg.field(value);
        }
        debg.finish()
    }
}
