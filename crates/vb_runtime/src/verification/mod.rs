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
pub mod flux {
    pub mod vb_y9d3v_action_ticket_refinements;
}

#[cfg(all(flux, feature = "vb-mrwe6-flux-refinements"))]
pub mod mrwe6_flux {
    include!("flux/vb_mrwe6_atomic_index_refinements.rs");
    include!("flux/vb_mrwe6_completion_policy_refinements.rs");
    include!("flux/vb_mrwe6_queue_intent_refinements.rs");
}

// Flux refinement for `coalesce_window_ticks` and the post-condition of
// `Shard::flush_coalesce_buffer()` (vb-5iuag, FIX-FLUX-002-A). Bound to
// `shard/config.rs::is_valid_coalesce_window_ticks` and
// `shard/impl_parts/journal_helpers.rs::flush_coalesce_buffer`. See the file
// header for obligation details (OBL-CW-WIN-001 / OBL-CW-WIN-002).
#[cfg(all(flux, feature = "vb-coalesce-window-ticks-flux-refinements"))]
pub mod coalesce_window_ticks_flux {
    include!("flux/vb_coalesce_window_ticks_refinements.rs");
}

#[cfg(all(flux, feature = "vb-egysa-flux-refinements"))]
pub mod vb_egysa_flux {
    include!("flux/vb_egysa_runtime_facade_refinements.rs");
}

#[cfg(all(test, loom))]
pub mod loom {
    pub mod vb_mrwe6_atomic_index_loom;
    pub mod vb_mrwe6_completion_policy_loom;
    pub mod vb_mrwe6_duplicate_loom;
    pub mod vb_mrwe6_queue_intent_loom;
    pub mod vb_mrwe6_recovery_reliance_loom;
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
    // Timer seam proofs (vb-0l9k0) — sub-modules declared in vb-0l9k0/mod.rs
    #[path = "vb-0l9k0/mod.rs"]
    pub mod vb_0l9k0;

    // Attempt-fence kernel proofs (vb-y9d3v)
    #[path = "vb_y9d3v_action_fence.rs"]
    pub mod vb_y9d3v_action_fence;

    // Action completion kernel proofs (vb-kzz99)
    #[path = "vb_kzz99_action_completion.rs"]
    pub mod vb_kzz99_action_completion;

    // Action dispatch/receiver proofs (vb-rxru0)
    #[path = "vb_rxru0_action_verus.rs"]
    pub mod vb_rxru0_action_verus;

    // Runtime facade API proofs
    #[path = "runtime_facade_api.rs"]
    pub mod runtime_facade_api;
}

// Kani harnesses (compiled with cargo kani)
// HVR-PO-RUNTIME-001/HVR-PO-RUNTIME-002/HVR-PO-RUNTIME-006: feature-isolated vb-god2f harness group.
// ARCH-W0-02: feature-isolated orphan Kani harness groups.
#[cfg(kani)]
pub(crate) mod kani {
    #[cfg(feature = "vb-y9d3v-attempt-fence")]
    pub(crate) mod kani_attempt_fence_harnesses;

    #[cfg(feature = "vb-god2f-action-completion")]
    pub(crate) mod kani_vb_god2f_action_completion;

    #[cfg(feature = "kani-ask-payload-bounds")]
    pub(crate) mod kani_ask_payload_bounds;

    #[cfg(feature = "kani-cancel-kill-lattice")]
    pub(crate) mod kani_cancel_kill_lattice;

    #[cfg(feature = "kani-idempotency-tracker")]
    pub(crate) mod kani_idempotency_tracker;

    #[cfg(feature = "kani-submit-frame-release")]
    pub(crate) mod kani_submit_frame_release;

    #[cfg(feature = "kani-fzgdn-timer-harnesses")]
    pub(crate) mod vb_fzgdn_timer_harnesses;
}

// Flux refinement modules for vb-rxru0 (action enum invariants & dispatch_generic)
#[cfg(all(flux, feature = "vb-rxru0-flux-refinements"))]
pub mod rxru0_flux {
    pub mod vb_rxru0_dispatch_generic;
}
