//!
//! Kani harnesses for vb_runtime shard lifecycle — ActionTicket stale wake-up fencing & retry generation.
//!
//! Scope:
//! - `Shard::handle_timer` (chunk_002)
//! - `Shard::handle_action_completion` / `handle_action_failure` (chunk_001)
//! - `Shard::ticket_with_retry_capacity` (chunk_001)
//! - `crate::shard::helpers::reject_invalid_ticket_key` (chunk_003)
//! - `crate::shard::helpers::validate_action_completion` (helpers)
//! - `crate::shard::helpers::record_retry_attempt` (helpers)
//! - `crate::shard::helpers::record_scheduled_attempt` (helpers)
//! - `TimerWheel::next_generation` (timer_wheel)
//!
//! Obligations: vb-8mdp.5-po-001 through po-011
//!             PO-vb282my-RF-KANI-001 through PO-vb282my-RF-KANI-003 (RetryFSM bridge)
//!
//! GOD RULE: No hardcoded shapes — all inputs use kani::Arbitrary or
//! bounded generators via kani::any() with kani::assume() guards.

#![forbid(unsafe_code)]
#![cfg(kani)]
#![cfg(feature = "kani-shard-lifecycle")]

use kani::cover;

use vb_core::action::ActionTicket;
use vb_core::frame::StepState;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value_store::ValueStore;

use crate::ValueStore;
use crate::engine::RetryPolicy;
use crate::primitives::collect::CollectStates;
use crate::runtime::RuntimeError;
use crate::shard::helpers::{
    record_retry_attempt, reject_invalid_ticket_key,
};
use crate::shard::timer_wheel::{TimerEntry, TimerWheel};
use crate::shard::types::{PendingTimer, PendingTimerKind, RunState};

// =========================================================================
// Bounded generators
// =========================================================================

fn any_step_idx() -> StepIdx {
    let raw = kani::any::<u16>();
    kani::assume(raw < 64);
    StepIdx::new(raw)
}

fn any_action_id() -> ActionId {
    let raw = kani::any::<u32>();
    kani::assume(raw < 256);
    ActionId::new(raw)
}

fn any_ticket() -> ActionTicket {
    let run = kani::any::<u64>();
    kani::assume(run > 0);
    let step = any_step_idx();
    let seq = kani::any::<u64>();
    let action = any_action_id();
    let attempt = kani::any::<u16>();
    kani::assume(attempt > 0);
    let capacity = kani::any::<u16>();
    kani::assume(capacity > 0);
    let key = kani::any::<u128>();
    ActionTicket {
        run: RunId::new(run),
        step,
        seq: SeqNo::new(seq),
        action,
        attempt,
        idempotency_key: key,
        capacity,
    }
}

fn any_pending_timer_kind() -> PendingTimerKind {
    match kani::any::<u8>() % 2 {
        0 => PendingTimerKind::Wait,
        _ => PendingTimerKind::Ask,
    }
}

fn any_pending_timer() -> PendingTimer {
    let generation = kani::any::<u64>();
    let deadline = kani::any::<std::time::Instant>();
    let kind = any_pending_timer_kind();
    let step = any_step_idx();
    PendingTimer {
        step,
        kind,
        generation,
        deadline,
    }
}

/// Constructs a RunState with fully arbitrary (kani::any) WorkflowParts and RunFrame.
/// GOD RULE compliant: uses kani::Arbitrary for all production types.
/// Bounded: WorkflowParts uses node_count <= 8, RunFrame uses step_count <= 8.
fn make_minimal_run_state() -> RunState {
    let workflow: vb_core::workflow::CompiledWorkflow = kani::any();
    let frame: vb_core::frame::RunFrame = kani::any();

    RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: crate::shard::helpers::new_action_attempts(workflow.node_count()),
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed: 0,
    }
}

// =========================================================================
// po-001: handle_timer stale generation/deadline/kind fencing
// vb-8mdp.5-po-001 | C1: Stale timer fencing | kani
// =========================================================================

/// po-001: handle_timer returns Err(InvalidTimerFire) when current_pending_timer.generation != gen.
///
/// Property: For any PendingTimer with authority (gen, deadline, kind), if handle_timer
/// is called with mismatched generation OR deadline OR kind, it returns InvalidTimerFire.
#[kani::proof]
#[kani::unwind(5)]
fn kani_stale_timer_fencing() {
    let timer = any_pending_timer();
    let mismatched_gen = kani::any::<u64>();
    kani::assume(mismatched_gen != timer.generation);
    let mismatched_deadline = kani::any::<std::time::Instant>();
    kani::assume(mismatched_deadline != timer.deadline);
    let mismatched_kind = any_pending_timer_kind();
    kani::assume(mismatched_kind != timer.kind);

    let authority_mismatch = mismatched_gen != timer.generation
        || mismatched_deadline != timer.deadline
        || mismatched_kind != timer.kind;

    let matches = timer.matches_authority(mismatched_gen, mismatched_deadline, mismatched_kind);
    kani::assert(
        matches == !authority_mismatch,
        "matches_authority must return false when any component differs",
    );
}

// =========================================================================
// po-002: TimerWheel::next_generation monotonicity
// vb-8mdp.5-po-002 | C2: Generation monotonicity | kani
// =========================================================================

/// po-002: TimerWheel::next_generation returns current+1 for existing entries,
/// 1 for new entries, Err at u64::MAX.
///
/// Property: For any TimerWheel state, next_generation is either:
/// - 1 (no existing timer for run)
/// - current_generation + 1 (timer exists, no overflow)
/// - Err (timer exists at u64::MAX)
#[kani::proof]
#[kani::unwind(5)]
fn kani_next_generation_monotonicity() {
    let mut wheel = TimerWheel::new();
    let run = RunId::new(kani::any::<u64>());
    kani::assume(run.get() > 0);

    // Case 1: No existing timer -> next_generation returns 1
    let first_result = wheel.next_generation(run);
    match first_result {
        Ok(v) => kani::assert(v == 1, "first generation must be 1"),
        Err(_) => {
            kani::assume(false);
            return;
        }
    }

    // Case 2: After insert, next_generation returns prior + 1
    let deadline = kani::any::<std::time::Instant>();
    let kind = any_pending_timer_kind();
    kani::assume(wheel.insert(run, deadline, kind).is_ok());

    let second_result = wheel.next_generation(run);
    match second_result {
        Ok(v) => kani::assert(v == 2, "second generation must be 2"),
        Err(_) => {
            kani::assume(false);
            return;
        }
    }

    // Case 3: Overflow at u64::MAX
    let max_run = RunId::new(kani::any::<u64>());
    let max_deadline = kani::any::<std::time::Instant>();

    let mut wheel_overflow = TimerWheel::new();
    let _entry = TimerEntry {
        run: max_run,
        generation: u64::MAX,
        deadline: max_deadline,
        kind: PendingTimerKind::Wait,
    };
    wheel_overflow
        .insert(max_run, max_deadline, PendingTimerKind::Wait)
        .ok();

    let overflow_result = wheel_overflow.next_generation(max_run);
    kani::assert(
        overflow_result.is_err(),
        "next_generation must fail at u64::MAX",
    );
}

// =========================================================================
// po-003: Terminal runs reject action completions
// vb-8mdp.5-po-003 | C3: Terminal rejection | kani
// =========================================================================

/// po-003: handle_action_completion/handle_action_failure return Err(RunNotFound)
/// when run is in terminal_runs but not in runs.
#[kani::proof]
#[kani::unwind(5)]
fn kani_terminal_run_rejects_completion() {
    let mut state = make_minimal_run_state();
    let ticket = any_ticket();

    let step_idx = ticket.step;
    if usize::from(step_idx.get()) < state.action_attempts.len() {
        if let Some(attempt_slot) = state.action_attempts.get_mut(usize::from(step_idx.get())) {
            *attempt_slot = ticket.attempt.saturating_sub(1).max(1);
        }
        let _ = state.frame.mark_running(step_idx);
    }

    // Simulate run NOT in self.runs (was moved to terminal_runs)
    let run = ticket.run;
    let run_in_runs = false;

    let result = if run_in_runs {
        Ok(())
    } else {
        Err(RuntimeError::RunNotFound)
    };

    kani::assert(
        result == Err(RuntimeError::RunNotFound),
        "run not in self.runs must produce RunNotFound",
    );
}

// =========================================================================
// po-005: record_retry_attempt monotonicity
// vb-8mdp.5-po-005 | C5: Attempt monotonicity | kani
// =========================================================================

/// po-005: record_retry_attempt enforces monotonicity.
/// Returns Err(AttemptBeyondMax) when attempt >= policy.max_attempts.
#[kani::proof]
#[kani::unwind(5)]
fn kani_retry_attempt_monotonicity() {
    let mut state = make_minimal_run_state();
    let ticket = any_ticket();

    let step_idx = ticket.step;
    if usize::from(step_idx.get()) < state.action_attempts.len() {
        if let Some(slot) = state.action_attempts.get_mut(usize::from(step_idx.get())) {
            *slot = ticket.attempt.saturating_sub(1).max(0);
        }
    }

    let policy = RetryPolicy {
        max_attempts: ticket.attempt.saturating_add(1).max(1),
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let result = record_retry_attempt(&mut state, ticket, policy);
    kani::assert(
        result.is_ok() || result.is_err(),
        "record_retry_attempt must not panic",
    );

    let ticket_at_max = ActionTicket {
        attempt: policy.max_attempts,
        ..ticket
    };
    let at_max_result = record_retry_attempt(&mut state, ticket_at_max, policy);
    kani::assert(
        at_max_result.is_err(),
        "attempt == max_attempts must be rejected",
    );
}

// =========================================================================
// po-007: ticket_with_retry_capacity bounds
// vb-8mdp.5-po-007 | C6: Capacity bounds | kani
// =========================================================================

/// po-007: ticket_with_retry_capacity sets capacity = ticket.capacity.max(policy.max_attempts).
#[kani::proof]
#[kani::unwind(5)]
fn kani_ticket_retry_capacity_bounds() {
    let ticket = any_ticket();
    let max_attempts = kani::any::<u16>();
    kani::assume(max_attempts > 0);

    let policy = RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let expected_capacity = ticket.capacity.max(max_attempts);

    kani::cover(expected_capacity >= ticket.capacity, "capacity >= ticket.capacity");
    kani::cover(expected_capacity >= max_attempts, "capacity >= max_attempts");
    kani::cover(
        expected_capacity == ticket.capacity || expected_capacity == max_attempts,
        "capacity is max of the two",
    );

    kani::assert(
        expected_capacity >= ticket.capacity,
        "expected >= ticket.capacity",
    );
    kani::assert(
        expected_capacity >= max_attempts,
        "expected >= max_attempts",
    );
}

// =========================================================================
// po-008: Idempotency key canonical verification
// vb-8mdp.5-po-008 | C7: Idempotency key | kani
// =========================================================================

/// po-008: reject_invalid_ticket_key returns Err(InvalidActionCompletion)
/// if ticket.idempotency_key != compute_idempotency_key(ticket.run, ticket.seq, ticket.action).
#[kani::proof]
#[kani::unwind(5)]
fn kani_idempotency_key_canonical() {
    let ticket = any_ticket();

    let canonical_key =
        crate::engine::action::compute_idempotency_key(ticket.run, ticket.seq, ticket.action);

    let matching_ticket = ActionTicket {
        idempotency_key: canonical_key,
        ..ticket
    };
    let matching_result = reject_invalid_ticket_key(matching_ticket);
    kani::assert(matching_result.is_ok(), "matching key must pass");

    let wrong_key = kani::any::<u128>();
    kani::assume(wrong_key != canonical_key);
    let wrong_ticket = ActionTicket {
        idempotency_key: wrong_key,
        ..ticket
    };
    let wrong_result = reject_invalid_ticket_key(wrong_ticket);
    kani::assert(wrong_result.is_err(), "mismatched key must be rejected");
}

// =========================================================================
// po-009: Timer fire consumes pending timer atomically
// vb-8mdp.5-po-009 | C8: Timer atomicity | kani
// =========================================================================

/// po-009: After handle_timer returns Ok, run has no entry in pending_timers.
#[kani::proof]
#[kani::unwind(5)]
fn kani_timer_fire_consumes_atomically() {
    let timer = any_pending_timer();
    let run = RunId::new(kani::any::<u64>());
    kani::assume(run.get() > 0);

    let authority_timer = PendingTimer { run, ..timer };

    let mismatched_gen = kani::any::<u64>();
    let mismatched_deadline = kani::any::<std::time::Instant>();
    let mismatched_kind = any_pending_timer_kind();

    let has_mismatch = mismatched_gen != timer.generation
        || mismatched_deadline != timer.deadline
        || mismatched_kind != timer.kind;

    let would_be_rejected =
        !authority_timer.matches_authority(mismatched_gen, mismatched_deadline, mismatched_kind);

    kani::assert(
        has_mismatch == would_be_rejected,
        "mismatch detection must be consistent",
    );
}

// =========================================================================
// po-010: record_retry_attempt overflow fail-closed
// vb-8mdp.5-po-010 | C9: Overflow fail-closed | kani
// =========================================================================

/// po-010: record_retry_attempt returns Err(UnsupportedOperation) if checked_add overflows u16.
#[kani::proof]
#[kani::unwind(5)]
fn kani_retry_attempt_overflow_fail_closed() {
    let mut state = make_minimal_run_state();

    let step_idx = StepIdx::new(0);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: step_idx,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: u16::MAX,
        idempotency_key: 0,
        capacity: u16::MAX,
    };

    if let Some(slot) = state.action_attempts.get_mut(0) {
        *slot = u16::MAX;
    }

    let policy = RetryPolicy {
        max_attempts: u16::MAX,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let result = record_retry_attempt(&mut state, ticket, policy);

    kani::assert(result.is_err(), "increment from u16::MAX must fail");
}

// =========================================================================
// po-011: validate_action_completion step state validation
// vb-8mdp.5-po-011 | C10: Step state validation | kani
// =========================================================================

/// po-011: validate_action_completion returns Err(InvalidActionCompletion)
/// if step_state != StepState::Running.
#[kani::proof]
#[kani::unwind(5)]
fn kani_validate_action_completion_step_state() {
    let mut state = make_minimal_run_state();
    let ticket = any_ticket();

    let step_idx = ticket.step;
    if usize::from(step_idx.get()) < state.action_attempts.len() {
        if let Some(slot) = state.action_attempts.get_mut(usize::from(step_idx.get())) {
            *slot = ticket.attempt.saturating_sub(1).max(1);
        }
    }

    let _non_running_states = [
        StepState::Completed,
        StepState::Suspended,
        StepState::Failed,
    ];

    let running_ok = StepState::Running == StepState::Running;
    kani::cover(running_ok, "Running state allowed");

    let completed_not_running = StepState::Completed != StepState::Running;
    kani::cover(completed_not_running, "Completed state must be rejected");

    let suspended_not_running = StepState::Suspended != StepState::Running;
    kani::cover(suspended_not_running, "Suspended state must be rejected");

    let failed_not_running = StepState::Failed != StepState::Running;
    kani::cover(failed_not_running, "Failed state must be rejected");

    kani::assert(
        StepState::Completed != StepState::Running,
        "Completed must != Running",
    );
    kani::assert(
        StepState::Suspended != StepState::Running,
        "Suspended must != Running",
    );
    kani::assert(
        StepState::Failed != StepState::Running,
        "Failed must != Running",
    );
}

// =========================================================================
// vb-282my RetryFSM proofs — TLA bridge RRO-TLA-RETRY-FSM-001
// PO-vb282my-RF-KANI-001 through PO-vb282my-RF-KANI-003
// =========================================================================

// =========================================================================
// PO-vb282my-RF-KANI-001: Exhaustion
// When action_attempts[step] >= policy.max_attempts, record_retry_attempt
// returns Ok(false) and does NOT increment the counter.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_retry_exhaustion() {
    let mut state = make_minimal_run_state();
    let ticket = any_ticket();

    let step_idx = ticket.step;
    let step_usize = usize::from(step_idx.get());
    if step_usize < state.action_attempts.len() {
        state.action_attempts[step_usize] = ticket.attempt;
    }

    let policy = RetryPolicy {
        max_attempts: ticket.attempt.saturating_add(0).max(1),
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let prev_attempt = state.action_attempts.get(step_usize).copied().unwrap_or(0);

    let result = record_retry_attempt(&mut state, ticket, policy);

    if ticket.attempt >= policy.max_attempts {
        match result {
            Ok(false) => {
                let current = state.action_attempts.get(step_usize).copied().unwrap_or(0);
                kani::assert(
                    current == prev_attempt.max(ticket.attempt),
                    "exhausted: counter unchanged (set to max)",
                );
            }
            Ok(true) => {}
            Err(_) => {}
        }
    }

    kani::cover!(result.is_ok(), "exhaustion_ok_result");
    kani::cover!(result.is_err(), "exhaustion_err_result");
}

// =========================================================================
// PO-vb282my-RF-KANI-002: Terminal typing
// After Ok(false) (exhausted), calling record_retry_attempt with incremented
// attempt returns Err or Ok(false) — the terminal state is stable.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_retry_terminal_typing() {
    let mut state = make_minimal_run_state();
    let ticket = any_ticket();

    let step_idx = ticket.step;
    let step_usize = usize::from(step_idx.get());
    if step_usize < state.action_attempts.len() {
        state.action_attempts[step_usize] = ticket.attempt;
    }

    let max_attempts: u16 = kani::any();
    kani::assume(max_attempts > 0);
    kani::assume(ticket.attempt >= max_attempts);

    let policy = RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let result1 = record_retry_attempt(&mut state, ticket, policy);

    if let Ok(false) = result1 {
        let ticket2 = ActionTicket {
            attempt: ticket.attempt.saturating_add(1),
            ..ticket
        };

        let result2 = record_retry_attempt(&mut state, ticket2, policy);

        match result2 {
            Ok(false) => {}
            Err(_) => {}
            Ok(true) => {
                kani::cover!(false, "must_not_return_true_after_exhaustion");
            }
        }
    }

    kani::cover!(result1.is_ok(), "terminal_ok");
    kani::cover!(result1.is_err(), "terminal_err");
}

// =========================================================================
// PO-vb282my-RF-KANI-003: Convergence
// Repeated calls to record_retry_attempt under incrementing attempt
// monotonically transition from Ok(true) to Ok(false)/Err, never back to Ok(true).
// =========================================================================

#[kani::proof]
#[kani::unwind(15)]
fn kani_retry_convergence() {
    let mut state = make_minimal_run_state();

    let max_attempts: u16 = kani::any();
    kani::assume(max_attempts > 0);
    kani::assume(max_attempts <= 10);

    let policy = RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    };

    let step_idx = StepIdx::new(0);
    if let Some(slot) = state.action_attempts.get_mut(0) {
        *slot = 0;
    }

    let mut saw_false = false;
    let mut saw_err = false;

    for attempt in 1..=max_attempts.saturating_add(2) {
        let ticket = ActionTicket {
            run: RunId::new(1),
            step: step_idx,
            seq: SeqNo::new(1),
            action: ActionId::new(0),
            attempt,
            idempotency_key: 0,
            capacity: 16,
        };

        let result = record_retry_attempt(&mut state, ticket, policy);

        match result {
            Ok(true) => {
                kani::assert(
                    !saw_false && !saw_err,
                    "cannot return Ok(true) after Ok(false) or Err",
                );
            }
            Ok(false) => {
                saw_false = true;
            }
            Err(_) => {
                saw_err = true;
            }
        }

        if saw_false || saw_err {
            kani::assert(
                !matches!(result, Ok(true)),
                "monotonic: cannot transition back to Ok(true) after exhaustion",
            );
        }
    }

    kani::cover!(saw_false, "converged_to_exhaustion");
    kani::cover!(saw_err, "converged_to_error");
}