// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for vb_core_replay_step_spec Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_core_replay_step_spec.rs` Verus spec. It includes the in-tree
// production mirror at
// `verification/verus/production_inner/vb_core_replay_step_production.rs`
// via `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_vb_core_replay_step.rs"]`; this file uses
//     `#[path = "production_inner/vb_core_replay_step_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/vb_core_replay_step_production.rs` mirror and
//     the spec proofs that depend on it.
//
// The mirror at
// `production_inner/vb_core_replay_step_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_core/src/replay/step.rs`,
// `crates/vb_core/src/replay/choose/mod.rs`, and
// `crates/vb_core/src/replay/ops.rs`. The substitutions relative to
// direct production `#[path]` inclusion are documented in the
// mirror's header (in summary: the production replay module
// transitively depends on `vb_core::frame::RunFrame`,
// `vb_core::value::ValueStore`, and `vb_core::workflow::CompiledWorkflow`,
// which contain heap allocations, indices, and runtime internals
// that Verus does not model end-to-end).
//
// ============================================================================
// BINDING LEDGER (mirrors production_inner/vb_core_replay_step_production.rs)
// ============================================================================
//   - `SpecReplayAction` (7-variant enum)              <- crates/vb_core/src/replay/step.rs:50-57
//   - `SpecNodeKind` (18-variant enum)                <- crates/vb_core/src/workflow/node.rs
//   - `SpecSuspensionKind` (4-variant enum)           <- crates/vb_core/src/replay/step.rs:18-27
//   - `SpecOpStackDelta` (5-variant enum)             <- projection of crates/vb_core/src/replay/ops.rs:13-44
//   - `replay_step_pure_decision`                     <- crates/vb_core/src/replay/step.rs:128-192
//   - `replay_choose_slot_pure_decision`              <- crates/vb_core/src/replay/choose/mod.rs:12-58
//   - `replay_choose_expr_pure_decision`              <- crates/vb_core/src/replay/choose/mod.rs:61-104
//   - `eval_replay_op_stack_delta`                    <- crates/vb_core/src/replay/ops.rs:13-44
//   - `pop_pair_pure`                                 <- crates/vb_core/src/replay/ops.rs:244-248
//   - `pop_i64_pair_pure`                             <- crates/vb_core/src/replay/ops.rs:250-254
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`vb_core_replay_step_spec.rs`) state the
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
// `production_inner/vb_core_replay_step_production.rs` (NOT the
// actual production source). The mirror is a hand-written structural
// copy of the `vb_core::replay::*` decision fns with documented
// substitutions (heap-allocating surfaces collapsed to scalars,
// method bodies replaced by `#[verifier::external]` wrappers). Any
// drift in field NAME, discriminant shape, or method signature
// breaks the verification build.
#[path = "production_inner/vb_core_replay_step_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;
