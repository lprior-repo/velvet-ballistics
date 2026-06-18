#![forbid(unsafe_code)]
//! PO-013: Kani harness for symbolic_code determinism and panic-freedom.
//!
//! Proves: For any error value implementing HasSymbolicCode, calling
//! symbolic_code() twice returns the same SymbolicCode; never panics;
//! no I/O side effects.
//!
//! Bound: Each HasSymbolicCode implementor — arbitrary instance (kani::any);
//! For DiagnosticCode::symbolic_code(): 90-entry registry scan (unwind=100).

use super::kani_symbolic_code_validation::SymbolicCode;

/// Trait for types that carry a symbolic diagnostic code.
pub trait HasSymbolicCode {
    fn symbolic_code(&self) -> SymbolicCode;
}

#[cfg(kani)]
mod harnesses {
    use super::super::kani_symbolic_code_validation::{CODE_REGISTRY, DiagnosticCode};

    /// PO-013 H1: For arbitrary DiagnosticCode values (kani::any), calling
    /// symbolic_code() twice returns the same result; never panics.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_symbolic_code_determinism() {
        let raw: u16 = kani::any();
        let dc = DiagnosticCode::new(raw);

        let result1 = dc.symbolic_code();
        let result2 = dc.symbolic_code();

        kani::assert(
            result1 == result2,
            "symbolic_code() must be deterministic: two calls must return same result",
        );

        if let Some(sym) = result1 {
            let found_in_registry = CODE_REGISTRY.iter().any(|e| e.symbolic == sym.as_str());
            kani::assert(
                found_in_registry,
                "Returned SymbolicCode must be in the registry",
            );

            let entry_numeric = CODE_REGISTRY
                .iter()
                .find(|e| e.symbolic == sym.as_str())
                .map(|e| e.numeric);
            kani::assert(
                entry_numeric == Some(raw),
                "Registry numeric must match input",
            );
        }
    }

    /// Verify that symbolic_code() is consistent across all registry entries.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_symbolic_code_consistency() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let dc = DiagnosticCode::new(entry.numeric);
            let result = dc.symbolic_code();
            kani::assert(result.is_some(), "Registered code must resolve");
            let result2 = dc.symbolic_code();
            kani::assert(result == result2, "Must be consistent");
        }
    }
}
