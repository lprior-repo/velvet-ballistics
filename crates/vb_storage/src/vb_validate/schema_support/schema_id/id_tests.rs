#![forbid(unsafe_code)]

use crate::vb_validate::*;

#[test]
fn is_valid_id_accepts_simple_lowercase() {
    assert!(is_valid_id("abc"));
}

#[test]
fn is_valid_id_accepts_with_underscores() {
    assert!(is_valid_id("step_one"));
}

#[test]
fn is_valid_id_accepts_with_digits() {
    assert!(is_valid_id("step_1"));
}

#[test]
fn is_valid_id_accepts_single_char() {
    assert!(is_valid_id("a"));
}

#[test]
fn is_valid_id_accepts_max_length() {
    let id = "a".repeat(64);
    assert!(is_valid_id(&id));
}

#[test]
fn is_valid_id_accepts_underscore_after_first() {
    assert!(is_valid_id("a_b"));
}

#[test]
fn is_valid_id_accepts_all_digits_after_first() {
    assert!(is_valid_id("a123"));
}

#[test]
fn is_valid_id_rejects_empty() {
    assert!(!is_valid_id(""));
}

#[test]
fn is_valid_id_rejects_too_long() {
    let id = "a".repeat(65);
    assert!(!is_valid_id(&id));
}

#[test]
fn is_valid_id_rejects_starts_with_digit() {
    assert!(!is_valid_id("1abc"));
}

#[test]
fn is_valid_id_rejects_uppercase() {
    assert!(!is_valid_id("Abc"));
}

#[test]
fn is_valid_id_rejects_hyphen() {
    assert!(!is_valid_id("a-b"));
}

#[test]
fn is_valid_id_rejects_space() {
    assert!(!is_valid_id("a b"));
}

#[test]
fn is_valid_id_rejects_dot() {
    assert!(!is_valid_id("a.b"));
}

#[test]
fn is_valid_id_rejects_underscore_first() {
    assert!(!is_valid_id("_abc"));
}

#[test]
fn reserved_ids_detect_known_words() {
    for reserved in [
        "runtime", "now", "null", "true", "false", "input", "inputs", "vars", "secrets", "steps",
        "error", "attempt", "total", "result", "when", "item", "do", "set", "finish",
    ] {
        assert!(is_reserved_id(reserved));
    }
}

#[test]
fn is_reserved_id_rejects_normal_id() {
    assert!(!is_reserved_id("my_step"));
}

#[test]
fn is_reserved_id_rejects_empty() {
    assert!(!is_reserved_id(""));
}

#[test]
fn validate_single_id_accepts_valid() {
    assert_eq!(validate_single_id("my_step", &[]), Ok(()));
}

#[test]
fn validate_single_id_rejects_invalid() {
    assert_eq!(
        validate_single_id("BAD", &[]),
        Err(ValidationError::InvalidId {
            id: "BAD".to_owned()
        })
    );
}

#[test]
fn validate_single_id_rejects_reserved() {
    assert_eq!(
        validate_single_id("runtime", &[]),
        Err(ValidationError::ReservedId {
            id: "runtime".to_owned()
        })
    );
}

#[test]
fn validate_single_id_rejects_duplicate() {
    assert_eq!(
        validate_single_id("step1", &["step1"]),
        Err(ValidationError::DuplicateId {
            id: "step1".to_owned()
        })
    );
}

#[test]
fn validate_single_id_accepts_with_different_seen() {
    assert_eq!(validate_single_id("step2", &["step1"]), Ok(()));
}

#[test]
fn validate_single_id_checks_invalid_before_reserved() {
    let result = validate_single_id("2runtime", &[]);
    assert!(matches!(result, Err(ValidationError::InvalidId { .. })));
}

#[test]
fn reserved_ids_count() {
    assert_eq!(RESERVED_IDS.len(), 31);
}
