#![forbid(unsafe_code)]
//! PO-012: Kani harness for DiagnosticCode::symbolic_code reverse lookup.
//!
//! Proves: (1) for each numeric code in registry, DiagnosticCode(code).symbolic_code()
//! == Some(expected SymbolicCode); (2) for u16 values outside registry, returns None.
//!
//! Bound: ~90 registry entries (unwind=100), ~1000 u16 samples for None case

use super::kani_symbolic_code_validation::{CODE_REGISTRY, DiagnosticCode, SymbolicCode};

/// Reverse lookup: numeric → symbolic name.
const fn numeric_to_symbolic(numeric: u16) -> Option<&'static str> {
    let mut i = 0;
    while i < CODE_REGISTRY.len() {
        if CODE_REGISTRY[i].numeric == numeric {
            return Some(CODE_REGISTRY[i].symbolic);
        }
        i += 1;
    }
    None
}

impl DiagnosticCode {
    /// Reverse lookup from numeric to symbolic code.
    #[must_use]
    pub fn symbolic_code(self) -> Option<SymbolicCode> {
        match numeric_to_symbolic(self.code()) {
            Some(s) => Some(SymbolicCode(s)),
            None => None,
        }
    }
}

/// Build a set of all registered numeric codes for fast membership testing.
fn is_registered_numeric(code: u16) -> bool {
    numeric_to_symbolic(code).is_some()
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-012 H1: For each registry numeric code, symbolic_code() returns
    /// the matching SymbolicCode.
    #[kani::proof]
    #[kani::unwind(100)]
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
                    sym.as_str(), entry.symbolic,
                    "symbolic_code() must return the matching SymbolicCode"
                );
            }
        }
    }

    /// PO-012 H2: For u16 values outside the registry, symbolic_code() returns None.
    #[kani::proof]
    #[kani::unwind(1000)]
    fn kani_reverse_lookup_returns_none_outside() {
        // Test edge values: 0, gaps between known ranges, and max value
        let test_values: [u16; 20] = [
            0x0000, 0x0001, 0x0100, 0x010C, 0x01FF,
            0x0200, 0x0205, 0x02FF,
            0x0300, 0x030A, 0x03FF,
            0x0400, 0x040D, 0x04FF,
            0x0500, 0x0514, 0x05FF,
            0x0600, 0x0604,
            0xFFFF,
        ];
        for code in test_values.iter() {
            if !is_registered_numeric(*code) {
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
}
