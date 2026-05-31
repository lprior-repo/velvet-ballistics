//! Verification harnesses for vb_runtime.
//!
//! Test-only verification path (proptest, non-kani test harnesses).
//! Kani, Flux, and Verus harnesses are gated behind appropriate cfg flags.

#[cfg(test)]
pub(crate) mod proptest;

// Flux refinement modules (compiled with flux-rs; extern specs)
// Gated behind a cfg to avoid compilation under normal rustc.
// Actual verification: `cargo flux -p vb_runtime`
#[cfg(feature = "vb-y9d3v-flux-refinements")]
pub mod flux {
    pub mod vb_y9d3v_action_ticket_refinements;
}

// Verus proof modules (compiled with verus toolchain)
// Gated behind a cfg to avoid compilation under normal rustc.
// Actual verification: `bash scripts/verify-verus.sh --target vb-y9d3v-action-fence`
// Note: Verus files use `verus!{}` macro and `vstd::prelude::*`.
// These files are compiled by the verus binary, not by rustc.
// The module is kept here for source discovery by verification scripts.
// To prevent rustc from trying to compile Verus syntax, we gate behind an
// impossible cfg (Verus uses its own compiler frontend).
#[cfg(verus)]
pub mod verus {
    pub mod vb_y9d3v_action_fence;
}

// Kani harnesses (compiled with cargo kani)
#[cfg(kani)]
pub(crate) mod kani {
    pub(crate) mod kani_attempt_fence_harnesses;
}
