//! Property tests for SymbolicCode construction and identity.
//!
//! Compensates: BLOCKED PO-001 (kani_from_static_validation).
//! Invariant: from_static(s).is_some() iff s exists in CODE_REGISTRY.

// Test code uses `.expect("descriptive message")` to convert fallible
// public-API results into asserted values inside proptest harnesses.
// Per repository policy (AGENTS.md: "Tests must compile and run, but test
// clippy is not strict"), `clippy::expect_used` is allowed in this test
// target. All messages are descriptive and identify the specific proptest
// scenario that would fail.
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::diagnostic::{CODE_REGISTRY, SymbolicCode, numeric_to_symbolic, symbolic_to_numeric};

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_registered_str() -> impl Strategy<Value = &'static str> {
    let names: Vec<&'static str> = CODE_REGISTRY.iter().map(|e| e.symbolic).collect();
    proptest::sample::select(names)
}

fn arb_unregistered_ascii() -> impl Strategy<Value = String> {
    proptest::string::string_regex(r#"[A-Za-z_][A-Za-z0-9_]*"#)
        .expect("valid regex")
        .prop_filter("must not be registered", |s| {
            CODE_REGISTRY.iter().all(|e| e.symbolic != s.as_str())
        })
}

// ---------------------------------------------------------------------------
// SymbolicCode property tests
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn from_static_returns_some_and_matches_registry(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s);
        prop_assert!(code.is_some());
        let code = code.expect("registered code must be Some");
        prop_assert_eq!(code.as_str(), s);
        let expected_numeric = symbolic_to_numeric(s);
        prop_assert_eq!(code.numeric_code(), expected_numeric);
    }

    #[test]
    fn from_str_rejects_unregistered(s in arb_unregistered_ascii()) {
        let parsed: Result<SymbolicCode, _> = s.as_str().parse();
        prop_assert!(parsed.is_err());
    }

    #[test]
    fn from_str_rejects_whitespace_wrapped_registered(s in arb_registered_str()) {
        let wrapped = format!(" {s} ");
        let parsed: Result<SymbolicCode, _> = wrapped.as_str().parse();
        prop_assert!(parsed.is_err());
    }

    #[test]
    fn from_str_rejects_lowercase_registered(s in arb_registered_str()) {
        let lower: String = s.to_lowercase();
        let parsed: Result<SymbolicCode, _> = lower.as_str().parse();
        prop_assert!(parsed.is_err());
    }

    #[test]
    fn from_str_matches_from_static_for_registered(s in arb_registered_str()) {
        let parsed: Result<SymbolicCode, _> = s.parse();
        let constructed = SymbolicCode::from_static(s);
        prop_assert_eq!(parsed.ok(), constructed);
    }

    #[test]
    fn as_str_preserves_constructor_string(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s)
            .expect("from_static must return Some for registered code");
        prop_assert_eq!(code.as_str(), s);
    }

    #[test]
    fn numeric_code_matches_registry(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s)
            .expect("from_static must return Some for registered code");
        let expected = symbolic_to_numeric(s)
            .expect("symbolic_to_numeric must return Some for registered code");
        prop_assert_eq!(code.numeric_code(), Some(expected));
    }

    #[test]
    fn copy_preserves_identity(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s)
            .expect("from_static must return Some for registered code");
        let copy = code;
        prop_assert_eq!(code, copy);
        prop_assert_eq!(code.as_str(), copy.as_str());
    }

    #[test]
    fn as_diagnostic_code_matches_registry(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s)
            .expect("from_static must return Some for registered code");
        let expected_num = symbolic_to_numeric(s)
            .expect("symbolic_to_numeric must return Some for registered code");
        prop_assert_eq!(
            code.as_diagnostic_code().map(|c| c.code()),
            Some(expected_num),
        );
    }

    #[test]
    fn display_formats_as_symbolic_name_not_e_hex(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s)
            .expect("from_static must return Some for registered code");
        let display = code.to_string();
        let num = code.numeric_code().expect("registered code must have a numeric code");
        let expected_hex = format!("E{:04X}", num);
        prop_assert_eq!(&display, s);
        let is_e_hex = display == expected_hex;
        prop_assert!(!is_e_hex);
    }

    #[test]
    fn numeric_to_symbolic_returns_some_for_registered_codes(s in arb_registered_str()) {
        let code = SymbolicCode::from_static(s)
            .expect("from_static must return Some for registered code");
        let back = numeric_to_symbolic(code.numeric_code().expect("registered code must have a numeric code"));
        prop_assert!(back.is_some());
    }
}

// ---------------------------------------------------------------------------
// Strateless tests (no proptest macro needed)
// ---------------------------------------------------------------------------

#[test]
fn from_static_returns_none_for_empty_string() {
    assert!(SymbolicCode::from_static("").is_none());
}

#[test]
fn from_str_rejects_empty() {
    let parsed: Result<SymbolicCode, _> = "".parse();
    assert!(parsed.is_err());
}

#[test]
fn symbolic_code_is_send_and_sync() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<SymbolicCode>();
}
