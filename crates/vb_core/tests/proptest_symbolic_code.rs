#![forbid(unsafe_code)]
//! PO-016: proptest property test for SymbolicCode::from_static
//! registration and determinism.
//!
//! Strategy: generate arbitrary &'static str slices from the known
//! CODE_REGISTRY entries; verify that from_static returns Some for
//! registered strings and None for unregistered strings.
//!
//! Bound: 1000 test cases, string lengths up to 64 chars.

use proptest::prelude::*;
use std::str::FromStr;
use vb_core::diagnostic::SymbolicCode;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy that generates a known registered symbolic code string.
fn arb_registered_symbol() -> impl Strategy<Value = &'static str> {
    let registered: Vec<&'static str> =
        vb_core::diagnostic::CODE_REGISTRY.iter().map(|e| e.symbolic).collect();
    proptest::sample::select(registered)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

proptest! {
    /// from_static returns Some for every registered symbolic string.
    #[test]
    fn from_static_returns_some_for_registered(symbol in arb_registered_symbol()) {
        let result = SymbolicCode::from_static(symbol);
        prop_assert!(result.is_some(), "from_static must return Some for registered symbol");
    }

    /// from_str always returns Ok for registered symbolic names.
    #[test]
    fn from_str_for_registered_is_ok(symbol in arb_registered_symbol()) {
        let result = SymbolicCode::from_str(symbol);
        prop_assert!(result.is_ok(), "from_str must return Ok for registered symbol");
    }

    /// from_str returns Err for known-unregistered strings.
    #[test]
    fn from_str_rejects_bogus(s in "[^A-Z]{1,32}") {
        let result = SymbolicCode::from_str(&s);
        match result {
            Err(err) => {
                prop_assert_eq!(
                    err.name.as_ref(),
                    s.as_str(),
                    "error must carry the rejected name"
                );
            }
            Ok(_) => prop_assert!(false, "must not parse bogus string"),
        }
    }

    /// as_str round-trips: from_static(s).unwrap().as_str() == s.
    #[test]
    fn as_str_roundtrips(symbol in arb_registered_symbol()) {
        let code = SymbolicCode::from_static(symbol).expect("must be registered");
        prop_assert_eq!(code.as_str(), symbol, "as_str must return the original string");
    }

    /// numeric_code is always non-zero.
    #[test]
    fn numeric_code_is_always_nonzero(symbol in arb_registered_symbol()) {
        let code = SymbolicCode::from_static(symbol).expect("must be registered");
        let num = code.numeric_code().expect("registered codes must have numeric codes");
        prop_assert_ne!(num, 0, "numeric code must be non-zero");
    }

    /// as_diagnostic_code → from_str round-trip.
    #[test]
    fn diagnostic_code_roundtrips(symbol in arb_registered_symbol()) {
        let code = SymbolicCode::from_static(symbol).expect("must be registered");
        let dc = code.as_diagnostic_code().expect("registered codes must have diagnostic codes");
        let display = dc.to_string();
        let parsed = vb_core::diagnostic::DiagnosticCode::from_str(&display);
        prop_assert_eq!(parsed, Ok(dc),
            "DiagnosticCode must round-trip via Display→from_str");
    }

    /// Display produces the symbolic name, not E-format.
    #[test]
    fn display_is_symbolic_name(symbol in arb_registered_symbol()) {
        let code = SymbolicCode::from_static(symbol).expect("must be registered");
        let displayed = code.to_string();
        prop_assert_eq!(displayed, symbol,
            "Display must produce the symbolic name, not the E-format");
    }

    /// Hash and Eq are consistent.
    #[test]
    fn hash_and_eq_consistent(symbol in arb_registered_symbol()) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let a = SymbolicCode::from_static(symbol).expect("must be registered");
        let b = SymbolicCode::from_static(symbol).expect("must be registered");
        prop_assert_eq!(a, b, "same symbolic must be equal");

        let mut ha = DefaultHasher::new();
        a.hash(&mut ha);
        let mut hb = DefaultHasher::new();
        b.hash(&mut hb);
        prop_assert_eq!(ha.finish(), hb.finish(), "same symbolic must have same hash");
    }
}
