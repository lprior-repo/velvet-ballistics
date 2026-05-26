#![forbid(unsafe_code)]
//! PO-013: Kani harness for symbolic_code determinism and panic-freedom.
//!
//! Proves: Calling DiagnosticCode::symbolic_code() twice returns the same
//! result; never panics; no I/O side effects. Uses the production
//! implementation from crate::diagnostic.
//!
//! Bound: Arbitrary u16 for DiagnosticCode (kani::any);
//! For DiagnosticCode::symbolic_code(): 157-entry registry scan (unwind=160).
//!
//! Rewired: uses production SymbolicCode, DiagnosticCode, and CODE_REGISTRY
//! from crate::diagnostic. Does NOT redefine DiagnosticCode::symbolic_code().

use crate::diagnostic::{CODE_REGISTRY, DiagnosticCode};

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-013 H1: For arbitrary DiagnosticCode values (kani::any), calling
    /// symbolic_code() twice returns the same result; never panics.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_symbolic_code_determinism() {
        let raw: u16 = kani::any();
        let dc = DiagnosticCode::new(raw);

        let result1 = dc.symbolic_code();
        let result2 = dc.symbolic_code();

        assert_eq!(
            result1, result2,
            "symbolic_code() must be deterministic: two calls must return same result"
        );

        if let Some(sym) = result1 {
            let found_in_registry = CODE_REGISTRY.iter().any(|e| e.symbolic == sym.as_str());
            assert!(
                found_in_registry,
                "Returned SymbolicCode must be in the registry"
            );

            let entry_numeric = CODE_REGISTRY
                .iter()
                .find(|e| e.symbolic == sym.as_str())
                .map(|e| e.numeric);
            assert_eq!(
                entry_numeric,
                Some(raw),
                "Registry numeric must match input"
            );
        }
    }

    /// Verify that symbolic_code() is consistent across all registry entries.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_symbolic_code_consistency() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let dc = DiagnosticCode::new(entry.numeric);
            let result = dc.symbolic_code();
            assert!(result.is_some(), "Registered code must resolve");
            let result2 = dc.symbolic_code();
            assert_eq!(result, result2, "Must be consistent");
        }
    }
}
