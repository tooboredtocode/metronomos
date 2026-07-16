use metronomos_loom::dependency::{DependencyItem, DependencyKey, IntoDependency};

use crate::dependency::{PulseDependency, PulseDependencyInfo};
use crate::value::{ArcValue, PulseValue};

/// Wraps a concrete value as a dependency that satisfies `IntoDependency`.
pub(crate) struct ValueDependency<V>(pub V);

impl<V> IntoDependency<PulseDependency> for ValueDependency<V>
where
    V: PulseValue,
{
    fn key(&self) -> DependencyKey<PulseDependency> {
        PulseDependencyInfo::key_for::<_, V>()
    }

    fn shallow(&self) -> PulseDependencyInfo {
        PulseDependencyInfo::new::<V>()
    }

    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<PulseDependencyInfo>> {
        std::iter::empty()
    }

    fn into_dependency(self) -> PulseDependency {
        PulseDependency::new_value(self.0)
    }
}

/// Wraps a thread-safe value as an [`ArcValue`] dependency.
pub(crate) struct ArcValueDependency<T>(pub T)
where
    T: Send + Sync + 'static;

impl<T> IntoDependency<PulseDependency> for ArcValueDependency<T>
where
    T: Send + Sync + 'static,
{
    fn key(&self) -> DependencyKey<PulseDependency> {
        PulseDependencyInfo::key_for::<_, ArcValue<T>>()
    }

    fn shallow(&self) -> PulseDependencyInfo {
        PulseDependencyInfo::new::<ArcValue<T>>()
    }

    fn dependencies(&self) -> impl Iterator<Item = DependencyItem<PulseDependencyInfo>> {
        std::iter::empty()
    }

    fn into_dependency(self) -> PulseDependency {
        let arc_value = ArcValue::new(self.0);
        PulseDependency::new_value(arc_value)
    }
}
