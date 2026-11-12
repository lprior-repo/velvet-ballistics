//! Verification harnesses for vb_runtime.
//!
//! Test-only verification path (proptest, non-kani test harnesses).
//! Kani, Flux, and Verus harnesses are gated behind appropriate cfg flags.

#[cfg(test)]
pub(crate) mod proptest;

// Flux refinement modules (compiled with flux-rs; extern specs)
// Gated behind a cfg to avoid compilation under normal rustc.
// Actual verification: `cargo flux -p vb_runtime`
#[cfg(all(flux, feature = "vb-y9d3v-flux-refinements"))]
pub(crate) mod flux {
    pub(crate) mod vb_y9d3v_action_ticket_refinements;
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

// Kani harnesses (compiled with cargo kani).
// Module declarations live in `kani/mod.rs` so the file count can scale.
#[cfg(kani)]
pub(crate) mod kani {
    pub(crate) mod kani_ask_answer_lifecycle;
    pub(crate) mod kani_attempt_fence_harnesses;
    // PO-vb282my-RS-KANI-001..006 / vb-4969v: repaired resume-state
    // harnesses remain feature-isolated because they are resource-heavy and
    // use bounded symbolic workflow/run-frame generators.
    #[cfg(feature = "kani-vb-4969v-runtime-a3")]
    pub(crate) mod kani_resume_state_machine;
    // PO-vb282my-RS-KANI-006 / vb-4969v: minimal aggregate-only invariant
    // harness. This lane intentionally avoids WorkflowParts, CompiledWorkflow,
    // and RunFrame construction.
    #[cfg(feature = "kani-sxkz6-shard-for-run")]
    pub(crate) mod kani_sxkz6_shard_for_run;
    #[cfg(feature = "kani-vb-4969v-aggregate-invariant")]
    pub(crate) mod kani_vb4969v_aggregate_invariant;
}
