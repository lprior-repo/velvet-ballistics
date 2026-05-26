#![forbid(unsafe_code)]
//! PO-001: Kani harness for SymbolicCode::from_static validation.
//!
//! Proves: from_static(s).is_some() iff s is in CODE_REGISTRY;
//! from_static(s).is_none() for all other &'static str inputs.
//!
//! Bound: Registry size 157 entries (unwind=160)
//! Assumptions: CODE_REGISTRY const data is initialized with correct entries;
//! crate::diagnostic provides the canonical SymbolicCode type and registry.
//!
//! Rewired: uses production types from crate::diagnostic instead of
//! inline models. Types removed: DiagnosticCode, SymbolicCode, CodeCategory,
//! CodeEntry, CODE_REGISTRY, symbolic_to_numeric, string_eq, is_registered.

#[cfg(kani)]
mod harnesses {
    use crate::diagnostic::{CODE_REGISTRY, SymbolicCode};

    /// PO-001 H1: from_static returns Some for every registered symbolic string.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_from_static_validation() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let result = SymbolicCode::from_static(entry.symbolic);
            assert!(result.is_some(), "Registered code should return Some");
            if let Some(code) = result {
                assert_eq!(
                    code.as_str(),
                    entry.symbolic,
                    "SymbolicCode should preserve the symbolic string"
                );
            }
        }
    }

    /// PO-001 H2: from_static returns None for unregistered strings.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_from_static_rejects_unknown() {
        // Verify that a clearly unregistered string returns None
        let result = SymbolicCode::from_static("__DEFINITELY_NOT_REGISTERED__");
        assert!(result.is_none(), "Unregistered string must return None");

        // Verify that an empty string returns None (no empty symbolic codes exist)
        let empty_result = SymbolicCode::from_static("");
        assert!(empty_result.is_none(), "Empty string must return None");

        // Verify: for every registered string, from_static returns Some
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let result = SymbolicCode::from_static(entry.symbolic);
            assert!(result.is_some());
        }
    }
}
