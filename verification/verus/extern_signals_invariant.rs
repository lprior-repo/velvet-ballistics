// SPDX-License-Identifier: MIT
//
// Extern surface for signals_invariant Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the
// `signals_invariant.rs` Verus spec. It contains a direct `#[path]`
// inclusion of the in-tree mirror at
// `verification/verus/production_inner/signals_production.rs`, which
// is a verbatim copy of `crates/vb_core/src/engine/signals.rs` with
// two minimal substitutions:
//
//   1. The `StepBudget::remaining` field is declared `pub` (instead
//      of private as in production). This relaxation is required so
//      Verus's `#[verifier::external_type_specification]` bridge can
//      read the field on the spec side. Field NAME and TYPE are
//      preserved byte-for-byte so any drift in NAME breaks the
//      verification build.
//
//   2. The `StepBudget::from_env` body is wrapped in
//      `#[verifier::external_body]` because the closure pattern
//      `|_| EngineError::BudgetParse { reason: ... }` in production
//      (signals.rs:84) is rejected by Verus 0.2026.05.05 (Rust
//      1.95.0) as "only variables are supported here, not general
//      patterns". The signature and field name remain
//      production-identical.
//
// The mirror is included via `#[path]` from inside `verus!` (WITHOUT
// module-level `#[verifier::external]`) so the type declarations are
// nameable in spec mode. The companion spec file
// `signals_invariant.rs` attaches `assume_specification` contracts
// to the production-bound exec methods.
//
// =============================================================================
// BINDING LEDGER (mirrors production_inner/signals_production.rs)
// =============================================================================
//   - `StepBudget`                          <- crates/vb_core/src/engine/signals.rs:13-16
//   - `StepBudget::MAX`                     <- crates/vb_core/src/engine/signals.rs:19-22
//   - `StepBudget::new`                     <- crates/vb_core/src/engine/signals.rs:26-35
//   - `StepBudget::try_take`                <- crates/vb_core/src/engine/signals.rs:50-60
//   - `StepBudget::remaining`               <- crates/vb_core/src/engine/signals.rs:62-65
//   - `StepBudget::from_env`                <- crates/vb_core/src/engine/signals.rs:80-94
//                                             (body wrapped in #[verifier::external_body]
//                                              due to closure-pattern blocker; signature
//                                              and field name unchanged)
//   - `EngineSignal`                        <- crates/vb_core/src/engine/signals.rs:99-115
//                                             (7 variants, all production discriminants
//                                              preserved verbatim)
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. The mirror is included inside `verus!` (so the type
// declarations are nameable in spec mode), but each production method
// body is either verbatim Rust (which Verus does not attempt to
// verify beyond syntax) or wrapped in `#[verifier::external_body]` for
// the `from_env` closure pattern. The contracts attached via
// `assume_specification` in `signals_invariant.rs` state the
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
// `production_inner/signals_production.rs` (NOT the actual
// production source). The mirror is verbatim except for the
// `pub remaining` field relaxation (required by Verus's
// `external_type_specification`) and the `#[verifier::external_body]`
// wrapper on `from_env` (required because of the closure-pattern
// blocker in production). Any drift in field NAME or method
// signature breaks the verification build.
#[path = "production_inner/signals_production.rs"]
pub mod production_signals;

} // verus!

// Re-export the production types so the spec file can reference them
// via `crate::production::production_signals::StepBudget`.
pub use production_signals::{EngineSignal, EngineError, StepBudget, MAX_STEP_BUDGET};