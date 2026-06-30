// SPDX-License-Identifier: MIT
//
// Extern surface for accepted_artifact_admission_decision Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/accepted_artifact_admission_decision_production.rs.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

#[path = "production_inner/accepted_artifact_admission_decision_production.rs"]
pub mod prod;

verus! {

pub use prod::{
    admission_decision, admission_decision_ok, SpecAdmissionError, SpecAdmissionOutcome,
    SpecArtifactEnvelopeError,
};

} // verus!