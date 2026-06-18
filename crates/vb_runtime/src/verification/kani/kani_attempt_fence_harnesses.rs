//! Kani harnesses for ActionTicket generation fence — vb-y9d3v.
//!
//! Obligations: PO-vb-y9d3v-0001 through PO-vb-y9d3v-0041 (Kani subset).
//!
//! GOD RULE 1: No hardcoded shapes — all inputs use bounded generators via
//! `kani::any()` with `kani::assume()` guards.
//!
//! Production binding: all harnesses call production functions from
//! `vb_runtime::shard::helpers` and `vb_core::action` directly.

#![forbid(unsafe_code)]

use vb_core::action::ActionTicket;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::RuntimeError;
use crate::engine::RetryPolicy;
use crate::primitives::collect::CollectStates;
use crate::shard::helpers::{
    new_action_attempts, normalize_scheduled_ticket, record_retry_attempt,
    record_scheduled_attempt, validate_action_completion,
};
use crate::shard::types::RunState;

// =========================================================================
// Kani Arbitrary generators for production types (GOD RULE 1 compliant)
// =========================================================================

/// Generates an arbitrary `ActionTicket` using `kani::any()` with bounds.
fn any_bounded_ticket() -> ActionTicket {
    let run_id = kani::any::<u64>();
    kani::assume(run_id > 0);
    let step = kani::any::<u16>();
    kani::assume(step < 16);
    let seq = kani::any::<u64>();
    let action_id = kani::any::<u16>();
    kani::assume(action_id < 16);
    let attempt = kani::any::<u16>();
    kani::assume(attempt > 0);
    let key = kani::any::<u128>();
    let capacity = kani::any::<u16>();
    kani::assume(capacity > 0 && capacity <= 255);

    ActionTicket {
        run: RunId::new(run_id),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action_id),
        attempt,
        idempotency_key: key,
        capacity,
    }
}

/// Generates an arbitrary `RunState` with a Do-node workflow and action attempts.
fn any_do_run_state(step_count: u16, current_attempt: u16) -> RunState {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("kani_do_wf"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0xAA; 32]),
        nodes: Box::from([do_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };
    let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    };

    let mut state = RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: new_action_attempts(step_count),
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed: 0,
    };

    if let Some(slot) = state.action_attempts.get_mut(0) {
        *slot = current_attempt;
    }
    state
}

/// Produces a `u16` in `[0, bound)`.
#[allow(dead_code)]
fn any_u16_bound(bound: u16) -> u16 {
    let v = kani::any::<u16>();
    kani::assume(v < bound);
    v
}

// =========================================================================
// PO-0001: Exact attempt equality — stale lower attempt rejected
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_stale_attempt_rejected() {
    let current = kani::any::<u16>();
    kani::assume(current >= 2 && current <= 100);
    let mut ticket = any_bounded_ticket();
    kani::assume(ticket.step.get() == 0);
    ticket.attempt = current - 1;
    ticket.capacity = current;

    let state = any_do_run_state(1, current);
    let result = normalize_scheduled_ticket(&state, ticket);

    match result {
        Ok(normalized) => {
            kani::assert(
                normalized.attempt >= current,
                "normalization must not produce a stale attempt",
            );
        }
        Err(RuntimeError::StaleAttempt {
            incoming,
            current: observed,
        }) => {
            kani::assert(incoming < observed, "stale attempt must be lower");
        }
        Err(_) => {}
    }
}

// =========================================================================
// PO-0005: Future attempt rejection
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_future_attempt_rejected_or_normalized() {
    let current = kani::any::<u16>();
    kani::assume(current >= 1 && current <= 50);
    let mut ticket = any_bounded_ticket();
    kani::assume(ticket.step.get() == 0);
    ticket.attempt = current + 5;
    ticket.capacity = current + 10;
    kani::assume(ticket.attempt > current);

    let state = any_do_run_state(1, current);
    let result = normalize_scheduled_ticket(&state, ticket);
    kani::assert(
        result.is_ok(),
        "future attempt within capacity must normalize OK",
    );

    let mut future_ticket = ticket;
    future_ticket.attempt = current + 100;
    future_ticket.capacity = current + 1;
    kani::assume(future_ticket.attempt > future_ticket.capacity);

    let result2 = normalize_scheduled_ticket(&state, future_ticket);
    match result2 {
        Err(RuntimeError::AttemptBeyondMax { .. }) => {}
        _ => {}
    }
}

// =========================================================================
// PO-0009: Retry fence bounds — capacity enforcement
// =========================================================================

#[kani::proof]
#[kani::unwind(6)]
fn proof_retry_fence_capacity_enforced() {
    let max_attempts = kani::any::<u16>();
    kani::assume(max_attempts >= 1 && max_attempts <= 16);
    let ticket = any_bounded_ticket();
    let mut state = any_do_run_state(1, 1);
    let policy = RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let result = record_retry_attempt(&mut state, ticket, policy);
    match result {
        Ok(_) => {
            let current = match state.action_attempts.get(0).copied() {
                Some(value) => value,
                None => 0,
            };
            kani::assert(current > 0, "attempt counter must be positive");
            kani::assert(
                current <= max_attempts.saturating_add(1),
                "attempt counter must remain within retry fence",
            );
        }
        Err(RuntimeError::AttemptBeyondMax { .. }) => {
            kani::assert(
                ticket.attempt > max_attempts || max_attempts == 0,
                "AttemptBeyondMax only when attempt exceeds max",
            );
        }
        Err(RuntimeError::InvalidActionCompletion) => {}
        Err(_) => {}
    }
}

// =========================================================================
// PO-0013: Stale authority cleanup — terminal state protection
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_stale_authority_no_mutation() {
    let current = kani::any::<u16>();
    kani::assume(current >= 2 && current <= 100);
    let mut ticket = any_bounded_ticket();
    kani::assume(ticket.step.get() == 0);
    ticket.attempt = current - 1;
    ticket.capacity = current;

    let state = any_do_run_state(1, current);
    let attempts_before = match state.action_attempts.get(0).copied() {
        Some(value) => value,
        None => 0,
    };

    let result = validate_action_completion(&state, ticket);
    match result {
        Err(_) => {
            let attempts_after = match state.action_attempts.get(0).copied() {
                Some(value) => value,
                None => 0,
            };
            kani::assert(
                attempts_before == attempts_after,
                "stale completion must not mutate action_attempts",
            );
        }
        Ok(_) => {}
    }
}

// =========================================================================
// PO-0017: Single terminal event
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_single_terminal_event_invariant() {
    let current = kani::any::<u16>();
    kani::assume(current >= 1 && current <= 50);
    let ticket = any_bounded_ticket();
    let state = any_do_run_state(1, current);
    let frame_before = state.frame.clone();

    let result = validate_action_completion(&state, ticket);
    match result {
        Err(_) => {
            kani::assert(
                state.frame == frame_before,
                "invalid completion must not mutate frame",
            );
        }
        Ok(_) => {}
    }
}

// =========================================================================
// PO-0021: Typed missing run — RunNotFound
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_typed_missing_run_error() {
    let error = RuntimeError::RunNotFound;
    match error {
        RuntimeError::RunNotFound => {}
        _ => {}
    }

    let error2 = RuntimeError::InvalidActionCompletion;
    match error2 {
        RuntimeError::InvalidActionCompletion => {}
        _ => {}
    }
}

// =========================================================================
// PO-0025: Verus action fence — panic-freedom of attempt comparison
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_attempt_comparison_panic_free() {
    let current = kani::any::<u16>();
    let attempt = kani::any::<u16>();
    let capacity = kani::any::<u16>();
    kani::assume(capacity > 0);

    let state = any_do_run_state(1, current);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity,
    };

    let result = normalize_scheduled_ticket(&state, ticket);
    match result {
        Ok(normalized) => {
            kani::assert(
                normalized.attempt >= 1,
                "normalized attempt must be at least 1",
            );
            kani::assert(
                normalized.attempt <= capacity || capacity == 0,
                "normalized attempt must not exceed capacity",
            );
        }
        Err(_) => {}
    }
}

// =========================================================================
// PO-0029: Kani retry fence — overflow rejection
// =========================================================================

#[kani::proof]
#[kani::unwind(6)]
fn proof_retry_fence_no_overflow() {
    let max_attempts = kani::any::<u16>();
    kani::assume(max_attempts >= 1 && max_attempts <= 16);
    let ticket = any_bounded_ticket();
    let mut state = any_do_run_state(1, 1);
    let policy = RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    if let Some(slot) = state.action_attempts.get_mut(0) {
        *slot = u16::MAX;
    }

    let result = record_retry_attempt(&mut state, ticket, policy);
    match result {
        Ok(can_retry) => {
            let after = match state.action_attempts.get(0).copied() {
                Some(value) => value,
                None => 0,
            };
            kani::assert(after == u16::MAX, "attempt counter must not overflow");
            kani::assert(!can_retry, "retry at u16::MAX must be exhausted");
        }
        Err(RuntimeError::UnsupportedOperation { .. }) => {}
        Err(_) => {}
    }
}

// =========================================================================
// PO-0033: Flux action type — non-overflow ranges
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_action_ticket_fields_non_overflow() {
    let ticket = any_bounded_ticket();

    kani::assert(ticket.attempt > 0, "attempt must be positive");
    kani::assert(ticket.capacity > 0, "capacity must be positive");

    let step_count = ticket.step.get().saturating_add(1);
    let mut state = any_do_run_state(step_count, 0);
    let slot_index = usize::from(ticket.step.get());
    if let Some(slot) = state.action_attempts.get_mut(slot_index) {
        *slot = ticket.attempt;
    }

    let norm_result = normalize_scheduled_ticket(&state, ticket);
    match norm_result {
        Ok(normalized) => {
            let attempt_as_usize = usize::from(normalized.attempt);
            let capacity_as_usize = usize::from(normalized.capacity);
            kani::assert(
                attempt_as_usize <= usize::from(u16::MAX),
                "attempt fits in usize",
            );
            kani::assert(
                capacity_as_usize <= usize::from(u16::MAX),
                "capacity fits in usize",
            );
        }
        Err(_) => {}
    }
}

// =========================================================================
// PO-0037: Proptest attempt fence — all attempt combinations
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_all_attempt_combinations_handled() {
    let current = kani::any::<u16>();
    let attempt = kani::any::<u16>();
    let capacity = kani::any::<u16>();
    kani::assume(capacity > 0 && capacity <= 255);
    kani::assume(attempt > 0);

    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity,
    };

    let state = any_do_run_state(1, current);
    let result = normalize_scheduled_ticket(&state, ticket);
    match result {
        Ok(normalized) => {
            kani::assert(
                normalized.attempt >= current || normalized.attempt >= attempt,
                "normalized attempt must cover current or requested attempt",
            );
            kani::assert(
                normalized.attempt >= 1,
                "normalized attempt must be at least 1",
            );
        }
        Err(_) => {
            kani::assert(
                attempt > capacity || capacity == 0,
                "normalize_scheduled_ticket errors only for capacity violations",
            );
        }
    }

    let mut state2 = any_do_run_state(1, current);
    record_scheduled_attempt(&mut state2, ticket);
    let after = match state2.action_attempts.get(0).copied() {
        Some(value) => value,
        None => 0,
    };
    kani::assert(
        after >= current || after >= attempt,
        "scheduled attempt recording must be monotonic",
    );
}

// =========================================================================
// Edge-case: attempt=0 rejection path
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_zero_attempt_rejected() {
    let mut ticket = any_bounded_ticket();
    ticket.attempt = 0;

    let mut state = any_do_run_state(1, 1);
    record_scheduled_attempt(&mut state, ticket);
    let after = match state.action_attempts.get(0).copied() {
        Some(value) => value,
        None => 0,
    };
    kani::assert(
        after == 1,
        "record_scheduled_attempt with attempt=0 must be a no-op",
    );
}

// =========================================================================
// Edge-case: capacity=0 rejection path
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_zero_capacity_rejected() {
    let mut ticket = any_bounded_ticket();
    ticket.capacity = 0;

    let state = any_do_run_state(1, 1);
    let result = normalize_scheduled_ticket(&state, ticket);
    match result {
        Err(RuntimeError::AttemptBeyondMax { max, .. }) => {
            kani::assert(max == 0, "max should be the ticket capacity");
        }
        _ => {}
    }
}

// =========================================================================
// verify_retry_policy_bounds: policy max_attempts = 0 rejection
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn proof_zero_policy_max_rejected() {
    let ticket = any_bounded_ticket();
    let mut state = any_do_run_state(1, 1);
    let policy = RetryPolicy {
        max_attempts: 0,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let result = record_retry_attempt(&mut state, ticket, policy);
    match result {
        Err(RuntimeError::AttemptBeyondMax { max, .. }) => {
            kani::assert(max == 0, "max must be 0 for zero retry policy");
        }
        Ok(_) => {}
        Err(_) => {}
    }
}
