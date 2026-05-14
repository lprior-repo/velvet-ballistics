#![forbid(unsafe_code)]
//! ID validation for schema validation.

#![allow(unreachable_pub)]
use crate::{ValidationError, ValidationResult};

pub fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId { id: id.to_owned() });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId { id: id.to_owned() });
    }
    if seen.contains(&id) {
        return Err(ValidationError::DuplicateId { id: id.to_owned() });
    }
    Ok(())
}

pub fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let Some(&byte) = id.as_bytes().first() else {
        return false;
    };
    if !byte.is_ascii_lowercase() {
        return false;
    }
    for byte in id.as_bytes() {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'_' {
            return false;
        }
    }
    true
}

pub fn is_reserved_id(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}

const RESERVED_IDS: &[&str] = &[
    "now",
    "random",
    "runtime",
    "null",
    "true",
    "false",
    "input",
    "inputs",
    "vars",
    "secrets",
    "steps",
    "error",
    "attempt",
    "total",
    "result",
    "when",
    "item",
    "do",
    "set",
    "choose",
    "for_each",
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

#[cfg(test)]
mod id_tests {
    use super::*;

    // -- is_valid_id: valid cases --

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

    // -- is_valid_id: invalid cases --

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

    // -- is_reserved_id --

    #[test]
    fn is_reserved_id_detects_runtime() {
        assert!(is_reserved_id("runtime"));
    }

    #[test]
    fn is_reserved_id_detects_now() {
        assert!(is_reserved_id("now"));
    }

    #[test]
    fn is_reserved_id_detects_null() {
        assert!(is_reserved_id("null"));
    }

    #[test]
    fn is_reserved_id_detects_true() {
        assert!(is_reserved_id("true"));
    }

    #[test]
    fn is_reserved_id_detects_false() {
        assert!(is_reserved_id("false"));
    }

    #[test]
    fn is_reserved_id_detects_input() {
        assert!(is_reserved_id("input"));
    }

    #[test]
    fn is_reserved_id_detects_inputs() {
        assert!(is_reserved_id("inputs"));
    }

    #[test]
    fn is_reserved_id_detects_vars() {
        assert!(is_reserved_id("vars"));
    }

    #[test]
    fn is_reserved_id_detects_secrets() {
        assert!(is_reserved_id("secrets"));
    }

    #[test]
    fn is_reserved_id_detects_steps() {
        assert!(is_reserved_id("steps"));
    }

    #[test]
    fn is_reserved_id_detects_error() {
        assert!(is_reserved_id("error"));
    }

    #[test]
    fn is_reserved_id_detects_attempt() {
        assert!(is_reserved_id("attempt"));
    }

    #[test]
    fn is_reserved_id_detects_total() {
        assert!(is_reserved_id("total"));
    }

    #[test]
    fn is_reserved_id_detects_result() {
        assert!(is_reserved_id("result"));
    }

    #[test]
    fn is_reserved_id_detects_when() {
        assert!(is_reserved_id("when"));
    }

    #[test]
    fn is_reserved_id_detects_item() {
        assert!(is_reserved_id("item"));
    }

    #[test]
    fn is_reserved_id_detects_do() {
        assert!(is_reserved_id("do"));
    }

    #[test]
    fn is_reserved_id_detects_set() {
        assert!(is_reserved_id("set"));
    }

    #[test]
    fn is_reserved_id_detects_finish() {
        assert!(is_reserved_id("finish"));
    }

    #[test]
    fn is_reserved_id_rejects_normal_id() {
        assert!(!is_reserved_id("my_step"));
    }

    #[test]
    fn is_reserved_id_rejects_empty() {
        assert!(!is_reserved_id(""));
    }

    // -- validate_single_id --

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

    // -- RESERVED_IDS completeness --

    #[test]
    fn reserved_ids_count() {
        assert_eq!(RESERVED_IDS.len(), 31);
    }
}
