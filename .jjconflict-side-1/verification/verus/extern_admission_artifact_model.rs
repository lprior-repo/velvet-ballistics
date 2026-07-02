// SPDX-License-Identifier: MIT
//
// Extern surface for admission_artifact_model Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
// This file binds the admission_artifact_model.rs Verus spec to the
// canonical artifact admission types and decision logic in
// `crates/vb_storage/src/admission.rs`. The binding is structural +
// contract via a verbatim production mirror at
// `verification/verus/production_inner/admission_artifact_production.rs`.
// Any drift in field names, discriminant sets, or fn signatures in
// the production source breaks the production_inner mirror at compile
// time.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production mirror bodies are NOT verified by Verus directly (the
// `#[path]`-included bodies are opaque to Verus because they live
// outside the `verus!` block; items declared outside `verus!` are
// external by default). The exec fns declared inside `verus!` are
// re-declared with `#[verifier::external]` so Verus skips their
// bodies, and the companion spec file attaches `assume_specification`
// contracts to those re-declarations.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of
// verification/verus/production_inner/admission_artifact_production.rs.
// Declared at the crate root (NOT inside `verus!` and NOT marked
// `#[verifier::external]`) so the production types and exec fns are
// visible to the spec file via `crate::production::prod::*` re-exports.
// The exec fn bodies are then re-declared inside `verus!` below with
// `#[verifier::external]` so Verus skips body verification; the
// companion spec file attaches `assume_specification` contracts.
#[path = "production_inner/admission_artifact_production.rs"]
pub mod prod;

verus! {

// ============================================================================
// Verus-visible re-exports of the production types and exec fns
// ============================================================================
//
// These are non-opaque re-exports of the production types and exec
// fns from `prod`. They are visible to Verus at the extern's crate
// root, and the spec file references them as `production::WorkflowDigest`,
// `production::is_strict_admission_valid`, etc.
pub use prod::{
    artifact_digest_bound, digest_eq, is_strict_admission_valid, AcceptedArtifact,
    VerificationProof, WorkflowDigest,
};

} // verus!