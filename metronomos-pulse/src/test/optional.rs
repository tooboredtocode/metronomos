use std::collections::HashSet;

use crate::PulseContainer;
use crate::dependency::PulseDependencyInfo;
use crate::error::PulseError;
use crate::test::macros::*;

// Test values
make_test_value!(TestValue);
make_test_value!(TestValue2);
make_test_value!(OptionalTestValue);

make_sync_test_type!((value: TestValue, optional_value: Option<OptionalTestValue>) => TestSyncType);
make_sync_test_type!((optional_value: OptionalTestValue) => TestSyncType2);
make_async_test_type!((value: TestValue, optional_value: Option<OptionalTestValue>) => TestAsyncType);
make_async_test_type!((optional_value: OptionalTestValue) => TestAsyncType2);

#[tokio::test]
async fn test_sync_optional_missing() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_sync!(builder, TestSyncType);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue);
    assert_not_contains!(container, OptionalTestValue);

    let sync_type = assert_contains!(container, TestSyncType);
    assert!(
        sync_type.optional_value.is_none(),
        "Expected optional_value to be None when OptionalTestValue is not provided"
    );
}

#[tokio::test]
async fn test_sync_optional_present() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_value!(builder, OptionalTestValue);
    assert_provide_sync!(builder, TestSyncType);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue);
    assert_contains!(container, OptionalTestValue);

    let sync_type = assert_contains!(container, TestSyncType);
    assert!(
        sync_type.optional_value.is_some(),
        "Expected optional_value to be Some when OptionalTestValue is provided"
    );
}

#[tokio::test]
async fn test_sync_optional_missing_and_required() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_sync!(builder, TestSyncType);
    assert_provide_sync!(builder, TestSyncType2);

    let error = assert_not_builds!(builder);
    let PulseError::MissingDependencies(error) = error else {
        panic!("Expected MissingDependencies error, got: {:?}", error);
    };

    assert_eq!(
        error.missing_dependencies(),
        &HashSet::from([PulseDependencyInfo::new::<OptionalTestValue>(),])
    );
}

#[tokio::test]
async fn test_sync_optional_present_and_required() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_value!(builder, OptionalTestValue);
    assert_provide_sync!(builder, TestSyncType);
    assert_provide_sync!(builder, TestSyncType2);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue);
    assert_contains!(container, OptionalTestValue);

    let sync_type = assert_contains!(container, TestSyncType);
    assert!(
        sync_type.optional_value.is_some(),
        "Expected optional_value to be Some when OptionalTestValue is provided"
    );
    assert_contains!(container, TestSyncType2);
}

#[tokio::test]
async fn test_async_optional_missing() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_value!(builder, TestValue2);
    assert_provide_async!(builder, TestAsyncType);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue);
    assert_not_contains!(container, OptionalTestValue);

    let async_type = assert_contains!(container, TestAsyncType);
    assert!(
        async_type.optional_value.is_none(),
        "Expected optional_value to be None when OptionalTestValue is not provided"
    );
}

#[tokio::test]
async fn test_async_optional_present() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_value!(builder, OptionalTestValue);
    assert_provide_async!(builder, TestAsyncType);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue);
    assert_contains!(container, OptionalTestValue);

    let async_type = assert_contains!(container, TestAsyncType);
    assert!(
        async_type.optional_value.is_some(),
        "Expected optional_value to be Some when OptionalTestValue is provided"
    );
}

#[tokio::test]
async fn test_async_optional_missing_and_required() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_async!(builder, TestAsyncType);
    assert_provide_async!(builder, TestAsyncType2);

    let error = assert_not_builds!(builder);
    let PulseError::MissingDependencies(error) = error else {
        panic!("Expected MissingDependencies error, got: {:?}", error);
    };

    assert_eq!(
        error.missing_dependencies(),
        &HashSet::from([PulseDependencyInfo::new::<OptionalTestValue>(),])
    );
}

#[tokio::test]
async fn test_async_optional_present_and_required() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue);
    assert_provide_value!(builder, OptionalTestValue);
    assert_provide_async!(builder, TestAsyncType);
    assert_provide_async!(builder, TestAsyncType2);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue);
    assert_contains!(container, OptionalTestValue);

    let async_type = assert_contains!(container, TestAsyncType);
    assert!(
        async_type.optional_value.is_some(),
        "Expected optional_value to be Some when OptionalTestValue is provided"
    );
    assert_contains!(container, TestAsyncType2);
}
