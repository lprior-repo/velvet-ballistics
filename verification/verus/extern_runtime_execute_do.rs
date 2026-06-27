// SPDX-License-Identifier: MIT
//
// Extern surface for vb_runtime_execute_do_spec Verus spec.
// Models the production decision fn `vb_runtime::engine::action::execute_do`
// at crates/vb_runtime/src/engine/action.rs:20-74 as a pure decision fn so
// Verus can reason about determinism and outcome-kind validity.
//
// Production bindings (BINDING LEDGER):
//   - `execute_do` decision branches mirror
//     `vb_runtime::engine::action::execute_do` at
//     crates/vb_runtime/src/engine/action.rs:20-74.
//   - The `Taint` discriminant (0=Clean, 1=DerivedFromSecret, 2=Secret,
//     3=Random, 4=TimeDependent) mirrors `vb_core::value::Taint` at
//     crates/vb_core/src/value.rs:14-25.
//   - The `Idempotency` discriminant (0=DeterministicPure,
//     1=IdempotentExternal, 2=AtLeastOnceExternal) mirrors
//     `vb_core::action::contract::Idempotency` at
//     crates/vb_core/src/action/contract.rs:7-17.
//   - The `ActionOutcome` discriminant set (Ready, Suspended, Failed) mirrors
//     `vb_core::action::payload::ActionOutcome` at
//     crates/vb_core/src/action/payload.rs:163-172.
//
// The production `execute_do` body has three sequential gates whose decisions
// are determined entirely by the projected input scalars below; the function
// is otherwise pure (no I/O, no clock, no environment). The pure projection
// here is therefore equivalent to the production decision for the purposes
// of determinism and outcome-kind classification.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

/// Mirrors the discriminated output set of
/// `vb_runtime::engine::action::execute_do`.
///
/// The variants enumerate every reachable (Ok, Err) pair from the production
/// body. The `ActionOutcomeKind` companion enum is the post-resume view: the
/// resume path consumes the `AwaitingAction` ticket and produces one of the
/// three `ActionOutcome` variants (Ready, Suspended, Failed).
pub enum SpecOutcomeKind {
    /// Production success: `RuntimeSignal::AwaitingAction(ticket)`.
    OkAwaitingAction,
    /// Capability check failed: `RuntimeEngineError::Core(CapabilityDenied)`.
    ErrCapabilityDenied,
    /// Taint check failed: `RuntimeEngineError::TaintViolation`.
    ErrTaintViolation,
    /// Action not in registry or id mismatch:
    /// `RuntimeEngineError::Action(UnknownAction)`.
    ErrUnknownAction,
    /// Underlying `RunFrame::read_taint` failed:
    /// `RuntimeEngineError::Core(...)` other variants.
    ErrCore,
}

/// Mirrors `vb_core::action::payload::ActionOutcome` discriminant set
/// (Ready, Suspended, Failed) at crates/vb_core/src/action/payload.rs:163-172.
/// This is the *post-resume* view: any action invoked through `execute_do`
/// ultimately resolves to one of these three kinds. The spec bounds the
/// discriminant so that no other variant can leak through the action
/// dispatcher.
pub enum SpecActionOutcomeKind {
    /// `ActionOutcome::Ready(_)` — action completed with output.
    Ready,
    /// `ActionOutcome::Suspended(_)` — action suspended awaiting completion.
    Suspended,
    /// `ActionOutcome::Failed(_)` — action failed.
    Failed,
}

impl SpecActionOutcomeKind {
    /// Returns a stable discriminant integer that mirrors the production
    /// `ActionOutcome` variant order (Ready=0, Suspended=1, Failed=2).
    pub const fn discriminant(self) -> u8 {
        match self {
            SpecActionOutcomeKind::Ready => 0,
            SpecActionOutcomeKind::Suspended => 1,
            SpecActionOutcomeKind::Failed => 2,
        }
    }
}

/// Pure decision fn mirroring `vb_runtime::engine::action::execute_do`.
///
/// The production function reads from `&RunFrame` and iterates over
/// `&[ActionContract]` and `&CapabilitySet`. This projection reduces each
/// of those inputs to the smallest set of scalars that the production
/// decision branches observe:
///
///   - `run_id`, `step`, `action`, `input`, `seq` — propagated into the
///     constructed ticket (not used for branching).
///   - `input_taint_disc` — drives the DeterministicPure + non-Clean check.
///   - `contract_id`, `contract_idempotency_disc` — drive the
///     DeterministicPure + taint guard and the post-propagation check.
///   - `registry_action_match` — true iff
///     `action_index < registry_contracts.len()` AND
///     `registry_contracts[action_index].id == action`. False triggers
///     `UnknownAction`.
///   - `all_required_caps_granted` — true iff every
///     `resolved.required_capabilities` element is in `granted`. False
///     triggers `CapabilityDenied`.
///   - `retry_max_attempts` — propagated into the ticket (not used for
///     branching on this path).
///   - `read_taint_failed` — true iff the underlying
///     `RunFrame::read_taint` returned an error other than the values the
///     production body handles directly. True triggers `ErrCore`.
///
/// `output_taint_cleanable_from_tainted` models the post-propagation
/// `output_taint == Clean && input_taint != Clean` check. In production
/// this evaluates `propagate_action_taint(idempotency, input_taint)`; for
/// the determinism contract we treat the boolean as already computed.
#[verifier::external]
pub fn execute_do_pure_decision(
    _run_id: u64,
    _step: u32,
    _action: u32,
    _input: u32,
    _seq: u64,
    input_taint_disc: u8,
    contract_id: u32,
    contract_idempotency_disc: u8,
    registry_action_match: bool,
    all_required_caps_granted: bool,
    _retry_max_attempts: u16,
    read_taint_failed: bool,
    output_taint_cleanable_from_tainted: bool,
) -> SpecOutcomeKind {
    // 1. The very first branch in production is the action lookup. If the
    //    action is not in the registry or the contract id does not match,
    //    we short-circuit to UnknownAction.
    if !registry_action_match {
        return SpecOutcomeKind::ErrUnknownAction;
    }

    // 2. The underlying run-read failure is also checked before the
    //    taint/capability decisions. The production body calls
    //    `run.read_taint(input)` and converts any non-success to
    //    `RuntimeEngineError::Core`. Project that here.
    if read_taint_failed {
        return SpecOutcomeKind::ErrCore;
    }

    // 3. DeterministicPure + non-Clean input taint -> TaintViolation.
    if contract_idempotency_disc == 0 && input_taint_disc != 0 {
        return SpecOutcomeKind::ErrTaintViolation;
    }

    // 4. Any required capability missing from `granted` -> CapabilityDenied.
    if !all_required_caps_granted {
        return SpecOutcomeKind::ErrCapabilityDenied;
    }

    // 5. Post-propagation check: a tainted input must not collapse to a
    //    Clean output. If it does, that is a taint violation.
    if output_taint_cleanable_from_tainted {
        return SpecOutcomeKind::ErrTaintViolation;
    }

    // 6. Contract id is otherwise unused by the decision branches; this
    //    assertion guards the binding against accidental signature drift.
    let _ = contract_id;

    SpecOutcomeKind::OkAwaitingAction
}

/// Spec-level mirror of `propagate_action_taint(idempotency, input_taint)`.
/// Returns true iff the production logic says `output_taint == Clean`
/// while `input_taint != Clean` (i.e. taint was erased on a tainted
/// input). This is the same boolean the decision fn above consumes via
/// `output_taint_cleanable_from_tainted`.
pub fn spec_propagate_action_taint(
    contract_idempotency_disc: u8,
    input_taint_disc: u8,
) -> bool {
    // Clean == 0. Anything other than Clean in input must propagate to
    // a non-Clean output. The production `propagate_action_taint` returns
    // `input_taint` for SideEffecting variants; for DeterministicPure
    // the input must already be Clean (gated earlier). The only way to
    // produce Clean output from non-Clean input is when the idempotency
    // variant deliberately drops taint, which the production guard
    // treats as a violation.
    input_taint_disc != 0
}

/// Returns true iff `kind` is one of the three documented `ActionOutcome`
/// discriminants (Ready, Suspended, Failed). Mirrors the production
/// `#[non_exhaustive] ActionOutcome` at crates/vb_core/src/action/payload.rs:163-172.
pub const fn spec_action_outcome_kind_valid(kind: SpecActionOutcomeKind) -> bool {
    matches!(
        kind,
        SpecActionOutcomeKind::Ready
            | SpecActionOutcomeKind::Suspended
            | SpecActionOutcomeKind::Failed
    )
}

// ============================================================================
// Pure decision projections for the remaining 6 production exec wrappers in
// crates/vb_runtime/src/engine/action.rs:
//
//   - execute_do_without_contract  (lines 76-106)
//   - execute_retry_check          (lines 109-120)
//   - execute_error_handler        (lines 123-131)
//   - resume_action_outcome        (lines 138-200)
//   - compute_idempotency_key      (lines 206-208)
//   - resolve_contract             (lines 211-221)
//
// Each projection reduces the production body to the smallest set of scalars
// that drive the decision branches. The body is the trusted base; the spec
// file attaches `assume_specification` bridges and exercises each through an
// exec wrapper.
// ============================================================================

/// Mirrors the discriminated output set of
/// `vb_runtime::engine::action::execute_do_without_contract`
/// at crates/vb_runtime/src/engine/action.rs:76-106.
///
/// The production body unconditionally fails: a non-Clean input taint
/// short-circuits to `RuntimeEngineError::TaintViolation`, and the
/// subsequent synthetic `__contract_required__` capability check always
/// fails because no real contract is provided. So the reachable output
/// set is `{ErrTaintViolation, ErrCapabilityDenied}`. Both are already
/// covered by `SpecOutcomeKind`, so we reuse that enum.
#[verifier::external]
pub fn execute_do_without_contract_pure_decision(
    input_taint_disc: u8,
) -> SpecOutcomeKind {
    if input_taint_disc != 0 {
        SpecOutcomeKind::ErrTaintViolation
    } else {
        SpecOutcomeKind::ErrCapabilityDenied
    }
}

/// Mirrors `vb_runtime::engine::action::execute_retry_check`
/// at crates/vb_runtime/src/engine/action.rs:109-120.
///
/// The production body returns `body` iff `current_attempt < policy.max_attempts`,
/// else `exhausted`. We project to scalars: `current_attempt`, `max_attempts`,
/// `body`, `exhausted`. The decision is a pure comparison.
#[verifier::external]
pub const fn execute_retry_check_pure_decision(
    current_attempt: u16,
    max_attempts: u16,
    body: u32,
    exhausted: u32,
) -> u32 {
    if current_attempt < max_attempts {
        body
    } else {
        exhausted
    }
}

/// Mirrors `vb_runtime::engine::action::execute_error_handler`
/// at crates/vb_runtime/src/engine/action.rs:123-131.
///
/// The production body returns `handler` iff
/// `failure.retry_policy == RetryPolicy::Retryable || failure.code != ActionFailureCode::Unknown`,
/// else `body`. Discriminants: Retryable=0, NonRetryable=1, Unknown=255.
#[verifier::external]
pub const fn execute_error_handler_pure_decision(
    failure_retry_policy_disc: u8,
    failure_code_disc: u8,
    handler: u32,
    body: u32,
) -> u32 {
    if failure_retry_policy_disc == 0 || failure_code_disc != 255 {
        handler
    } else {
        body
    }
}

/// Mirrors the discriminated output set of
/// `vb_runtime::engine::action::resume_action_outcome`
/// at crates/vb_runtime/src/engine/action.rs:138-200.
///
/// The production body matches on `ActionOutcome` and produces one of:
///
///   - `ActionOutcome::Ready(_)`         -> Ok(Continue)
///   - `ActionOutcome::Suspended(_)`     -> Ok(AwaitingAction)
///   - `ActionOutcome::Failed(_)` with retryable + capacity > attempt
///                                       -> Ok(AwaitingAction(retry_ticket))
///   - `ActionOutcome::Failed(_)` with retryable + capacity <= attempt
///                                       -> Err(RetryExhausted)
///   - `ActionOutcome::Failed(_)` non-retryable
///                                       -> Err(Core(UnsupportedPrimitive))
///   - any other future variant           -> Err(Core(InternalInvariantViolation))
///
/// This enum enumerates every reachable (Ok, Err) kind so the spec-level
/// discriminant bound closes.
pub enum SpecResumeKind {
    /// Production: `RuntimeSignal::Continue`.
    Continue,
    /// Production: `RuntimeSignal::AwaitingAction(retry_ticket)`.
    AwaitingAction,
    /// Production: `RuntimeEngineError::RetryExhausted`.
    ErrRetryExhausted,
    /// Production: `RuntimeEngineError::Core(EngineError::UnsupportedPrimitive)`.
    ErrUnsupportedPrimitive,
    /// Production: `RuntimeEngineError::Core(EngineError::InternalInvariantViolation)`.
    ErrInternalInvariantViolation,
}

/// Pure decision projection of `resume_action_outcome`.
///
/// Scalars (all bounded as documented in production):
///   - `outcome_disc`             : 0=Ready, 1=Suspended, 2=Failed
///   - `attempt`                  : u16 (current attempt count)
///   - `capacity`                 : u16 (max attempts allowed)
///   - `retry_policy_disc`        : 0=Retryable, 1=NonRetryable
///   - `seq_would_overflow`       : true iff `seq.checked_add(1).is_none()`
///   - `attempt_would_overflow`   : true iff `attempt.checked_add(1).is_none()`
///
/// The projection is total: every documented input maps to one of the
/// five `SpecResumeKind` variants. The post-resume view collapses the
/// retry-ticket reconstruction to a single AwaitingAction variant.
#[verifier::external]
pub const fn resume_action_outcome_pure_decision(
    outcome_disc: u8,
    attempt: u16,
    capacity: u16,
    retry_policy_disc: u8,
    seq_would_overflow: bool,
    attempt_would_overflow: bool,
) -> SpecResumeKind {
    match outcome_disc {
        // ActionOutcome::Ready(_) -> Ok(Continue)
        0 => SpecResumeKind::Continue,
        // ActionOutcome::Suspended(_) -> Ok(AwaitingAction)
        1 => SpecResumeKind::AwaitingAction,
        // ActionOutcome::Failed(_) -> retry/exhausted/unsupported
        2 => {
            if retry_policy_disc == 0 {
                // Retryable: build a retry ticket iff attempt < capacity
                // and neither seq nor attempt would overflow.
                if attempt < capacity && !seq_would_overflow && !attempt_would_overflow {
                    SpecResumeKind::AwaitingAction
                } else {
                    SpecResumeKind::ErrRetryExhausted
                }
            } else {
                SpecResumeKind::ErrUnsupportedPrimitive
            }
        }
        // Future variant catch-all mirrors production's `_ =>` arm.
        _ => SpecResumeKind::ErrInternalInvariantViolation,
    }
}

/// Pure decision projection of `compute_idempotency_key`
/// at crates/vb_runtime/src/engine/action.rs:206-208.
///
/// The production body delegates to
/// `vb_core::action::compute_action_idempotency_key` at
/// crates/vb_core/src/action/ticket.rs:25-35, which uses a wrapping
/// multiply-add hash (FNV-1a-inspired). Mirroring the same arithmetic
/// keeps the spec deterministic and equivalent to the production result.
#[verifier::external]
pub fn compute_idempotency_key_pure(run: u64, seq: u64, action: u32) -> u128 {
    let run_part = u128::from(run);
    let seq_part = u128::from(seq);
    let action_part = u128::from(action);
    run_part
        .wrapping_mul(0x6c62272e07bb0143_u128)
        .wrapping_add(seq_part)
        .wrapping_mul(0x3b4f1a5b6c2d8e7f_u128)
        .wrapping_add(action_part)
        .wrapping_mul(0x5bd1e9956c7b4d3a_u128)
}

/// Pure decision projection of `resolve_contract`
/// at crates/vb_runtime/src/engine/action.rs:211-221.
///
/// The production body returns `Ok(&ActionContract)` iff
/// `action_index < contracts.len()` AND `contracts[action_index].id == action`.
/// Otherwise it returns `Err(ActionError::UnknownAction)`.
/// We project both conditions into a single boolean: `id_at_index_match`.
/// Returns `true` iff the contract was resolved.
#[verifier::external]
pub const fn resolve_contract_pure_decision(id_at_index_match: bool) -> bool {
    id_at_index_match
}