// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_run_atomic_admission Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/accepted_run_atomic_admission_production.rs.
// The production mirror contains the same type shapes and decision-fn
// semantics as the production source for accepted-run admission:
// the prepared-proof gate-count decision
// (`vb_storage::admission::submit_artifact_with_contracts` at
// crates/vb_storage/src/admission.rs:354-374 performs no gate/flag
// validation; the prepared proof carries gate_count 0 per
// `verification_proof_core` at crates/vb_storage/src/admission.rs:113-128,
// with only bounded_claimed and idempotency_verified_claimed set by
// `prepare_with_profile` at
// crates/vb_storage/src/admission/verification.rs:26-52).

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

#[path = "production_inner/accepted_run_atomic_admission_production.rs"]
pub mod prod;

verus! {

// Re-export the production types and decision fns so the spec file can
// reference them.
pub use prod::{
    all_required_gates_accepted, artifact_matches_header_and_source, bind_accepted_at_seq,
    is_strict_accepted_artifact_tag, required_index_preconditions, valid_commit_input,
    PayloadTag,
};

} // verus!