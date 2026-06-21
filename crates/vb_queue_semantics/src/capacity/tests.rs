use super::*;

// -- SHARED_QUEUE_CAPACITY_MAX -------------------------------------------------

#[test]
fn shared_queue_capacity_max_is_65_536() {
    assert_eq!(SHARED_QUEUE_CAPACITY_MAX, 65_536);
}

#[test]
fn shared_queue_capacity_max_is_pow_of_two() {
    // 65_536 = 2^16
    assert_eq!(SHARED_QUEUE_CAPACITY_MAX.count_ones(), 1);
    let pow: usize = 1 << 16;
    assert_eq!(SHARED_QUEUE_CAPACITY_MAX, pow);
}

#[test]
fn shared_queue_capacity_max_fits_usize() {
    let max: usize = SHARED_QUEUE_CAPACITY_MAX;
    assert_eq!(max, 65_536);
}

// -- CapacityRejection ---------------------------------------------------------

#[test]
fn capacity_rejection_zero_clone_eq() {
    let a = CapacityRejection::Zero;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn capacity_rejection_above_maximum_eq() {
    let a = CapacityRejection::AboveMaximum { maximum: 100 };
    let b = CapacityRejection::AboveMaximum { maximum: 100 };
    assert_eq!(a, b);
}

#[test]
fn capacity_rejection_above_maximum_ne_on_max() {
    let a = CapacityRejection::AboveMaximum { maximum: 100 };
    let b = CapacityRejection::AboveMaximum { maximum: 200 };
    assert_ne!(a, b);
}

#[test]
fn capacity_rejection_zero_ne_above_maximum() {
    assert_ne!(
        CapacityRejection::Zero,
        CapacityRejection::AboveMaximum { maximum: 10 }
    );
}

#[test]
fn capacity_rejection_debug_strings_are_distinct() {
    let z = format!("{:?}", CapacityRejection::Zero);
    let a = format!("{:?}", CapacityRejection::AboveMaximum { maximum: 7 });
    assert_ne!(z, a);
    assert!(z.contains("Zero"));
    assert!(a.contains("AboveMaximum"));
}

// -- validate_capacity ---------------------------------------------------------

#[test]
fn validate_capacity_zero_rejected() {
    let result = validate_capacity(0, 16);
    assert!(matches!(result, Err(CapacityRejection::Zero)));
}

#[test]
fn validate_capacity_one_accepted() {
    assert!(validate_capacity(1, 16).is_ok());
}

#[test]
fn validate_capacity_maximum_accepted() {
    assert!(validate_capacity(16, 16).is_ok());
}

#[test]
fn validate_capacity_above_maximum_rejected() {
    let result = validate_capacity(17, 16);
    assert!(matches!(
        result,
        Err(CapacityRejection::AboveMaximum { maximum: 16 })
    ));
}

#[test]
fn validate_capacity_above_maximum_zero_rejected_first() {
    // Zero is rejected before above-maximum check
    let result = validate_capacity(0, 16);
    assert!(matches!(result, Err(CapacityRejection::Zero)));
}

// -- helper_valid_capacity -----------------------------------------------------

#[test]
fn helper_valid_capacity_rejects_zero() {
    assert!(!helper_valid_capacity(0));
}

#[test]
fn helper_valid_capacity_accepts_one() {
    assert!(helper_valid_capacity(1));
}

#[test]
fn helper_valid_capacity_accepts_max() {
    assert!(helper_valid_capacity(SHARED_QUEUE_CAPACITY_MAX));
}

#[test]
fn helper_valid_capacity_rejects_above_max() {
    assert!(!helper_valid_capacity(SHARED_QUEUE_CAPACITY_MAX + 1));
}

#[test]
fn helper_valid_capacity_accepts_max_minus_one() {
    assert!(helper_valid_capacity(SHARED_QUEUE_CAPACITY_MAX - 1));
}
