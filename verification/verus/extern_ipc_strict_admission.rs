// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for ipc_strict_admission Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the `ipc_strict_admission.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/ipc_strict_admission_production.rs`
// via `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_ipc_strict_admission.rs"]`; this file uses
//     `#[path = "production_inner/ipc_strict_admission_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/ipc_strict_admission_production.rs` mirror
//     and the spec proofs that depend on it.
//
// The mirror at
// `production_inner/ipc_strict_admission_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_runtime/src/ipc_refinement.rs` (REFINE-IPC-001). The
// substitutions relative to direct production `#[path]` inclusion are
// documented in the mirror's header (in summary: the production
// source depends on `vb_core`, `crate::admission`, `crate::shard`,
// and serde derives that cannot be resolved in a single-file Verus
// unit under the "no installs / no production changes" constraints).
//
// ============================================================================
// BINDING LEDGER (mirrors production_inner/ipc_strict_admission_production.rs)
// ============================================================================
//   - `StrictAdmissionRefinement` (struct)             <- crates/vb_runtime/src/ipc_refinement.rs:21-29
//   - `is_refined`                                     <- crates/vb_runtime/src/ipc_refinement.rs:34-36
//   - `evidence_complete_projection`                  <- derived projection
//   - `strict_admission_refinement`                   <- crates/vb_runtime/src/ipc_refinement.rs:123-134
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`ipc_strict_admission.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/ipc_strict_admission_production.rs` (NOT the
// actual production source). The mirror is a hand-written structural
// copy of `crates/vb_runtime/src/ipc_refinement.rs` with documented
// substitutions (extern-crate imports stripped, method bodies
// replaced by `#[verifier::external]` wrappers). Any drift in field
// NAME, discriminant shape, or method signature breaks the
// verification build.
#[path = "production_inner/ipc_strict_admission_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;
