use std::ops::Deref;

use crate::PulseContainer;
use crate::error::BuildDependencyError;
use crate::test::macros::{assert_builds, assert_contains, assert_provide_value};
use crate::value::{ArcValue, GroupValues, ValueGroupEntry};

fn make_string(
    values: GroupValues<ArcValue<u32>>,
) -> Result<ArcValue<String>, BuildDependencyError> {
    let mut result = String::new();
    let mut sorted_values: Vec<_> = values.iter().map(Deref::deref).copied().collect();
    sorted_values.sort();

    for value in sorted_values.iter() {
        result.push_str(&value.to_string());
        result.push(',');
    }
    if !result.is_empty() {
        result.pop(); // Remove the last comma
    }
    Ok(ArcValue::new(result))
}

#[tokio::test]
pub async fn test_groups() {
    let mut builder = PulseContainer::builder();

    for i in 0..25 {
        assert_provide_value!(builder, ValueGroupEntry(ArcValue::<u32>::new(i)));
    }
    builder
        .provide(make_string)
        .expect("Failed to provide make_string function");

    let container = assert_builds!(builder);

    let string = assert_contains!(container, ArcValue<String>);
    assert_eq!(
        string.deref(),
        "0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24"
    );
}
