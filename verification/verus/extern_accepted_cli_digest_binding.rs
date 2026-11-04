// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_cli_digest_binding Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/accepted_cli_digest_binding_production.rs.
// The production mirror contains the same type shapes and decision-fn
// semantics as the production source for digest-binding.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

#[path = "production_inner/accepted_cli_digest_binding_production.rs"]
pub mod prod;

verus! {

pub use prod::{
    production_artifact_digest_eq_header, production_digest_binding_total,
    production_proof_digest_eq_artifact, production_run_admission_artifact_digest,
    production_run_admission_new_digest, SpecAcceptedArtifact, SpecRunAdmission,
    SpecRuntimePolicy, SpecVerificationProof, SpecWorkflowDigest,
};

} // verus!