use std::collections::HashSet;
use std::ops::Deref;

use crate::PulseContainer;
use crate::dependency::PulseDependencyInfo;
use crate::error::PulseError;
use crate::test::macros::*;

// Test values
make_test_value!(TestValue1);
make_test_value!(TestValue2);

// Optional dependencies
make_test_value!(TestOptionalValue1);
make_test_value!(TestOptionalValue2);

// Sync test types
make_sync_test_type!((TestSyncType4, TestAsyncType4) => TestSyncType1);
make_sync_test_type!((TestValue1, TestSyncType5) => TestSyncType2);
make_sync_test_type!(() => TestSyncType3);
make_sync_test_type!((TestAsyncType3, TestSyncType2, Option<TestOptionalValue1>) => TestSyncType4);
make_sync_test_type!((Option<TestOptionalValue2>, TestValue1) => TestSyncType5);

// Async test types
make_async_test_type!((TestAsyncType4, TestSyncType4) => TestAsyncType1);
make_async_test_type!((TestValue2, TestAsyncType5) => TestAsyncType2);
make_async_test_type!(() => TestAsyncType3);
make_async_test_type!((TestSyncType3, TestAsyncType2, Option<TestOptionalValue1>) => TestAsyncType4);
make_async_test_type!((Option<TestOptionalValue2>, TestValue2) => TestAsyncType5);

macro_rules! assert_lines_equal {
    ($left:expr, $right:expr) => {
        let left_lines: Vec<&str> = $left.lines().collect();
        let right_lines: Vec<&str> = $right.lines().collect();
        assert_eq!(
            left_lines.len(),
            right_lines.len(),
            "Expected both strings to have the same number of lines"
        );
        for (i, (left_line, right_line)) in left_lines.iter().zip(right_lines.iter()).enumerate() {
            assert_eq!(
                left_line,
                right_line,
                "Expected all lines to be equal, but the strings differ at line {}",
                i + 1
            );
        }
    };
}

#[tokio::test]
async fn integration_test_container() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue1);
    assert_provide_value!(builder, TestValue2);

    assert_provide_sync!(builder, TestSyncType1);
    assert_provide_sync!(builder, TestSyncType2);
    assert_provide_sync!(builder, TestSyncType3);
    assert_provide_sync!(builder, TestSyncType4);
    assert_provide_sync!(builder, TestSyncType5);

    assert_provide_async!(builder, TestAsyncType1);
    assert_provide_async!(builder, TestAsyncType2);
    assert_provide_async!(builder, TestAsyncType3);
    assert_provide_async!(builder, TestAsyncType4);
    assert_provide_async!(builder, TestAsyncType5);

    let container = assert_builds!(builder);

    assert_contains!(container, TestValue1);
    assert_contains!(container, TestValue2);

    assert_not_contains!(container, TestOptionalValue1);
    assert_not_contains!(container, TestOptionalValue2);

    assert_contains!(container, TestSyncType1);
    assert_contains!(container, TestSyncType2);
    assert_contains!(container, TestSyncType3);
    assert_contains!(container, TestSyncType4);
    assert_contains!(container, TestSyncType5);

    assert_contains!(container, TestAsyncType1);
    assert_contains!(container, TestAsyncType2);
    assert_contains!(container, TestAsyncType3);
    assert_contains!(container, TestAsyncType4);
    assert_contains!(container, TestAsyncType5);

    let compare_string = r#"digraph {
    0 [ label="TestValue1" ]
    1 [ label="TestValue2" ]
    2 [ label="TestSyncType1" ]
    3 [ label="TestSyncType4" ]
    4 [ label="TestAsyncType4" ]
    5 [ label="TestSyncType2" ]
    6 [ label="TestSyncType5" ]
    7 [ label="TestSyncType3" ]
    8 [ label="TestAsyncType3" ]
    9 [ label="TestAsyncType1" ]
    10 [ label="TestAsyncType2" ]
    11 [ label="TestAsyncType5" ]
    3 -> 2
    4 -> 2
    1 -> 11
    6 -> 5
    5 -> 3
    8 -> 3
    0 -> 6
    4 -> 9
    3 -> 9
    10 -> 4
    11 -> 10
    7 -> 4
}"#;
    let dot_string = container.context().dot_string();
    assert_lines_equal!(dot_string.deref(), compare_string);
}

#[tokio::test]
async fn integration_test_container_missing_deps() {
    let mut builder = PulseContainer::builder();

    assert_provide_value!(builder, TestValue1);
    assert_provide_value!(builder, TestValue2);

    assert_provide_sync!(builder, TestSyncType1);
    assert_provide_sync!(builder, TestSyncType2);
    assert_provide_sync!(builder, TestSyncType4);
    assert_provide_sync!(builder, TestSyncType5);

    assert_provide_async!(builder, TestAsyncType1);
    assert_provide_async!(builder, TestAsyncType2);
    assert_provide_async!(builder, TestAsyncType4);
    assert_provide_async!(builder, TestAsyncType5);

    let error = assert_not_builds!(builder);
    let PulseError::MissingDependencies(error) = error else {
        panic!("Expected MissingDependencies error, got: {:?}", error);
    };

    assert_eq!(
        error.missing_dependencies(),
        &HashSet::from([
            PulseDependencyInfo::new::<TestSyncType3>(),
            PulseDependencyInfo::new::<TestAsyncType3>(),
        ])
    );
}
