#![forbid(unsafe_code)]
//! PO-002 + PO-010: Kani harnesses for CODE_REGISTRY bijection and non-zero invariants.
//!
//! PO-002 H1-H4: Registry is a bijection — no duplicate symbolic names, no duplicate
//! numeric codes, symbolic↔numeric round-trip identity.
//! PO-010: No diagnostic code has numeric value 0x0000.
//!
//! Bound: Registry size ~90 entries (unwind=200 for pairwise, unwind=100 for non-zero).

// Re-use the types declared in kani_symbolic_code_validation
use super::kani_symbolic_code_validation::CODE_REGISTRY;

/// Const helper: check if a u16 value appears exactly once in the registry's numeric fields.
const fn count_numeric(value: u16) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < CODE_REGISTRY.len() {
        if CODE_REGISTRY[i].numeric == value {
            count += 1;
        }
        i += 1;
    }
    count
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-002 H1: No duplicate symbolic names in CODE_REGISTRY.
    #[kani::proof]
    #[kani::unwind(200)]
    fn kani_registry_bijection_unique_symbolic() {
        for i in 0..CODE_REGISTRY.len() {
            for j in (i + 1)..CODE_REGISTRY.len() {
                kani::assert(
                    CODE_REGISTRY[i].symbolic != CODE_REGISTRY[j].symbolic,
                    "Duplicate symbolic name detected",
                );
            }
        }
    }

    /// PO-002 H2: No duplicate numeric codes in CODE_REGISTRY.
    #[kani::proof]
    #[kani::unwind(200)]
    fn kani_registry_bijection_unique_numeric() {
        for i in 0..CODE_REGISTRY.len() {
            for j in (i + 1)..CODE_REGISTRY.len() {
                kani::assert(
                    CODE_REGISTRY[i].numeric != CODE_REGISTRY[j].numeric,
                    "Duplicate numeric code detected",
                );
            }
        }
    }

    /// PO-002 H3: For every entry, if we look up by symbolic name, we get
    /// the correct numeric code, and vice versa (bijection).
    #[kani::proof]
    #[kani::unwind(200)]
    fn kani_registry_bijection_roundtrip_symbolic_to_numeric() {
        for i in 0..CODE_REGISTRY.len() {
            let entry = &CODE_REGISTRY[i];
            // The symbolic→numeric lookup should find this entry
            // This is verified by construction — the const function scans the registry.
            let found =
                super::super::kani_symbolic_code_validation::symbolic_to_numeric(entry.symbolic);
            kani::assert(
                found.is_some(),
                "Every registered symbolic name must resolve",
            );
            kani::assert_eq!(found, Some(entry.numeric), "Symbolic→numeric mismatch");

            // And the numeric→symbolic lookup should also find it
            let rev = super::super::kani_registry_bijection::count_numeric(entry.numeric);
            kani::assert_eq!(rev, 1, "Each numeric code must appear exactly once");
        }
    }

    /// PO-002 H4: Every numeric code in the registry maps to exactly one
    /// symbolic name (verified via the count_numeric invariant).
    #[kani::proof]
    #[kani::unwind(200)]
    fn kani_registry_bijection() {
        // Combined harness: verify uniqueness of symbolic and numeric,
        // and round-trip property
        kani_registry_bijection_unique_symbolic();
        kani_registry_bijection_unique_numeric();
        kani_registry_bijection_roundtrip_symbolic_to_numeric();
    }

    /// PO-010: No entry has numeric code 0x0000 (reserved sentinel).
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_registry_nonzero() {
        for i in 0..CODE_REGISTRY.len() {
            kani::assert(
                CODE_REGISTRY[i].numeric != 0,
                "No diagnostic code may have numeric value 0x0000",
            );
        }
    }
}
