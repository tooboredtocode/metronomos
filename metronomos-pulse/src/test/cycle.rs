use std::assert_matches;

use crate::PulseContainer;
use crate::builder::ProvideError;
use crate::test::macros::*;

make_sync_test_type!((TestType4) => TestType1);
make_sync_test_type!((TestType1) => TestType2);
make_sync_test_type!((TestType2) => TestType3);
make_sync_test_type!((TestType3) => TestType4);

#[tokio::test]
async fn test_cycle_detection() {
    let mut builder = PulseContainer::builder();

    assert_provide_sync!(builder, TestType1);
    assert_provide_sync!(builder, TestType2);
    assert_provide_sync!(builder, TestType3);

    let result = builder.provide(TestType4::new).map(|_| ());
    assert_matches!(result, Err(ProvideError::DependencyCycle))
}
