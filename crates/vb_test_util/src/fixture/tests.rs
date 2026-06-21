use super::*;

#[test]
fn zero_capacity_rejected() {
    let result = FixtureCapacity::new(0);
    assert!(matches!(result, Err(TestSetupError::InvalidCapacity(_))));
}

#[test]
fn valid_capacity_accepted() {
    let result = match FixtureCapacity::new(100) {
        Ok(v) => v,
        Err(e) => panic!("FixtureCapacity::new(100) should succeed, got Err({e:?})"),
    };
    assert_eq!(result.value, 100);
}

#[test]
fn valid_capacity_minimal() {
    let result = match FixtureCapacity::new(1) {
        Ok(v) => v,
        Err(e) => panic!("FixtureCapacity::new(1) should succeed, got Err({e:?})"),
    };
    assert_eq!(result.value, 1);
}

#[test]
fn max_capacity_boundary() {
    let result = match FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY) {
        Ok(v) => v,
        Err(e) => {
            panic!("FixtureCapacity::new(MAX_CAPACITY) should succeed, got Err({e:?})")
        }
    };
    assert_eq!(result.value, FixtureCapacity::MAX_CAPACITY);
}

#[test]
fn over_max_capacity_rejected() {
    let result = FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY + 1);
    assert!(matches!(result, Err(TestSetupError::InvalidCapacity(_))));
}

#[test]
fn over_max_capacity_error_message_contains_capacity() {
    let err = match FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY + 1) {
        Err(e) => e,
        Ok(_) => panic!("should have returned Err"),
    };
    let msg = format!("{err}");
    assert!(
        msg.contains(&FixtureCapacity::MAX_CAPACITY.to_string()),
        "error message must contain the exceeding capacity value"
    );
}
