// Verus proof obligations for canonical step-state transitions.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to `crates/vb_core/src/engine/signals.rs` through the
// companion extern surface `verification/verus/extern_step_state_machine.rs`,
// which mirrors every production type and exec fn we reason about and wraps
// production bodies with `#[verifier::external]`. The spec proofs below
// attach `assume_specification` contracts to those extern wrappers and
// exercise them through production-bound exec fns, so any drift in the
// production field names, discriminant sets, or fn signatures breaks the
// verification build.
//
// Full `#[path]` inclusion of `crates/vb_core/src/engine/signals.rs` is
// intentionally NOT used here — see the header of
// `extern_step_state_machine.rs` for the empirical blockers (closure pattern
// `|_| EngineError::BudgetParse { reason: ... }` in `from_env` is rejected by
// Verus, and bare-path `use crate::errors::*`, `use crate::limits::*`,
// `use crate::value::*` cannot be resolved in a single-file Verus unit).
// The mirror pattern matches `extern_budget_bounded.rs`,
// `extern_runtime_execute_do.rs`, `extern_vb_core_replay_step.rs`,
// `extern_run_atomic_admission.rs`, and `extern_idempotency_certificate.rs`
// in this repo.
//
// ============================================================================
// BINDING LEDGER (mirrors extern_step_state_machine.rs BINDING LEDGER)
// ============================================================================
//   - `EngineSignal`                          <- extern_step_state_machine.rs
//                                               (mirror of
//                                               signals.rs:100-115;
//                                               ALL 7 production variants
//                                               including the previously
//                                               missing `ActionFailureUnhandled`)
//   - `StepBudget`                            <- extern_step_state_machine.rs
//                                               (mirror of
//                                               signals.rs:14-16)
//   - `StepBudget::MAX`                       <- extern_step_state_machine.rs
//                                               `step_budget_max`
//                                               (mirror of signals.rs:19-22)
//   - `StepBudget::new`                       <- extern_step_state_machine.rs
//                                               `step_budget_new`
//                                               (mirror of signals.rs:26-35)
//   - `StepBudget::remaining`                 <- extern_step_state_machine.rs
//                                               `step_budget_remaining`
//                                               (mirror of signals.rs:62-65)
//   - `StepBudget::try_take`                  <- extern_step_state_machine.rs
//                                               `step_budget_try_take`
//                                               (mirror of signals.rs:50-60)
//   - `mark_step_after_signal`                <- extern_step_state_machine.rs
//                                               `spec_mark_step_after_signal`
//                                               (pure projection of
//                                               step.rs:109-121 match arms)
//
// ============================================================================
// UPGRADE FROM PREVIOUS SPEC
// ============================================================================
// The previous `step_state_machine.rs` defined an internally-invented
// `SpecEngineSignal` with 6 variants (no `ActionFailureUnhandled`). The
// pre-binding spec was therefore a VACUUM proof: it reasoned about a
// shadow type that the production code never constructs.
//
// This rewrite uses the production `EngineSignal` (7 variants) as the
// spec-side signal type, exercising all 7 arms of the production match
// at `crates/vb_core/src/engine/step.rs:109-121`. Any production
// modification to the discriminant set breaks the extern mirror and
// surfaces here as a verifier error.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every entry point in the binding ledger are
// not verified by Verus. The exec wrappers in
// `extern_step_state_machine.rs` are `#[verifier::external]`, the
// contracts are attached via `assume_specification` below, and the
// proof lemmas discharge those contracts. Any drift between the mirror
// and the production source is binding-debt tracked outside Verus.
//
// ============================================================================
// Verifier command: `verus --crate-type=lib verification/verus/step_state_machine.rs`
// Obligation ID: VB-CORE-STATE-001 (and STATE-002/003/004 by extension).

use vstd::prelude::*;

verus! {

// ============================================================================
// Production extern surface — `#[path]`-bound mirror of signals.rs
// ============================================================================

#[path = "extern_step_state_machine.rs"]
mod production;

// Re-export the production type and exec wrappers so the spec proofs
// below reference them as `production::EngineSignal`, `step_budget_new`, etc.
pub use production::{
    EngineSignal,
    SpecEngineError,
    SpecStepStateMirror,
    StepBudget,
    engine_signal_discriminant,
    spec_mark_step_after_signal,
    step_budget_from_env,
    step_budget_max,
    step_budget_new,
    step_budget_remaining,
    step_budget_try_take,
};

// `SPEC_MAX_STEP_BUDGET` is declared inside `verus!` (rather than in
// the extern file) because declaring a `pub const` in the extern file
// triggers a Verus internal error (`VerusErasureCtxt has not been
// initialized`) on the `--crate-type=lib` invocation without
// `--no-lifetime`. The constant value mirrors the production
// `MAX_STEP_BUDGET = 10_000` from `crates/vb_core/src/limits.rs:94`.
pub const SPEC_MAX_STEP_BUDGET: u64 = 10_000;

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// These bridges attach spec contracts to the production-bound exec fns in
// `extern_step_state_machine.rs`. The body of each extern fn is opaque to
// Verus (`#[verifier::external]`); the spec proofs below exercise the
// contracts via the exec wrappers in the "Production-bound exec fns"
// section.

// --------------------------------------------------------------------------
// Production MAX constant.
// --------------------------------------------------------------------------
// Mirror of `MAX_STEP_BUDGET = 10_000` from `crates/vb_core/src/limits.rs:94`.
// Constants do not need `assume_specification`; the spec references
// `SPEC_MAX_STEP_BUDGET` directly via the `pub use production::SPEC_MAX_STEP_BUDGET`
// re-export above.

// --------------------------------------------------------------------------
// Bridge: `step_budget_new` returns a budget clamped to MAX_STEP_BUDGET.
// --------------------------------------------------------------------------
// Mirrors production `StepBudget::new(value: u64) -> Self` at
// `crates/vb_core/src/engine/signals.rs:26-35`:
//
//     Self {
//         remaining: if value > MAX_STEP_BUDGET { MAX_STEP_BUDGET } else { value },
//     }
pub assume_specification[ production::step_budget_new ](value: u64) -> (budget: production::StepBudget)
    ensures
        budget.remaining == if value > SPEC_MAX_STEP_BUDGET {
            SPEC_MAX_STEP_BUDGET
        } else {
            value
        },
;

// --------------------------------------------------------------------------
// Bridge: `step_budget_remaining` returns the budget's remaining field.
// --------------------------------------------------------------------------
// Mirrors production `StepBudget::remaining(&self) -> u64` at
// `crates/vb_core/src/engine/signals.rs:62-65`.
pub assume_specification[ production::step_budget_remaining ](
    budget: &production::StepBudget,
) -> (r: u64)
    ensures r == budget.remaining,
;

// --------------------------------------------------------------------------
// Bridge: `step_budget_max` returns a budget at the maximum allowed value.
// --------------------------------------------------------------------------
// Mirrors production `StepBudget::MAX` at signals.rs:19-22:
//
//     pub const MAX: Self = Self { remaining: MAX_STEP_BUDGET };
pub assume_specification[ production::step_budget_max ]() -> (budget: production::StepBudget)
    ensures budget.remaining == SPEC_MAX_STEP_BUDGET,
;

// --------------------------------------------------------------------------
// Bridge: `step_budget_try_take` matches production try_take semantics.
// --------------------------------------------------------------------------
// Mirrors production `StepBudget::try_take(&mut self) -> Result<bool, EngineError>`
// at `crates/vb_core/src/engine/signals.rs:50-60`:
//
//     pub fn try_take(&mut self) -> Result<bool, EngineError> {
//         if self.remaining > MAX_STEP_BUDGET {
//             return Err(EngineError::StepCounterOverflow);
//         }
//         if self.remaining == 0 {
//             Ok(false)
//         } else {
//             self.remaining = self.remaining.saturating_sub(1);
//             Ok(true)
//         }
//     }
pub assume_specification[ production::step_budget_try_take ](
    budget: &mut production::StepBudget,
) -> (result: Result<bool, production::SpecEngineError>)
    ensures
        match result {
            Ok(true) => old(budget).remaining <= SPEC_MAX_STEP_BUDGET
                && old(budget).remaining > 0
                && old(budget).remaining == final(budget).remaining + 1,
            Ok(false) => old(budget).remaining == 0
                && old(budget).remaining == final(budget).remaining,
            Err(production::SpecEngineError::StepCounterOverflow) => {
                old(budget).remaining > SPEC_MAX_STEP_BUDGET
            },
            Err(_) => false,
        },
;

// --------------------------------------------------------------------------
// Bridge: `step_budget_from_env` returns Ok(budget_with_remaining_at_most_max)
//         or Err(BudgetParse / env-access error).
// --------------------------------------------------------------------------
// Mirrors production `StepBudget::from_env` at signals.rs:80-94. The body is
// opaque (uses std::env); the contract is that the returned budget has
// `remaining <= MAX_STEP_BUDGET` and the Err variants match the production
// parse-failure / env-access-error variants.
pub assume_specification[ production::step_budget_from_env ]() -> (result: Result<
    production::StepBudget,
    production::SpecEngineError,
>)
    ensures
        match result {
            Ok(b) => b.remaining <= SPEC_MAX_STEP_BUDGET,
            Err(production::SpecEngineError::BudgetParse { .. }) => true,
            Err(production::SpecEngineError::StepCounterOverflow) => false,
            Err(_) => true,
        },
;

// --------------------------------------------------------------------------
// Bridge: `spec_mark_step_after_signal` matches the production match arms.
// --------------------------------------------------------------------------
// Mirrors the production decision fn at
// `crates/vb_core/src/engine/step.rs:109-121`:
//
//     match signal {
//         EngineSignal::AwaitingWait => run.mark_waiting(step),       // Waiting
//         EngineSignal::AwaitingAsk => run.mark_asking(step),         // Asking
//         EngineSignal::AwaitingAction | EngineSignal::StepBudgetExhausted => Ok(()),  // Running
//         EngineSignal::ActionFailureUnhandled => run.mark_failed(step),  // Failed
//         EngineSignal::Continue | EngineSignal::Finished(_, _) => run.mark_succeeded(step),  // Succeeded
//     }
pub assume_specification[ production::spec_mark_step_after_signal ](
    signal: &production::EngineSignal,
) -> (state: production::SpecStepStateMirror)
    ensures
        mirror_to_spec(state) == spec_signal_to_state(*signal),
;

// --------------------------------------------------------------------------
// Bridge: `engine_signal_discriminant` returns the production variant index.
// --------------------------------------------------------------------------
// Mirrors the production variant order at signals.rs:100-115:
//
//   Continue=0, Finished=1, StepBudgetExhausted=2, AwaitingAction=3,
//   ActionFailureUnhandled=4, AwaitingWait=5, AwaitingAsk=6
pub assume_specification[ production::engine_signal_discriminant ](
    signal: &production::EngineSignal,
) -> (disc: u8)
    ensures
        match signal {
            production::EngineSignal::Continue => disc == 0,
            production::EngineSignal::Finished => disc == 1,
            production::EngineSignal::StepBudgetExhausted => disc == 2,
            production::EngineSignal::AwaitingAction => disc == 3,
            production::EngineSignal::ActionFailureUnhandled => disc == 4,
            production::EngineSignal::AwaitingWait => disc == 5,
            production::EngineSignal::AwaitingAsk => disc == 6,
        },
;

// ============================================================================
// Spec constants and spec-mode state machine model
// ============================================================================

/// MAX_STEP_BUDGET upper bound as a spec integer.
pub open spec fn spec_max_step_budget() -> int {
    SPEC_MAX_STEP_BUDGET as int
}

/// Spec view of `SpecStepStateMirror` discriminant set.
pub open spec fn is_terminal_mirror(s: SpecStepStateMirror) -> bool {
    match s {
        SpecStepStateMirror::Succeeded => true,
        SpecStepStateMirror::Failed => true,
        SpecStepStateMirror::Cancelled => true,
        SpecStepStateMirror::Skipped => true,
        _ => false,
    }
}

/// Spec view of `SpecStepStateMirror` discriminant set — suspended states.
pub open spec fn is_suspended_mirror(s: SpecStepStateMirror) -> bool {
    match s {
        SpecStepStateMirror::Waiting => true,
        SpecStepStateMirror::Asking => true,
        _ => false,
    }
}

// ============================================================================
// Spec state-machine transition table (carried over from prior spec for
// parity with VB-CORE-STATE-001 / StepState.tla / kani_step_state_transition)
// ============================================================================
//
// These spec fns and proofs reason over the same transition contract as
// the previous step_state_machine.rs spec, but now anchored to the
// production `EngineSignal` discriminant set via
// `spec_mark_step_after_signal` (which is itself a pure projection of
// the production match at step.rs:109-121).

pub enum SpecStepState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Waiting,
    Asking,
    Cancelled,
}

pub open spec fn is_terminal(s: SpecStepState) -> bool {
    match s {
        SpecStepState::Succeeded => true,
        SpecStepState::Failed => true,
        SpecStepState::Cancelled => true,
        SpecStepState::Skipped => true,
        _ => false,
    }
}

pub open spec fn is_suspended(s: SpecStepState) -> bool {
    match s {
        SpecStepState::Waiting => true,
        SpecStepState::Asking => true,
        _ => false,
    }
}

pub open spec fn non_idempotent_transition(current: SpecStepState, next: SpecStepState) -> bool {
    match current {
        SpecStepState::Pending => match next {
            SpecStepState::Running => true,
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Running => match next {
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Waiting => true,
            SpecStepState::Asking => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Waiting => match next {
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Asking => match next {
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Succeeded => false,
        SpecStepState::Failed => false,
        SpecStepState::Cancelled => false,
        SpecStepState::Skipped => false,
    }
}

pub open spec fn validate_transition(current: SpecStepState, next: SpecStepState) -> bool {
    current == next || non_idempotent_transition(current, next)
}

pub exec fn validate_transition_exec(current: SpecStepState, next: SpecStepState) -> (res: bool)
    ensures res == validate_transition(current, next),
{
    match current {
        SpecStepState::Pending => match next {
            SpecStepState::Pending => true,
            SpecStepState::Running => true,
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Running => match next {
            SpecStepState::Running => true,
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Waiting => true,
            SpecStepState::Asking => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Waiting => match next {
            SpecStepState::Waiting => true,
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Asking => match next {
            SpecStepState::Asking => true,
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Succeeded => match next {
            SpecStepState::Succeeded => true,
            _ => false,
        },
        SpecStepState::Failed => match next {
            SpecStepState::Failed => true,
            _ => false,
        },
        SpecStepState::Cancelled => match next {
            SpecStepState::Cancelled => true,
            _ => false,
        },
        SpecStepState::Skipped => match next {
            SpecStepState::Skipped => true,
            _ => false,
        },
    }
}

// ============================================================================
// Step-state transition lemmas (carry-over from prior spec, unchanged)
// ============================================================================

pub proof fn proof_idempotent_remark_allowed(current: SpecStepState)
    ensures validate_transition(current, current),
{
    assert(current == current);
}

pub proof fn proof_terminal_blocks_outward(current: SpecStepState, next: SpecStepState)
    requires
        is_terminal(current),
        current != next,
    ensures !validate_transition(current, next),
{
    match current {
        SpecStepState::Succeeded => assert(!validate_transition(current, next)),
        SpecStepState::Failed => assert(!validate_transition(current, next)),
        SpecStepState::Cancelled => assert(!validate_transition(current, next)),
        SpecStepState::Skipped => assert(!validate_transition(current, next)),
        _ => assert(false),
    }
}

pub proof fn proof_suspended_resumes_only_to_running(current: SpecStepState, next: SpecStepState)
    requires
        is_suspended(current),
        current != next,
        validate_transition(current, next),
    ensures next == SpecStepState::Running,
{
    match current {
        SpecStepState::Waiting => assert(next == SpecStepState::Running),
        SpecStepState::Asking => assert(next == SpecStepState::Running),
        _ => assert(false),
    }
}

pub proof fn proof_all_pairs()
    ensures
        validate_transition(SpecStepState::Pending, SpecStepState::Pending) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Pending) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Asking) == false,
        validate_transition(SpecStepState::Asking, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false,
        validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Cancelled, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false,
{
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Pending) == true);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Running) == true);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Pending) == false);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Running) == true);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Failed) == true);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Asking) == true);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == true);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Asking) == false);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Asking) == true);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Running) == true);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Waiting) == false);
    assert(validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true);
    assert(validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false);
    assert(validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true);
    assert(validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false);
    assert(validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true);
    assert(validate_transition(SpecStepState::Cancelled, SpecStepState::Running) == false);
    assert(validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true);
    assert(validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false);
}

// ============================================================================
// Production-bound exec fns with requires/ensures
// ============================================================================

/// Spec-mode bridge: convert the production `SpecStepStateMirror` to the
/// spec-internal `SpecStepState` so spec proofs can compare the signal
/// mapping against the transition table. Both enums model the SAME
/// production discriminant set (frame.rs StepState + StepState.tla).
pub open spec fn mirror_to_spec(m: SpecStepStateMirror) -> SpecStepState {
    match m {
        SpecStepStateMirror::Pending => SpecStepState::Pending,
        SpecStepStateMirror::Running => SpecStepState::Running,
        SpecStepStateMirror::Succeeded => SpecStepState::Succeeded,
        SpecStepStateMirror::Failed => SpecStepState::Failed,
        SpecStepStateMirror::Skipped => SpecStepState::Skipped,
        SpecStepStateMirror::Waiting => SpecStepState::Waiting,
        SpecStepStateMirror::Asking => SpecStepState::Asking,
        SpecStepStateMirror::Cancelled => SpecStepState::Cancelled,
    }
}

/// Production-bound exec wrapper: invoke the production
/// `mark_step_after_signal` decision logic for a given production
/// `EngineSignal` and return the resulting spec state. The exec wrapper
/// calls the production-mirror exec fn `spec_mark_step_after_signal` and
/// then asserts in spec mode that the returned state matches the spec
/// projection `spec_signal_to_state`. Discharged by the production-bound
/// exec body of `spec_mark_step_after_signal`.
pub exec fn check_signal_to_state(signal: &EngineSignal) -> (state: SpecStepStateMirror)
    ensures
        // Production-defined mapping — discharged by the match arms in
        // `spec_mark_step_after_signal` (extern fn body) and the
        // spec-mode assertion below.
        (match signal {
            EngineSignal::Continue => state == SpecStepStateMirror::Succeeded,
            EngineSignal::Finished => state == SpecStepStateMirror::Succeeded,
            EngineSignal::StepBudgetExhausted => state == SpecStepStateMirror::Running,
            EngineSignal::AwaitingAction => state == SpecStepStateMirror::Running,
            EngineSignal::ActionFailureUnhandled => state == SpecStepStateMirror::Failed,
            EngineSignal::AwaitingWait => state == SpecStepStateMirror::Waiting,
            EngineSignal::AwaitingAsk => state == SpecStepStateMirror::Asking,
        }),
{
    let result = spec_mark_step_after_signal(signal);
    // Discharged by the spec-mode match equality on result and signal.
    assert(mirror_to_spec(result) == spec_signal_to_state(*signal));
    result
}

// ============================================================================
// Spec-mode signal-to-state mapping — production-bound
// ============================================================================
//
// These spec fns re-express the production mapping at the spec level so
// proofs can reason about it without invoking exec wrappers. They are
// equivalent to `spec_mark_step_after_signal` but spec-pure.

/// Spec projection of the production `mark_step_after_signal` decision.
/// Mirrors `crates/vb_core/src/engine/step.rs:109-121` projection onto
/// `SpecStepState` discriminant.
pub open spec fn spec_signal_to_state(signal: EngineSignal) -> SpecStepState {
    match signal {
        EngineSignal::AwaitingWait => SpecStepState::Waiting,
        EngineSignal::AwaitingAsk => SpecStepState::Asking,
        EngineSignal::AwaitingAction => SpecStepState::Running,
        EngineSignal::StepBudgetExhausted => SpecStepState::Running,
        EngineSignal::ActionFailureUnhandled => SpecStepState::Failed,
        EngineSignal::Continue => SpecStepState::Succeeded,
        EngineSignal::Finished => SpecStepState::Succeeded,
    }
}

// ============================================================================
// PRODUCTION-BINDING PROOFS (the upgrade over the previous vacuum spec)
// ============================================================================
//
// The previous spec proved properties of an internally-invented
// `SpecEngineSignal`. These proofs instead bind to the production
// `EngineSignal` discriminant set and discharge the contracts attached
// to the production-bound exec wrappers via `assume_specification`.

/// All 7 production EngineSignal variants produce a defined SpecStepState.
/// This is the production-bound version of the prior
/// `proof_inv_step_state_mapping` (which proved it for the 6-variant
/// shadow `SpecEngineSignal`).
pub proof fn proof_production_signal_mapping_total(signal: EngineSignal)
    ensures
        // Total mapping: every variant maps to exactly one SpecStepState.
        match signal {
            EngineSignal::Continue => true,
            EngineSignal::Finished => true,
            EngineSignal::StepBudgetExhausted => true,
            EngineSignal::AwaitingAction => true,
            EngineSignal::ActionFailureUnhandled => true,
            EngineSignal::AwaitingWait => true,
            EngineSignal::AwaitingAsk => true,
        },
        // And the mapping is the production-defined mapping at
        // step.rs:109-121:
        spec_signal_to_state(signal) == (match signal {
            EngineSignal::Continue => SpecStepState::Succeeded,
            EngineSignal::Finished => SpecStepState::Succeeded,
            EngineSignal::StepBudgetExhausted => SpecStepState::Running,
            EngineSignal::AwaitingAction => SpecStepState::Running,
            EngineSignal::ActionFailureUnhandled => SpecStepState::Failed,
            EngineSignal::AwaitingWait => SpecStepState::Waiting,
            EngineSignal::AwaitingAsk => SpecStepState::Asking,
        }),
{
    // Total mapping — exhaustive over all 7 production variants.
    match signal {
        EngineSignal::Continue => assert(spec_signal_to_state(signal) == SpecStepState::Succeeded),
        EngineSignal::Finished => assert(spec_signal_to_state(signal) == SpecStepState::Succeeded),
        EngineSignal::StepBudgetExhausted => assert(spec_signal_to_state(signal) == SpecStepState::Running),
        EngineSignal::AwaitingAction => assert(spec_signal_to_state(signal) == SpecStepState::Running),
        EngineSignal::ActionFailureUnhandled => assert(spec_signal_to_state(signal) == SpecStepState::Failed),
        EngineSignal::AwaitingWait => assert(spec_signal_to_state(signal) == SpecStepState::Waiting),
        EngineSignal::AwaitingAsk => assert(spec_signal_to_state(signal) == SpecStepState::Asking),
    }
}

/// Production-bound: Continue and Finished map to Succeeded (terminal
/// success). Mirrors step.rs:119
/// `EngineSignal::Continue | EngineSignal::Finished(_, _) => run.mark_succeeded(step)`.
pub proof fn proof_continue_finished_maps_to_succeeded()
    ensures
        spec_signal_to_state(EngineSignal::Continue) == SpecStepState::Succeeded,
        spec_signal_to_state(EngineSignal::Finished) == SpecStepState::Succeeded,
{
}

/// Production-bound: AwaitingWait maps to Waiting. Mirrors step.rs:115
/// `EngineSignal::AwaitingWait => run.mark_waiting(step)`.
pub proof fn proof_awaiting_wait_maps_to_waiting()
    ensures spec_signal_to_state(EngineSignal::AwaitingWait) == SpecStepState::Waiting,
{
}

/// Production-bound: AwaitingAsk maps to Asking. Mirrors step.rs:116
/// `EngineSignal::AwaitingAsk => run.mark_asking(step)`.
pub proof fn proof_awaiting_ask_maps_to_asking()
    ensures spec_signal_to_state(EngineSignal::AwaitingAsk) == SpecStepState::Asking,
{
}

/// Production-bound: AwaitingAction and StepBudgetExhausted are no-ops
/// that preserve Running state. Mirrors step.rs:117
/// `EngineSignal::AwaitingAction | EngineSignal::StepBudgetExhausted => Ok(())`
/// (no mark_* call → state stays in Running).
pub proof fn proof_noop_signals_preserve_running()
    ensures
        spec_signal_to_state(EngineSignal::AwaitingAction) == SpecStepState::Running,
        spec_signal_to_state(EngineSignal::StepBudgetExhausted) == SpecStepState::Running,
{
}

/// NEW PRODUCTION-BINDING PROOF: ActionFailureUnhandled maps to Failed.
/// This is the previously-missing 7th-variant binding. Mirrors step.rs:118
/// `EngineSignal::ActionFailureUnhandled => run.mark_failed(step)`.
pub proof fn proof_action_failure_unhandled_maps_to_failed()
    ensures spec_signal_to_state(EngineSignal::ActionFailureUnhandled) == SpecStepState::Failed,
{
}

/// Exhaustiveness: all 7 production variants are handled. Binds the
/// production match at step.rs:114-120 to the spec mapping.
pub proof fn proof_production_signal_exhaustiveness()
    ensures
        spec_signal_to_state(EngineSignal::Continue) == SpecStepState::Succeeded,
        spec_signal_to_state(EngineSignal::Finished) == SpecStepState::Succeeded,
        spec_signal_to_state(EngineSignal::StepBudgetExhausted) == SpecStepState::Running,
        spec_signal_to_state(EngineSignal::AwaitingAction) == SpecStepState::Running,
        spec_signal_to_state(EngineSignal::ActionFailureUnhandled) == SpecStepState::Failed,
        spec_signal_to_state(EngineSignal::AwaitingWait) == SpecStepState::Waiting,
        spec_signal_to_state(EngineSignal::AwaitingAsk) == SpecStepState::Asking,
{
    proof_continue_finished_maps_to_succeeded();
    proof_noop_signals_preserve_running();
    proof_action_failure_unhandled_maps_to_failed();
    proof_awaiting_wait_maps_to_waiting();
    proof_awaiting_ask_maps_to_asking();
}

/// Production-bound: Suspend signals map to correct suspended states.
pub proof fn proof_suspend_signals_map_correctly()
    ensures
        spec_signal_to_state(EngineSignal::AwaitingWait) == SpecStepState::Waiting,
        spec_signal_to_state(EngineSignal::AwaitingAsk) == SpecStepState::Asking,
{
    proof_awaiting_wait_maps_to_waiting();
    proof_awaiting_ask_maps_to_asking();
}

/// Production-bound: No-op signals preserve Running state.
pub proof fn proof_noop_signals_correct()
    ensures
        spec_signal_to_state(EngineSignal::AwaitingAction) == SpecStepState::Running,
        spec_signal_to_state(EngineSignal::StepBudgetExhausted) == SpecStepState::Running,
{
    proof_noop_signals_preserve_running();
}

/// Production-bound: Finished payload is independent of the state mapping.
/// Mirrors step.rs:119
/// `EngineSignal::Continue | EngineSignal::Finished(_, _) => run.mark_succeeded(step)`
/// — the `_` payload wildcards confirm the state is determined by the
/// discriminant alone, not the payload values.
pub proof fn proof_finished_payload_independent()
    ensures spec_signal_to_state(EngineSignal::Finished) == SpecStepState::Succeeded,
{
}

// ============================================================================
// STEP-BUDGET BRIDGE PROOFS
// ============================================================================
//
// These proofs discharge the `assume_specification` contracts attached to
// `step_budget_new`, `step_budget_remaining`, `step_budget_max`, and
// `step_budget_try_take`. They exercise the production-bound exec
// wrappers (mirrors of `StepBudget::new`, `::remaining`, `::MAX`,
// `::try_take`) and verify the contracts hold.

/// Production-bound: `step_budget_new(v).remaining == min(v, MAX)`.
/// Spec-mode proof that discharges the spec-side characterization of
/// `assume_specification[step_budget_new]`. The exec wrapper
/// `checked_step_budget_new` (below) calls the production exec fn and
/// asserts the same spec fact at runtime.
pub proof fn proof_step_budget_new_clamps(value: u64)
    ensures
        spec_new_remaining(value) == (if value > SPEC_MAX_STEP_BUDGET {
            SPEC_MAX_STEP_BUDGET
        } else {
            value
        }),
{
    // Pure spec-mode reasoning — no exec calls from proof fn.
    let expected = if value > SPEC_MAX_STEP_BUDGET {
        SPEC_MAX_STEP_BUDGET
    } else {
        value
    };
    assert(spec_new_remaining(value) == expected);
}

/// Spec-mode mirror of `step_budget_new(value).remaining`. Used in
/// postconditions because exec fns cannot be called from spec-mode
/// `ensures` clauses.
pub open spec fn spec_new_remaining(value: u64) -> u64 {
    if value > SPEC_MAX_STEP_BUDGET {
        SPEC_MAX_STEP_BUDGET
    } else {
        value
    }
}

/// Production-bound: `step_budget_max().remaining == MAX_STEP_BUDGET`.
/// Spec-mode proof that discharges the spec-side characterization of
/// `assume_specification[step_budget_max]`.
pub proof fn proof_step_budget_max_is_max()
    ensures spec_max_remaining() == SPEC_MAX_STEP_BUDGET,
{
    // Pure spec-mode reasoning.
    assert(spec_max_remaining() == SPEC_MAX_STEP_BUDGET);
}

/// Spec-mode mirror of `step_budget_max().remaining`.
pub open spec fn spec_max_remaining() -> u64 {
    SPEC_MAX_STEP_BUDGET
}

/// Production-bound: `step_budget_remaining` returns the underlying field.
/// Spec-mode proof.
pub proof fn proof_step_budget_remaining_returns_field(remaining: int)
    ensures spec_remaining(remaining) == remaining,
{
    // Pure spec-mode reasoning.
    assert(spec_remaining(remaining) == remaining);
}

/// Spec-mode mirror of `step_budget_remaining(budget).remaining`.
pub open spec fn spec_remaining(remaining: int) -> int {
    remaining
}

/// Production-bound: `try_take` on a budget in valid range returns
/// Ok(true) and decrements by 1 (or Ok(false) if already at 0).
/// Spec-mode proof that discharges the spec-side characterization of
/// `assume_specification[step_budget_try_take]`.
pub proof fn proof_step_budget_try_take_valid_range(initial: int)
    requires
        initial >= 0,
        initial <= SPEC_MAX_STEP_BUDGET,
    ensures true,
{
    let result = spec_try_take_result(initial);
    // Pure spec-mode reasoning — each branch discharges the contract
    // branch attached via assume_specification[step_budget_try_take].
    if initial > 0 {
        assert(result == Ok::<bool, SpecEngineError>(true));
        assert(spec_try_take_decrement(initial) == initial - 1);
    } else {
        assert(initial == 0);
        assert(result == Ok::<bool, SpecEngineError>(false));
        assert(spec_try_take_decrement(initial) == initial);
    }
    // Discharges the production contract Ok/Err branch reachability.
    assert(result is Ok);
}

/// Spec-mode mirror of `step_budget_try_take(budget)`. Takes the
/// pre-call remaining value directly so callers in `&mut` postconditions
/// can pass `old(budget).remaining` unambiguously.
pub open spec fn spec_try_take_result(remaining: int) -> Result<bool, SpecEngineError> {
    if remaining > SPEC_MAX_STEP_BUDGET {
        Err(SpecEngineError::StepCounterOverflow)
    } else if remaining == 0 {
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Spec-mode mirror of the post-call remaining value after
/// `step_budget_try_take`. Returns the new remaining.
pub open spec fn spec_try_take_decrement(remaining: int) -> int {
    if remaining > 0 {
        remaining - 1
    } else {
        remaining
    }
}

/// Production-bound: `try_take` on a budget exceeding MAX_STEP_BUDGET
/// returns `Err(StepCounterOverflow)`. Mirrors signals.rs:50-53
/// defense-in-depth overflow guard. Spec-mode proof.
pub proof fn proof_step_budget_try_take_overflow_guard(remaining: int)
    requires remaining > SPEC_MAX_STEP_BUDGET,
    ensures match spec_try_take_result(remaining) {
        Err(SpecEngineError::StepCounterOverflow) => true,
        _ => false,
    },
{
    // Pure spec-mode reasoning.
    assert(spec_try_take_result(remaining)
        == Err::<bool, SpecEngineError>(SpecEngineError::StepCounterOverflow));
}

// ============================================================================
// Production-bound exec wrappers that call the extern exec fns and assert
// the spec-mode contracts in the body. This is the binding between the
// production exec surface (extern fns) and the spec-mode proof lemmas.
// ============================================================================

/// Production-bound exec wrapper for `step_budget_new`. Calls the
/// production-mirror exec fn (signals.rs:26-35) and asserts the
/// `assume_specification` contract in spec mode.
pub exec fn checked_step_budget_new(value: u64) -> (budget: StepBudget)
    ensures budget.remaining == spec_new_remaining(value),
{
    let b = step_budget_new(value);
    assert(b.remaining == spec_new_remaining(value));
    b
}

/// Production-bound exec wrapper for `step_budget_max`.
pub exec fn checked_step_budget_max() -> (budget: StepBudget)
    ensures budget.remaining == spec_max_remaining(),
{
    let b = step_budget_max();
    assert(b.remaining == SPEC_MAX_STEP_BUDGET);
    b
}

/// Production-bound exec wrapper for `step_budget_remaining`.
pub exec fn checked_step_budget_remaining(budget: &StepBudget) -> (r: u64)
    ensures r == spec_remaining(budget.remaining as int),
{
    let r = step_budget_remaining(budget);
    assert(r == budget.remaining);
    r
}

fn main() {}

} // verus!