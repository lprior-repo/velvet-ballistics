// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN WRAPPER for step_state_machine.rs Verus spec
// ============================================================================
//
// Companion extern file referenced by `step_state_machine.rs` via
// `#[path = "extern_step_state_machine.rs"] mod production;`. The actual
// production mirror content (types + exec wrappers) lives in the
// `production_inner/step_state_machine_production.rs` file. This wrapper
// `#[path]`-includes that mirror and re-exports its items under the
// `production::*` namespace so the spec file's `pub use production::{...}`
// resolution continues to work without modification.
//
// This WEAK-binding pattern satisfies
// `scripts/check-verus-production-binding.sh`:
//
//   1. The spec file references this extern via `#[path = "extern_*"]`
//      and uses `assume_specification` (already present).
//   2. THIS extern file has `#[path = "production_inner/..."]` which is
//      the binding-gate WEAK marker.
//
// Production mirror binding ledger (pointing to production source files):
//   - `StepBudget`             <- crates/vb_core/src/engine/signals.rs:14-16
//   - `EngineSignal`           <- crates/vb_core/src/engine/signals.rs:100-115
//   - `StepBudget::new`        <- crates/vb_core/src/engine/signals.rs:26-35
//   - `StepBudget::try_take`   <- crates/vb_core/src/engine/signals.rs:50-60
//   - `StepBudget::remaining`  <- crates/vb_core/src/engine/signals.rs:62-65
//   - `StepBudget::from_env`   <- crates/vb_core/src/engine/signals.rs:80-94
//   - `StepBudget::MAX`        <- crates/vb_core/src/engine/signals.rs:19-22
//   - `mark_step_after_signal` <- crates/vb_core/src/engine/step.rs:109-121
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the production mirror are NOT
// verified by Verus. Each exec fn is `#[verifier::external]` so Verus
// skips body verification, and the contracts attached via
// `assume_specification` in the companion spec file state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt item
// outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

#[path = "production_inner/step_state_machine_production.rs"]
pub mod step_state_machine_production;

pub use step_state_machine_production::{
    EngineSignal, SpecEngineError, SpecStepStateMirror, StepBudget,
    engine_signal_discriminant, spec_mark_step_after_signal,
    step_budget_from_env, step_budget_max, step_budget_new,
    step_budget_remaining, step_budget_try_take,
};

} // verus!
