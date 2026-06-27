// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for v1_primitive_lowering Verus spec
// (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the
// `v1_primitive_lowering.rs` Verus spec. It includes the in-tree
// production mirror at
// `verification/verus/production_inner/v1_primitive_lowering_production.rs`
// via `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_v1_primitive_lowering.rs"]`; this file uses
//     `#[path = "production_inner/v1_primitive_lowering_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/v1_primitive_lowering_production.rs` mirror and
//     the spec proofs that depend on it.
//
// The mirror at `production_inner/v1_primitive_lowering_production.rs`
// is a hand-written structural copy of the production surface in
// `crates/vb_compile/src/mod_compile_lowering/{part_05_ir,part_06,part_07}.rs`.
// The substitutions relative to direct production `#[path]` inclusion
// are documented in the mirror's header and at the section heads of
// each block.
//
// BINDING LEDGER (mirrors production_inner/v1_primitive_lowering_production.rs)
// ============================================================================
//   - lower_set           <- part_05_ir.rs:41-55
//   - lower_do            <- part_05_ir.rs:58-75
//   - lower_choose        <- part_06.rs:20-51
//   - lower_for_each      <- part_06.rs:54-94
//   - lower_together      <- part_06.rs:97-135
//   - lower_collect       <- part_06.rs:146-193
//   - lower_reduce        <- part_06.rs:196-244
//   - lower_repeat        <- part_07.rs:16-65
//   - lower_wait          <- part_07.rs:84-111
//   - lower_ask           <- part_07.rs:114-152
//   - lower_finish        <- part_07.rs:155-165
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every projection in the mirror are NOT
// verified by Verus. Each projection is `#[verifier::external]` so
// Verus skips body verification. The contracts attached via
// `assume_specification` in the companion spec file
// `v1_primitive_lowering.rs` state the production behavior the spec
// proofs discharge. Drift between the mirror and the production source
// is reported as binding-debt tracked outside Verus.

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
// `production_inner/v1_primitive_lowering_production.rs` (NOT the
// actual production source). The mirror is a hand-written structural
// copy of the production `lower_*` projections with documented
// substitutions (ID types flattened, projection bodies replaced by
// no-op `#[verifier::external]` wrappers). Any drift in field NAME,
// discriminant shape, or projection signature breaks the verification
// build.
#[path = "production_inner/v1_primitive_lowering_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;