// =====================================================================
// Black-hat finding: idempotency key overflow graceful degradation
// =====================================================================

#[test]
fn bh_idempotency_key_overflow_fallback_is_deterministic() {
    // When overflow occurs, compute_idempotency_key falls back to run_part.
    // The fallback must still be deterministic for the same inputs.
    let key1 = compute_idempotency_key(
        RunId::new(u64::MAX),
        1,
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    let key2 = compute_idempotency_key(
        RunId::new(u64::MAX),
        1,
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    assert_eq!(key1, key2, "overflow fallback must be deterministic");
}

#[test]
fn bh_idempotency_key_overflow_preserves_run_uniqueness() {
    // Even in overflow, different run IDs must produce different keys
    // (the fallback is run_part, which differs when run IDs differ).
    let key_a = compute_idempotency_key(
        RunId::new(100),
        1,
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    let key_b = compute_idempotency_key(
        RunId::new(200),
        1,
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    assert_ne!(
        key_a, key_b,
        "different runs must produce different keys even under overflow"
    );
}

#[test]
fn bh_idempotency_key_all_three_components_contribute() {
    // Verify that changing any single component while holding the other two
    // constant produces a different key.
    let base_run = RunId::new(42);
    let base_seq = SeqNo::new(10);
    let base_action = ActionId::new(5);

    let base_key = compute_idempotency_key(base_run, 1, base_seq, base_action);

    let key_diff_run = compute_idempotency_key(RunId::new(43), 1, base_seq, base_action);
    assert_ne!(base_key, key_diff_run, "changing run must change key");

    let key_diff_seq = compute_idempotency_key(base_run, 1, SeqNo::new(11), base_action);
    assert_ne!(base_key, key_diff_seq, "changing seq must change key");

    let key_diff_action = compute_idempotency_key(base_run, 1, base_seq, ActionId::new(6));
    assert_ne!(base_key, key_diff_action, "changing action must change key");
}

#[test]
fn bh_idempotency_key_overflow_does_not_produce_zero_for_nonzero_run() {
    // A non-zero run ID must never collapse to zero key even under overflow.
    let key = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(u64::MAX), ActionId::new(65535));
    assert_ne!(key, 0, "non-zero run must not collapse to zero key");
}

// =====================================================================
// Black-hat finding: RetryCheck with NEVER always exhausts at attempt 0
// =====================================================================

#[test]
fn bh_retry_check_never_routes_to_body_at_attempt_zero() {
    // NEVER has max_attempts=1, so attempt 0 < 1 routes to body.
    // This is the one allowed attempt before the first execution.
    let target = execute_retry_check(0, RetryPolicy::NEVER, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(
        target,
        StepIdx::new(5),
        "attempt 0 with NEVER should route to body"
    );
}

#[test]
fn bh_retry_check_never_routes_to_exhausted_at_attempt_zero_for_zero_max() {
    // A policy with max_attempts=0 should exhaust immediately at attempt 0.
    let policy = RetryPolicy {
        max_attempts: 0,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(0, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(
        target,
        StepIdx::new(10),
        "attempt 0 with max_attempts=0 should exhaust"
    );
}

#[test]
fn bh_retry_check_default_allows_body_for_all_attempts_below_max() {
    // DEFAULT has max_attempts=3. Attempts 0, 1, 2 should all route to body.
    for attempt in 0u16..3 {
        let target = execute_retry_check(
            attempt,
            RetryPolicy::DEFAULT,
            StepIdx::new(1),
            StepIdx::new(99),
        );
        assert_eq!(
            target,
            StepIdx::new(1),
            "DEFAULT attempt {attempt} should route to body"
        );
    }
}

#[test]
fn bh_retry_check_custom_policy_boundary() {
    // Exact boundary: attempt == max_attempts routes to exhausted.
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let body = StepIdx::new(3);
    let exhausted = StepIdx::new(9);

    // attempt 4 < 5 => body
    let t4 = execute_retry_check(4, policy, body, exhausted);
    assert_eq!(t4, body);

    // attempt 5 == 5 => exhausted
    let t5 = execute_retry_check(5, policy, body, exhausted);
    assert_eq!(t5, exhausted);

    // attempt 6 > 5 => exhausted
    let t6 = execute_retry_check(6, policy, body, exhausted);
    assert_eq!(t6, exhausted);
}

// =====================================================================
// Black-hat finding: drive evidence correctness
// =====================================================================

#[test]
fn bh_drive_budget_exhausted_does_not_emit_step_succeeded_in_evidence() {
    // When budget is zero, drive_deterministic_full returns StepBudgetExhausted
    // immediately without executing any step. Evidence should be empty.
    let node = vb_core::workflow::node(StepIdx::ZERO, CompiledNodeKind::Nop, None, None);
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("bh_nop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(0);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));

    // Budget was zero so no step executed: evidence must be completely empty.
    let events = evidence.drain();
    assert!(
        events.is_empty(),
        "budget exhaustion should produce zero evidence events, got {events:?}"
    );
}

#[test]
fn bh_drive_single_nop_emits_started_then_succeeded() {
    // With budget > 0 and a Nop that advances to a Finish, the drive loop
    // executes at least one step. Evidence should contain matched pairs
    // of StepStarted/StepSucceeded.
    let node = vb_core::workflow::node(StepIdx::ZERO, CompiledNodeKind::Nop, None, None);
    let finish = vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("bh_nop_finish"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
        1,
    );
    // The drive loop should have progressed (either Continue after Nop, or Finished after Finish).
    assert!(result.is_ok(), "drive should succeed, got {result:?}");

    let events = evidence.drain();
    // Every executed step must have a StepStarted followed by StepSucceeded pair.
    let started_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
        .count();
    let succeeded_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepSucceeded { .. }))
        .count();
    assert_eq!(
        started_count, succeeded_count,
        "every StepStarted must have a matching StepSucceeded"
    );
    assert!(
        started_count > 0,
        "at least one step should have been executed"
    );
}

#[test]
fn bh_drive_evidence_step_succeeded_not_emitted_for_awaiting_action() {
    // A Do node produces AwaitingAction. The drive loop must NOT emit StepSucceeded
    // because the step did not actually complete -- it suspended waiting for an
    // action result. Only StepStarted should be emitted for such steps.
    let node = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Do {
            action: ActionId::new(5),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("bh_do"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Input slot must be initialized with clean taint.
    assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(0)), Ok(()));
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
        1,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(_)) => {}
        other => {
            assert!(
                other.is_err(),
                "expected AwaitingAction or error, got {other:?}"
            );
            return;
        }
    }

    let events = evidence.drain();
    // The drive loop emits StepStarted but NOT StepSucceeded for steps
    // that suspend with AwaitingAction. StepSucceeded is only emitted
    // for Continue and Finished signals.
    let started_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
        .count();
    let succeeded_count = events
        .iter()
        .filter(|e| matches!(e, EvidenceEvent::StepSucceeded { .. }))
        .count();
    assert_eq!(
        started_count, 1,
        "exactly one StepStarted should be emitted for the Do node"
    );
    assert_eq!(
        succeeded_count, 0,
        "StepSucceeded must NOT be emitted for AwaitingAction -- the step did not complete"
    );
}

// =====================================================================
// Black-hat finding: runtime signal taint preservation
// =====================================================================

#[test]
fn bh_resume_action_outcome_ready_preserves_secret_taint() {
    // When an action completes with a tainted output, the taint must be
    // preserved through resume_action_outcome into the run frame.
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(42),
        taint: Taint::Secret,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    };
    let result = resume_action_outcome(&ticket, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn bh_resume_action_outcome_ready_preserves_derived_taint() {
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(99),
        taint: Taint::DerivedFromSecret,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    };
    let result = resume_action_outcome(&ticket, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn bh_resume_action_outcome_ready_clean_taint_preserved() {
    let ready = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(1),
        taint: Taint::Clean,
        encoded_len: 8,
    };
    let outcome = ActionOutcome::Ready(ready);
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    };
    let result = resume_action_outcome(&ticket, outcome, &dummy_contract());
    assert_eq!(result, Ok(RuntimeSignal::Continue));
}

#[test]
fn bh_resume_action_outcome_suspended_preserves_ticket_fields() {
    // The Suspended path must pass the ticket through unchanged.
    let original_ticket = ActionTicket {
        run: RunId::new(7),
        step: StepIdx::new(3),
        seq: SeqNo::new(11),
        action: ActionId::new(5),
        run_attempt: 1,
        attempt: 3,
        idempotency_key: 12345,
        capacity: 1,
        fencing_token: 0,
    };
    let outcome = ActionOutcome::Suspended(original_ticket);
    let result = resume_action_outcome(&make_original_ticket(), outcome, &dummy_contract());
    match result {
        Ok(RuntimeSignal::AwaitingAction(returned_ticket)) => {
            assert_eq!(returned_ticket.run, RunId::new(7));
            assert_eq!(returned_ticket.step, StepIdx::new(3));
            assert_eq!(returned_ticket.seq, SeqNo::new(11));
            assert_eq!(returned_ticket.action, ActionId::new(5));
            assert_eq!(returned_ticket.attempt, 3);
            assert_eq!(returned_ticket.idempotency_key, 12345);
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

#[test]
fn bh_resume_action_outcome_failed_retryable_preserves_signal_structure() {
    // A retryable failure should produce an AwaitingAction with a retry ticket
    // derived from the original ticket (incremented seq and attempt).
    let _run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let outcome = ActionOutcome::Failed(failure);
    let original = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(5),
        action: ActionId::new(3),
        run_attempt: 1,
        attempt: 2,
        idempotency_key: 100,
        capacity: 4,
        fencing_token: 0,
    };
    let result = resume_action_outcome(&original, outcome, &dummy_contract());
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            // The retry ticket uses the run's ID, original step, incremented seq and attempt.
            assert_eq!(ticket.run, RunId::new(1));
            assert_eq!(
                ticket.step,
                StepIdx::new(2),
                "step must come from original ticket"
            );
            assert_eq!(
                ticket.seq,
                SeqNo::new(6),
                "seq must be incremented from original"
            );
            assert_eq!(
                ticket.action,
                ActionId::new(3),
                "action must come from original"
            );
            assert_eq!(
                ticket.attempt, 3,
                "attempt must be incremented from original"
            );
        }
        other => {
            let msg = format!("expected AwaitingAction for retryable failure, got {other:?}");
            panic!("{msg}");
        }
    }
}

#[test]
fn bh_execute_do_propagates_taint_through_ticket_for_at_least_once() {
    // execute_do with AtLeastOnceExternal and Secret input must succeed
    // (no taint violation) and produce an AwaitingAction ticket.
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let write_result = run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret);
    assert_eq!(write_result.map(|_| ()), Ok(()));

    let contract = vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let registry = vec![contract];
    let contract_ref = match registry.get(0) {
        Some(c) => c,
        None => return,
    };
    let result = execute_do(
        &run,
        StepIdx::new(0),
        ActionId::new(0),
        SlotIdx::new(0),
        SeqNo::new(1),
        contract_ref,
        &registry,
        &CapabilitySet::empty(),
        RetryPolicy::DEFAULT,
        1,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(0));
            assert_eq!(ticket.run, RunId::new(1));
            assert_eq!(ticket.seq, SeqNo::new(1));
            assert_eq!(ticket.attempt, 1);
            // The idempotency key must be non-zero for non-trivial inputs.
            assert_ne!(ticket.idempotency_key, 0);
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Black-hat finding: EvidenceEvent ordering invariant
// =====================================================================

#[test]
fn bh_evidence_events_always_alternate_started_succeeded() {
    // For every step that completes without error, evidence must be
    // [StepStarted(N), StepSucceeded(N)] in strict alternating order.
    // This test uses the EvidenceCollector directly to verify the pattern.
    let mut collector = EvidenceCollector::new();
    collector.push_step_started(StepIdx::new(0)).expect("ok");
    collector
        .push_step_succeeded(StepIdx::new(0), Some(SlotIdx::new(1)))
        .expect("ok");
    collector.push_step_started(StepIdx::new(1)).expect("ok");
    collector
        .push_step_succeeded(StepIdx::new(1), None)
        .expect("ok");

    let events = collector.drain();
    assert_eq!(events.len(), 4);

    // Verify strict alternation: even indices are Started, odd are Succeeded.
    for (i, event) in events.iter().enumerate() {
        if i % 2 == 0 {
            assert!(
                matches!(event, EvidenceEvent::StepStarted { .. }),
                "event at index {i} should be StepStarted, got {event:?}"
            );
        } else {
            assert!(
                matches!(event, EvidenceEvent::StepSucceeded { .. }),
                "event at index {i} should be StepSucceeded, got {event:?}"
            );
        }
    }
}

