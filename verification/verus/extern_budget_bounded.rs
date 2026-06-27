// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for budget_bounded Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the `budget_bounded.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/budget_bounded_production.rs` via
// `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_budget_bounded.rs"]`; this file uses
//     `#[path = "production_inner/budget_bounded_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the `production_inner/budget_bounded_production.rs`
//     mirror and the spec proofs that depend on it.
//
// The mirror at `production_inner/budget_bounded_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_core/src/budget.rs`. The substitutions relative to direct
// production `#[path]` inclusion are documented in the mirror's header
// and at the section heads of each block.
//
// BINDING LEDGER (mirrors production_inner/budget_bounded_production.rs)
// ============================================================================
//   - `WholeWorkflowBudget`                  <- crates/vb_core/src/budget.rs:11-59
//   - `WholeWorkflowBudget::compute`         <- crates/vb_core/src/budget.rs:64-70
//   - `BoundednessPolicy`                    <- crates/vb_core/src/budget.rs:341-375
//   - `BoundednessPolicy::validate`          <- crates/vb_core/src/budget.rs:400-457
//   - `BudgetError`                          <- crates/vb_core/src/budget.rs:533-568
//   - `AggregateResourceBudget`              <- crates/vb_core/src/budget.rs:571-596
//   - `AggregateResourceUsage`               <- crates/vb_core/src/budget.rs:622-644
//   - `AggregateBudgetError`                 <- crates/vb_core/src/budget.rs:655-725
//   - `validate_aggregate_budget`            <- crates/vb_core/src/budget.rs:1110-1209
//   - `validate_step_ceilings`               <- crates/vb_core/src/budget.rs:1213-1248
//   - `add_dim`, `sub_dim`                   <- crates/vb_core/src/budget.rs:1250-1268
//   - `count_total_steps`                    <- crates/vb_core/src/budget.rs:1332-1360
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file `budget_bounded.rs` state the production
// behavior the spec proofs discharge. Drift between the mirror and
// the production source is reported as binding-debt tracked outside
// Verus.

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
// `production_inner/budget_bounded_production.rs` (NOT the actual
// production source). The mirror is a hand-written structural copy of
// `crates/vb_core/src/budget.rs` with documented substitutions
// (thiserror/serde stripped, method bodies replaced by no-op
// `#[verifier::external]` wrappers). Any drift in field NAME,
// discriminant shape, or method signature breaks the verification build.
#[path = "production_inner/budget_bounded_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;