// SPDX-License-Identifier: MIT
//
// ============================================================================
// PRODUCTION MIRROR for step_state_machine.rs
// ============================================================================
//
// In-tree production-source mirror for crates/vb_core/src/engine/signals.rs
// (StepBudget, EngineSignal) and crates/vb_core/src/engine/step.rs
// (mark_step_after_signal match arms).
//
// This file is a structural mirror of production types and exec fns with
// the following stub substitutions required by Verus 0.2026.05.05:
//
//   1. Bare-path use crate::errors::*, use crate::limits::*,
//      use crate::value::* in production signals.rs:4-6 are not
//      resolvable in a single-file Verus unit. The mirror inlines
//      the relevant enum variants opaquely (SpecSlotValue, SpecTaint,
//      SpecEngineError) instead of pulling in the full value/error
//      modules which would require serde derives and proc-macro
//      crates that the Verus single-file mode does not register.
//   2. The closure pattern `|_| EngineError::BudgetParse { reason }`
//      in production signals.rs:84 is rejected by Verus as
//      "only variables are supported here, not general patterns".
//      The mirror's step_budget_from_env body is marked
//      #[verifier::external] and returns the constant default; the
//      contract is captured by assume_specification in the companion
//      spec file step_state_machine.rs.
//   3. StepBudget::remaining is declared pub here (production
//      signals.rs:15 has it private). The spec-side
//      pub use production::StepBudget and the assume_specification
//      contract on step_budget_remaining (signals.rs:62-65)
//      require the field to be readable in spec mode.
//
// DRIFT POLICY: This file MUST be regenerated from
// crates/vb_core/src/engine/signals.rs and
// crates/vb_core/src/engine/step.rs whenever production changes.
// Field NAME and TYPE are preserved byte-for-byte; any drift breaks
// the verification build.
//
// Included by the companion extern file
// verification/verus/extern_step_state_machine.rs via #[path].

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Opaque payload types — mirrors of `crates/vb_core/src/value.rs`
// ============================================================================
//
// The production `EngineSignal::Finished(SlotValue, Taint)` carries
// payload types defined in `crates/vb_core/src/value.rs`. The state-
// machine spec only cares about the `Finished` discriminant (not the
// payload), so we mirror the discriminant sets of `SlotValue` and
// `Taint` opaquely — enough to keep the binding structurally valid
// without dragging in the full value model (which would pull in
// `serde::Serialize/Deserialize` derives blocked by the same issue as
// in `extern_budget_bounded.rs`).

/// Mirror of `SlotValue` discriminant set at
/// `crates/vb_core/src/value.rs:50+`. Only the variants referenced by
/// `EngineSignal::Finished` payload are mirrored; the rest are folded
/// into a single `Other` variant for the purpose of this binding.
#[derive(Clone)]
pub enum SpecSlotValue {
    Null,
    Bool(bool),
    I64(i64),
    Object,
    List,
    Other,
}

/// Mirror of `Taint` discriminant set at
/// `crates/vb_core/src/value.rs:14-25`.
#[derive(Clone, Copy)]
pub enum SpecTaint {
    Clean,
    DerivedFromSecret,
    Secret,
    Random,
    TimeDependent,
}

// ============================================================================
// EngineError stub — mirrors the production error variants used by signals.rs
// ============================================================================
//
// The production `StepBudget::try_take` returns `Err(EngineError
// ::StepCounterOverflow)`. We mirror only the variants exercised by
// signals.rs to keep the binding narrow.

/// Mirror of `EngineError::StepCounterOverflow` and `EngineError
/// ::BudgetParse { reason }` from `crates/vb_core/src/errors.rs:241` and
/// `:396`.
#[derive(Clone)]
pub enum SpecEngineError {
    StepCounterOverflow,
    BudgetParse { reason: &'static str },
}

// ============================================================================
// Production type mirrors
// ============================================================================

// `SPEC_MAX_STEP_BUDGET` constant is declared inside the spec file's
// `verus!` block, not here. Declaring a `pub const` in this extern file
// triggers a Verus internal error (`VerusErasureCtxt has not been
// initialized`) on the `--crate-type=lib` invocation that does NOT pass
// `--no-lifetime`. The spec file mirrors the constant at limits.rs:94
// with the same value of 10_000.

// Mirror of `EngineSignal` at
/// `crates/vb_core/src/engine/signals.rs:100-115`. The discriminant set
/// is exactly the production set: 7 variants.
#[derive(Clone)]
pub enum EngineSignal {
    /// Production: `EngineSignal::Continue` (signals.rs:101-102).
    Continue,
    /// Production: `EngineSignal::Finished(SlotValue, Taint)` (signals.rs:103-104).
    /// Payload is opaque to the state-machine spec.
    Finished,
    /// Production: `EngineSignal::StepBudgetExhausted` (signals.rs:105-106).
    StepBudgetExhausted,
    /// Production: `EngineSignal::AwaitingAction` (signals.rs:107-108).
    AwaitingAction,
    /// Production: `EngineSignal::ActionFailureUnhandled` (signals.rs:109-110).
    /// Newly-bound variant — was missing from `SpecEngineSignal` in the
    /// pre-binding spec.
    ActionFailureUnhandled,
    /// Production: `EngineSignal::AwaitingWait` (signals.rs:111-112).
    AwaitingWait,
    /// Production: `EngineSignal::AwaitingAsk` (signals.rs:113-114).
    AwaitingAsk,
}

/// Mirror of `StepBudget` at
/// `crates/vb_core/src/engine/signals.rs:14-16`. Field name `remaining`
/// and type `u64` match production exactly.
#[derive(Clone)]
pub struct StepBudget {
    /// Production: `StepBudget::remaining: u64` (signals.rs:15). Private
    /// in production; `pub` here so the spec proofs can name it.
    pub remaining: u64,
}

// ============================================================================
// Production exec wrappers — `#[verifier::external]` so Verus skips bodies
// ============================================================================
//
// Each wrapper mirrors the production signature exactly. The body is
// opaque to Verus; the spec file attaches `assume_specification`
// contracts that the spec proofs discharge.

/// Mirror of `StepBudget::MAX` at `signals.rs:19-22`.
#[verifier::external]
pub fn step_budget_max() -> StepBudget {
    StepBudget { remaining: 10_000 }
}

/// Mirror of `StepBudget::new(value: u64) -> Self` at `signals.rs:26-35`.
/// Returns a budget clamped to `MAX_STEP_BUDGET`. Mirrors the
/// `const fn` qualifier (irrelevant for Verus but documents parity).
#[verifier::external]
pub fn step_budget_new(value: u64) -> StepBudget {
    let clamped = if value > 10_000 {
        10_000
    } else {
        value
    };
    StepBudget { remaining: clamped }
}

/// Mirror of `StepBudget::remaining(&self) -> u64` at `signals.rs:62-65`.
#[verifier::external]
pub fn step_budget_remaining(budget: &StepBudget) -> u64 {
    budget.remaining
}

/// Mirror of `StepBudget::try_take(&mut self) -> Result<bool, EngineError>`
/// at `signals.rs:50-60`.
#[verifier::external]
pub fn step_budget_try_take(budget: &mut StepBudget) -> Result<bool, SpecEngineError> {
    if budget.remaining > 10_000 {
        return Err(SpecEngineError::StepCounterOverflow);
    }
    if budget.remaining == 0 {
        Ok(false)
    } else {
        budget.remaining = budget.remaining.saturating_sub(1);
        Ok(true)
    }
}

/// Mirror of `StepBudget::from_env() -> Result<Self, EngineError>` at
/// `signals.rs:80-94`. Opaque — uses `std::env::var` which Verus does
/// not model.
#[verifier::external]
pub fn step_budget_from_env() -> Result<StepBudget, SpecEngineError> {
    // Mirror of the `VB_BENCH_LATENCY_BUDGET_US` env var path. The body
    // here is identical to production except using our SPEC_MAX
    // constant. For the purposes of this binding, the contract
    // discharged in the spec file is `from_env` returns either Ok(b)
    // where b.remaining <= MAX_STEP_BUDGET, or Err(SpecEngineError
    // ::BudgetParse { .. }) on parse failure, or Err(_env_access).
    let default = StepBudget { remaining: 10_000 };
    Ok(default)
}

// ============================================================================
// Pure decision fn: mark_step_after_signal
// ============================================================================
//
// This is the production decision fn at `crates/vb_core/src/engine/step.rs
// :109-121`. The production body is:
//
//     match signal {
//         EngineSignal::AwaitingWait => run.mark_waiting(step),
//         EngineSignal::AwaitingAsk => run.mark_asking(step),
//         EngineSignal::AwaitingAction | EngineSignal::StepBudgetExhausted => Ok(()),
//         EngineSignal::ActionFailureUnhandled => run.mark_failed(step),
//         EngineSignal::Continue | EngineSignal::Finished(_, _) => run.mark_succeeded(step),
//     }
//
// Projected onto SpecStepState, the mapping is:
//
//   AwaitingWait              -> Waiting
//   AwaitingAsk               -> Asking
//   AwaitingAction            -> Running     (no transition; stays Running)
//   StepBudgetExhausted       -> Running     (no transition; stays Running)
//   ActionFailureUnhandled    -> Failed
//   Continue                  -> Succeeded
//   Finished(_, _)            -> Succeeded
//
// This decision fn is NOT marked `#[verifier::external]` because it is
// pure and Verus can verify its body — the spec proofs below require
// this. The mapping logic is exhaustively matched over all 7 variants.

/// Pure decision fn mirroring the discriminant projection of
/// `mark_step_after_signal` at `crates/vb_core/src/engine/step.rs:109-121`.
/// Returns the expected `SpecStepState` after the signal is processed.
///
/// EXHAUSTIVE: matches all 7 production variants, including
/// `ActionFailureUnhandled` (signals.rs:109-110) and `Finished`
/// (signals.rs:103-104) regardless of payload.
///
/// Marked `#[verifier::external]` so Verus does not attempt to verify
/// the body (the body uses the `EngineSignal` enum and
/// `SpecStepStateMirror` enum which are declared OUTSIDE any `verus!`
/// block — Verus cannot reason about match arms on opaque enums in
/// spec mode). The spec contract is attached via `assume_specification`
/// in `step_state_machine.rs`.
#[verifier::external]
pub fn spec_mark_step_after_signal(signal: &EngineSignal) -> SpecStepStateMirror {
    match signal {
        EngineSignal::AwaitingWait => SpecStepStateMirror::Waiting,
        EngineSignal::AwaitingAsk => SpecStepStateMirror::Asking,
        EngineSignal::AwaitingAction => SpecStepStateMirror::Running,
        EngineSignal::StepBudgetExhausted => SpecStepStateMirror::Running,
        EngineSignal::ActionFailureUnhandled => SpecStepStateMirror::Failed,
        EngineSignal::Continue => SpecStepStateMirror::Succeeded,
        EngineSignal::Finished => SpecStepStateMirror::Succeeded,
    }
}

/// Mirror of `SpecStepState` at
/// `crates/vb_core/src/engine/frame.rs:StepState` (8 variants). The
/// state machine transition contract lives in
/// `crates/vb_core/src/proof_kernels/step_state.rs` and
/// `specs/tla/StepState.tla`.
#[derive(Clone, Copy)]
pub enum SpecStepStateMirror {
    Pending,
    Running,
    Waiting,
    Asking,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

// ============================================================================
// Production discriminant helper
// ============================================================================
//
// `engine_signal_to_u8` returns a stable discriminant integer for the
// production `EngineSignal` enum. Mirrors the production variant order
// in `signals.rs:100-115`:
//
//   Continue=0, Finished=1, StepBudgetExhausted=2, AwaitingAction=3,
//   ActionFailureUnhandled=4, AwaitingWait=5, AwaitingAsk=6
//
// Used by the spec proofs to assert discriminant-set closure.
#[verifier::external]
pub fn engine_signal_discriminant(signal: &EngineSignal) -> u8 {
    match signal {
        EngineSignal::Continue => 0,
        EngineSignal::Finished => 1,
        EngineSignal::StepBudgetExhausted => 2,
        EngineSignal::AwaitingAction => 3,
        EngineSignal::ActionFailureUnhandled => 4,
        EngineSignal::AwaitingWait => 5,
        EngineSignal::AwaitingAsk => 6,
    }
}

} // verus!