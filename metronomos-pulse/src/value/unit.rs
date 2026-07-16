use metronomos_loom::dependency::DependencyKeyKind;

use crate::container::PulseContext;
use crate::value::{PulseValue, PulseValueSealed};

impl PulseValueSealed for () {}
impl PulseValue for () {
    const KIND: DependencyKeyKind = DependencyKeyKind::Group;
    const IS_FINALIZER: bool = true;
    type StorageType = ();
    type GetValueType<'a> = usize;

    fn name() -> &'static str {
        "()"
    }

    fn map_to_storage_type(value: Self) -> Self::StorageType {
        value
    }

    fn get_value_type(context: PulseContext<'_>) -> Self::GetValueType<'_> {
        context.inner_get_value_group::<()>().len()
    }
}
