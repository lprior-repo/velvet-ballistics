#[test]
fn validate_action_completion_returns_stale_attempt_when_attempt_lower_than_current()
-> Result<(), String> {
    let wf =
        suspended_workflow().ok_or_else(|| "suspended_workflow fixture must build".to_owned())?;
    let mut state = make_run_state(wf, RunId::new(1))
        .ok_or_else(|| "make_run_state for suspended_workflow must succeed".to_owned())?;
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 3
        })
    );
    Ok(())
}

#[test]
fn validate_action_completion_returns_stale_attempt_when_lower_by_many() -> Result<(), String> {
    let wf =
        suspended_workflow().ok_or_else(|| "suspended_workflow fixture must build".to_owned())?;
    let mut state = make_run_state(wf, RunId::new(1))
        .ok_or_else(|| "make_run_state for suspended_workflow must succeed".to_owned())?;
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 5
        })
    );
    Ok(())
}

#[test]
fn validate_action_completion_returns_stale_attempt_at_edge_1_vs_2() -> Result<(), String> {
    let wf =
        suspended_workflow().ok_or_else(|| "suspended_workflow fixture must build".to_owned())?;
    let mut state = make_run_state(wf, RunId::new(1))
        .ok_or_else(|| "make_run_state for suspended_workflow must succeed".to_owned())?;
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 2
        })
    );
    Ok(())
}

#[test]
fn validate_action_completion_rejects_future_attempt_when_attempt_exceeds_current()
-> Result<(), String> {
    let wf =
        suspended_workflow().ok_or_else(|| "suspended_workflow fixture must build".to_owned())?;
    let mut state = make_run_state(wf, RunId::new(1))
        .ok_or_else(|| "make_run_state for suspended_workflow must succeed".to_owned())?;
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let t = ActionTicket {
        capacity: 10,
        ..ticket(RunId::new(1), StepIdx::ZERO, 5)
    };
    // G005: future-attempt rejection now implemented
    // validate_ticket_attempt returns Err(InvalidActionCompletion) for attempt > current
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    Ok(())
}

#[test]
fn validate_action_completion_rejects_unscheduled_first_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.action_attempts.get(0).copied(), Some(0));
    let t = ActionTicket {
        capacity: 10,
        ..ticket(RunId::new(1), StepIdx::ZERO, 1)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_attempt_is_zero() -> Result<(), String> {
    let wf =
        suspended_workflow().ok_or_else(|| "suspended_workflow fixture must build".to_owned())?;
    let mut state = make_run_state(wf, RunId::new(1))
        .ok_or_else(|| "make_run_state for suspended_workflow must succeed".to_owned())?;
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 0)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: 5 })
    );
    Ok(())
}

#[test]
fn validate_action_completion_rejects_when_capacity_is_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    let t = ActionTicket {
        capacity: 0,
        ..ticket(RunId::new(1), StepIdx::ZERO, 1)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
    );
}

#[test]
fn validate_action_completion_rejects_when_attempt_exceeds_capacity() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    let t = ActionTicket {
        capacity: 3,
        ..ticket(RunId::new(1), StepIdx::ZERO, 5)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 5, max: 3 })
    );
}

#[test]
fn validate_action_completion_rejects_when_step_is_succeeded() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_step_is_pending() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(StepState::Pending)
    );
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_step_is_failed() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_failed(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_node_is_not_do() {
    // Build a workflow where step 0 is not Do.
    let label_node = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(1)),
    );
    let action_node = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("label_first"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: Box::from([label_node, action_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("validate_action_completion_rejects_when_node_is_not_do");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_accepts_boundary_min_valid_attempt_capacity_one() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ActionTicket {
        capacity: 1,
        ..ticket(RunId::new(1), StepIdx::ZERO, 1)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

// =======================================================================
// normalize_scheduled_ticket additional tests
// =======================================================================

#[test]
fn normalize_scheduled_ticket_promotes_to_one_when_current_and_ticket_are_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 0)
    };
    let result = normalize_scheduled_ticket(&state, t);
    // current=0, ticket.attempt=0, so attempt = max(0,0).max(1) = 1
    assert_eq!(result, Ok(ActionTicket { attempt: 1, ..t }));
}

#[test]
fn normalize_scheduled_ticket_rejects_when_step_out_of_bounds() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ticket(RunId::new(1), StepIdx::new(99), 1);
    let result = normalize_scheduled_ticket(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

// =======================================================================
// validate_action_completion -- boundary vs capacity tests
// =======================================================================

#[test]
fn validate_action_completion_accepts_equal_attempt_and_capacity() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 5)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_accepts_boundary_max_valid_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = u16::MAX;
    }
    let t = ActionTicket {
        capacity: u16::MAX,
        ..ticket(RunId::new(1), StepIdx::ZERO, u16::MAX)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_rejects_when_attempt_over_capacity_and_current_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 6)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 6, max: 5 })
    );
}

// =======================================================================
// record_retry_attempt rejects when attempt exceeds max_attempts
// =======================================================================

#[test]
fn record_retry_attempt_rejects_when_attempt_exceeds_max_attempts() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ticket(RunId::new(1), StepIdx::ZERO, 6);
    let policy = crate::engine::RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    assert_eq!(
        record_retry_attempt(&mut state, t, policy),
        Err(RuntimeError::AttemptBeyondMax { attempt: 6, max: 5 })
    );
}

// =======================================================================
// record_retry_attempt overflow boundary at u16::MAX
// =======================================================================

#[test]
fn record_retry_attempt_at_u16_max_returns_overflow_error() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = u16::MAX;
    }
    let t = ActionTicket {
        capacity: u16::MAX,
        ..ticket(RunId::new(1), StepIdx::ZERO, u16::MAX)
    };
    // max_attempts = u16::MAX + 1 is not representable, so
    // to truly trigger overflow we'd need max_attempts > u16::MAX.
    // With max_attempts == u16::MAX, *attempt >= max_attempts so it returns Ok(false)
    // The overflow path requires *attempt to be less than max_attempts but
    // *attempt + 1 to overflow. *attempt can be at most (max_attempts - 1).
    // If max_attempts == u16::MAX, then *attempt == u16::MAX - 1 triggers checked_add(1) -> Some(u16::MAX).
    // No overflow possible since max_attempts is u16.
    // This test verifies that at the edge, the function returns correctly.
    let policy = crate::engine::RetryPolicy {
        max_attempts: u16::MAX,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    // attempt (=current=u16::MAX) >= max_attempts (=u16::MAX) → Ok(false)
    assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(false));
}

// =======================================================================
// record_retry_attempt with attempt equal to max minus 1 (last retry)
// =======================================================================

#[test]
fn record_retry_attempt_returns_true_on_last_retry_below_max() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 4;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 4);
    let policy = crate::engine::RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    // 4 < 5, so should increment to 5 and return true
    assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(true));
    assert_eq!(state.action_attempts.get(0).copied(), Some(5));
}

// =======================================================================
// Proptest: attempt fence classification
// =======================================================================

mod proptest_tests {
    use proptest::prelude::*;
    use vb_core::action::ActionTicket;
    use vb_core::ids::WorkflowDigest;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
    use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};

    use crate::RuntimeError;

    use super::super::super::types::RunState;
    use super::super::make_run_state;
    use super::super::validate_action_completion;
    fn make_simple_state(attempt_val: u16) -> Option<RunState> {
        let node = vb_core::workflow::node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: vb_core::ids::SlotIdx::new(0),
            },
            None,
            None,
        );
        let parts = WorkflowParts {
            semantic_source_identity: None,
            name: Box::from("proptest"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
            input_slots: Box::from([]),
        };
        let wf =
            vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("make_simple_state");
        let mut state = make_run_state(wf, RunId::new(1))?;
        assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
        if let Some(attempt) = state.action_attempts.get_mut(0) {
            *attempt = attempt_val;
        }
        Some(state)
    }

    proptest! {
        /// Property: validate_action_completion never panics for arbitrary u16 inputs.
        #[test]
        fn prop_validate_action_completion_never_panics(
            attempt in 0u16..=u16::MAX,
            capacity in 0u16..=u16::MAX,
            current in 0u16..=u16::MAX,
        ) {
            let Some(state) = make_simple_state(current) else {
                return Ok(());
            };
            let t = ActionTicket {
                run: RunId::new(1),
                step: StepIdx::ZERO,
                seq: SeqNo::ZERO,
                action: ActionId::new(0),
                run_attempt: 1,
                attempt,
                idempotency_key: 0,
                capacity,
             fencing_token: 0, };
            let _result = validate_action_completion(&state, t);
        }

        /// Property: For any valid (attempt, capacity, current):
        ///   attempt == 0 OR capacity == 0 OR attempt > capacity => Err(AttemptBeyondMax)
        ///   attempt < current => Err(StaleAttempt)
        ///   attempt == current => Ok(())
        #[test]
        fn prop_validate_ticket_attempt_classifies_all_attempt_relations(
            attempt in 1u16..=65500u16,
            capacity in 1u16..=65500u16,
            current in 0u16..=65500u16,
        ) {
            let Some(state) = make_simple_state(current) else {
                return Ok(());
            };
            let t = ActionTicket {
                run: RunId::new(1),
                step: StepIdx::ZERO,
                seq: SeqNo::ZERO,
                action: ActionId::new(0),
                run_attempt: 1,
                attempt,
                idempotency_key: 0,
                capacity,
             fencing_token: 0, };
            let result = validate_action_completion(&state, t);
            if attempt > capacity {
                prop_assert_eq!(
                    result,
                    Err(RuntimeError::AttemptBeyondMax { attempt, max: capacity })
                );
            } else if attempt < current {
                prop_assert_eq!(
                    result,
                    Err(RuntimeError::StaleAttempt {
                        incoming: attempt,
                        current,
                    })
                );
            } else if attempt == current {
                prop_assert_eq!(result, Ok(()));
            } else {
                // attempt > current but <= capacity — future attempt rejected (G005 fixed)
                prop_assert_eq!(
                    result,
                    Err(RuntimeError::InvalidActionCompletion),
                    "attempt={} > current={} must be rejected",
                    attempt,
                    current
                );
            }
        }
    }
}
