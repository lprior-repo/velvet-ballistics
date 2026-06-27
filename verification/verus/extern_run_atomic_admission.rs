// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_run_atomic_admission Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/accepted_run_atomic_admission_production.rs.
// The production mirror contains the same type shapes and decision-fn
// semantics as the production source for strict-run admission
// (`vb_runtime::admission::REQUIRED_GATE_COUNT`,
// `vb_storage::admission::submit_artifact_with_contracts` strict
// branch).

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