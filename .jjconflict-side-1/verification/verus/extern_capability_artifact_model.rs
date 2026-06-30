// SPDX-License-Identifier: MIT
//
// Extern surface for capability_artifact_model Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// PRODUCTION INCLUSION via #[path]:
// Direct `#[path]` inclusion of
// verification/verus/production_inner/capability_artifact_production.rs.
// The production mirror contains the same type shapes and decision-fn
// semantics as the production source for capability admission.
// Any drift in field names, discriminant sets, or fn signatures in the
// production source breaks the production_inner mirror at compile time.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of
// verification/verus/production_inner/capability_artifact_production.rs.
// Declared at the crate root (NOT inside `verus!`) so the production
// types and exec fns are visible to the spec file via `prod::*`
// re-exports.
#[path = "production_inner/capability_artifact_production.rs"]
pub mod prod;

verus! {

// Re-export the production types and decision fn so the spec file can
// reference them.
pub use prod::{
    admit_artifact_run_with_certificate_floor, SpecAdmitError, SpecCapability, SpecRuntimePolicy,
};

} // verus!