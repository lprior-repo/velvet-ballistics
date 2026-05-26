#![forbid(unsafe_code)]
//! PO-012: Kani harness for DiagnosticCode::symbolic_code reverse lookup.
//!
//! Proves: (1) for each numeric code in production registry,
//! DiagnosticCode(code).symbolic_code() returns the expected SymbolicCode;
//! (2) for u16 values outside registry, returns None.
//!
//! Bound: 157 registry entries (unwind=160), ~20 u16 samples for None case
//!
//! Rewired: uses production SymbolicCode, DiagnosticCode, and CODE_REGISTRY
//! from crate::diagnostic. Does NOT redefine DiagnosticCode::symbolic_code()
//! — uses the production implementation directly.

use crate::diagnostic::{CODE_REGISTRY, DiagnosticCode};

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-012 H1: For each registry numeric code, symbolic_code() returns
    /// the matching SymbolicCode via the production implementation.
    #[kani::proof]
    #[kani::unwind(160)]
    fn kani_reverse_lookup() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            let dc = DiagnosticCode::new(entry.numeric);
            let result = dc.symbolic_code();
            assert!(
                result.is_some(),
                "Registered numeric code {:04X} must resolve via symbolic_code()",
                entry.numeric
            );
            if let Some(sym) = result {
                assert_eq!(
                    sym.as_str(),
                    entry.symbolic,
                    "symbolic_code() must return the matching SymbolicCode"
                );
            }
        }
    }

    /// PO-012 H2: For u16 values outside the registry, symbolic_code() returns None.
    #[kani::proof]
    #[kani::unwind(20)]
    fn kani_reverse_lookup_returns_none_outside() {
        let test_values: [u16; 20] = [
            0x0000, 0x0001, 0x0100, 0x010C, 0x01FF, 0x0200, 0x0205, 0x02FF, 0x0300, 0x030A, 0x03FF,
            0x0400, 0x040D, 0x04FF, 0x0500, 0x0514, 0x05FF, 0x0600, 0x0604, 0xFFFF,
        ];
        for code in test_values.iter() {
            let dc = DiagnosticCode::new(*code);
            let result = dc.symbolic_code();
            assert!(
                result.is_none(),
                "Unregistered numeric code {:04X} must return None",
                code
            );
        }
    }
}
