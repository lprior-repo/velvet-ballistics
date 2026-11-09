#![forbid(unsafe_code)]
//! `kani::Arbitrary` impls for the production diagnostic code types.
//!
//! These impls let symbolic harnesses generate production
//! `SymbolicCode` and `DiagnosticCode` values directly, so the
//! models are values of the production types rather than copies
//! (GOD RULE 2). The `SymbolicCode` generator selects a uniform
//! registry index under `kani::assume`, hence only yields
//! registered codes; the `DiagnosticCode` generator covers the
//! full `u16` range, registered or not.

use crate::diagnostic::{CODE_REGISTRY, DiagnosticCode, SymbolicCode};

impl kani::Arbitrary for DiagnosticCode {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for SymbolicCode {
    fn any() -> Self {
        let idx: usize = kani::any();
        kani::assume(idx < CODE_REGISTRY.len());
        let mut selected = SymbolicCode::INTERNAL_INVARIANT;
        for (i, entry) in CODE_REGISTRY.iter().enumerate() {
            if i == idx {
                if let Some(code) = SymbolicCode::from_static(entry.symbolic) {
                    selected = code;
                }
            }
        }
        selected
    }
}
