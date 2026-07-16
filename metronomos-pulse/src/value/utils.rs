use metronomos_loom::dependency::DependencyItem;

use crate::container::PulseContext;
use crate::dependency::PulseDependencyInfo;
use crate::value::FromPulseValue;

pub(crate) fn from_context<V: FromPulseValue>(context: PulseContext<'_>) -> Option<V> {
    V::from_value(context.get_value::<V::Value>())
}

pub(crate) fn dependency_info<V: FromPulseValue>() -> Option<DependencyItem<PulseDependencyInfo>> {
    if V::REQUIRED {
        Some(DependencyItem::required(
            PulseDependencyInfo::new::<V::Value>(),
        ))
    } else {
        Some(DependencyItem::optional(
            PulseDependencyInfo::new::<V::Value>(),
        ))
    }
}
