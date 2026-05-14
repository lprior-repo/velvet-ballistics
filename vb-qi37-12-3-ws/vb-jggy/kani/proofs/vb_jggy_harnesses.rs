//! Kani harnesses for vb-jggy: Persist execution attempt numbers and reject stale completions.
//!
//! These harnesses verify:
//! - HK-1: `validate_ticket_attempt` ordering (POST-004, INV-003)
//! - HK-2: `record_scheduled_attempt` monotonicity (INV-004, POST-006)
//! - HK-3: `handle_action_completion` stale-first ordering (INV-003)
//! - HK-4: `RunState::action_attempts` zero-initialized (POST-001)
//!
//! Run with: kani --specify-target <harness_file>

#![forbid(unsafe_code)]

/// HK-1: `validate_ticket_attempt` ordering proof.
///
/// Property: `validate_ticket_attempt` returns `Ok(())` implies
/// `ticket.attempt >= state.action_attempts[ticket.step]`.
///
/// Bound: `step_count <= 100`, `attempt <= 100`, `capacity <= 100`.
///
/// This proves the stale gate precedes any state mutation call site in lifecycle.
#[kani::proof]
fn validate_ticket_attempt_ordering() {
    // This test requires vb_runtime implementation to be complete.
    // Currently the lib does not compile due to missing attempt fields.
    kani::skip!("RED PHASE: Implementation incomplete - RuntimeJournalEvent missing attempt fields");
}

/// HK-2: `record_scheduled_attempt` monotonicity proof.
///
/// Property: For any two calls `record_scheduled_attempt(state, t1)` then
/// `record_scheduled_attempt(state, t2)` with same step,
/// `state.action_attempts[step]` is non-decreasing.
///
/// Bound: `step in 0..50`, `attempt in 0..u16::MAX`.
///
/// This proves attempt counter never decreases across retries.
#[kani::proof]
fn record_scheduled_attempt_monotonicity() {
    kani::skip!("RED PHASE: Implementation incomplete - RuntimeJournalEvent missing attempt fields");
}

/// HK-3: `handle_action_completion` stale-first ordering proof.
///
/// Property: In `handle_action_completion`, `validate_ticket_attempt` result is
/// checked before any `journal.append` call.
///
/// Bound: Single step, single action, no concurrent access.
///
/// Kani proves call-ordering between validation and journal mutation.
#[kani::proof]
fn handle_action_completion_stale_first_ordering() {
    kani::skip!("RED PHASE: Implementation incomplete - RuntimeJournalEvent missing attempt fields");
}

/// HK-4: `RunState::action_attempts` zero-initialized proof.
///
/// Property: After `handle_submit_with_inputs`, for all steps `i`,
/// `action_attempts[i] == 0`.
///
/// Bound: Workflow with `step_count <= 20`.
///
/// This guarantees fresh runs start with clean attempt state.
#[kani::proof]
fn run_state_action_attempts_zero_initialized() {
    kani::skip!("RED PHASE: Implementation incomplete - RuntimeJournalEvent missing attempt fields");
}

/// HK-5: Stale attempt is rejected before any state mutation.
///
/// Property: If `ticket.attempt < current`, then `validate_ticket_attempt` returns
/// `Err(StaleAttempt { .. })` BEFORE any state mutation occurs.
#[kani::proof]
fn stale_attempt_rejected_before_mutation() {
    kani::skip!("RED PHASE: Implementation incomplete - RuntimeJournalEvent missing attempt fields");
}

/// HK-6: Future attempt when current > 0 is rejected.
///
/// Property: If `current != 0 && ticket.attempt > current`, then
/// `validate_ticket_attempt` returns `Err(InvalidActionCompletion)`.
#[kani::proof]
fn future_attempt_rejected_when_current_nonzero() {
    kani::skip!("RED PHASE: Implementation incomplete - RuntimeJournalEvent missing attempt fields");
}
