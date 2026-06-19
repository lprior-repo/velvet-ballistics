//! Verification artifacts for vb_core.

// HVR-PO-CORE-001/HVR-PO-CORE-003/HVR-PO-CORE-004: exclude legacy verification Kani modules from vb-god2f lane discovery.
#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub(crate) mod kani;

// Flux refinement modules (compiled with flux-rs; extern specs)
#[cfg(all(flux, feature = "vb-rxru0-flux-refinements"))]
pub mod flux {
    pub mod vb_rxru0_action_enums;
}

// Proptest modules (compiled with cargo test)
#[cfg(test)]
pub(crate) mod proptest {
    pub(crate) mod vb_rxru0_action_properties;
}

// Verus proof modules (compiled with verus toolchain).
// Note: production-binding proofs live in-frame in frame.rs and action.rs
// via #[cfg(verus)] verus! { reveal_with_fuel(...) blocks.
#[cfg(verus)]
pub mod verus {
    // reserved: add new verus modules here
}
