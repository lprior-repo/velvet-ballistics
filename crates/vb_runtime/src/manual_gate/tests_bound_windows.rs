const LATE_OPENED_AT_MS: u64 = 700;
const LATE_OPENING_SEQ: u64 = 3;

#[test]
fn decision_exactly_at_deadline_is_accepted_and_one_past_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let past_deadline = deadline
        .checked_add(1)
        .ok_or("fixture deadline is u64::MAX")?;

    let mut at_deadline = input(GateDecision::Approve);
    at_deadline.decided_at_ms = deadline;
    let approved = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &at_deadline,
    )?;
    assert_eq!(approved.record.outcome, GateOutcome::Approved);

    let mut late = input(GateDecision::Approve);
    late.decided_at_ms = past_deadline;
    let err = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &late,
    )
    .err()
    .ok_or("decision past deadline was accepted")?;
    assert_eq!(
        err,
        GateError::DecisionAfterTimeout {
            deadline_ms: deadline,
            decided_at_ms: past_deadline,
        }
    );
    Ok(())
}

#[test]
fn recovery_rejects_journaled_decision_after_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let past_deadline = deadline
        .checked_add(1)
        .ok_or("fixture deadline is u64::MAX")?;
    let mut at_deadline = input(GateDecision::Deny);
    at_deadline.decided_at_ms = deadline;
    // Live admission would reject a post-deadline decision, so the journal row is
    // forged by moving a valid at-deadline record one millisecond past the window.
    let mut record = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &at_deadline,
    )?;
    record.record.decided_at_ms = past_deadline;
    let journal = vec![
        bound_gate_opened_event(
            gate.clone(),
            OPENED_AT_MS,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record),
    ];
    let expected = GateError::DecisionAfterTimeout {
        deadline_ms: deadline,
        decided_at_ms: past_deadline,
    };
    assert_eq!(replay_bound_gate_history(&journal), Err(expected.clone()));
    assert_eq!(reconstruct_bound_open_gates(&journal), Err(expected));
    Ok(())
}

#[test]
fn recovery_rejects_journaled_decision_before_opening() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut at_open = input(GateDecision::Deny);
    at_open.decided_at_ms = LATE_OPENED_AT_MS;
    let mut record = evaluate_bound_decision(
        &gate,
        LATE_OPENED_AT_MS,
        LATE_OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &at_open,
    )?;
    let before_open = LATE_OPENED_AT_MS
        .checked_sub(1)
        .ok_or("fixture opening underflow")?;
    record.record.decided_at_ms = before_open;
    let journal = vec![
        bound_gate_opened_event(
            gate.clone(),
            LATE_OPENED_AT_MS,
            person("bob"),
            vb_storage::EventSeq::new(LATE_OPENING_SEQ),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(4), record),
    ];
    let expected = GateError::DecisionBeforeOpen {
        opened_at_ms: LATE_OPENED_AT_MS,
        decided_at_ms: before_open,
    };
    assert_eq!(replay_bound_gate_history(&journal), Err(expected.clone()));
    assert_eq!(reconstruct_bound_open_gates(&journal), Err(expected));
    Ok(())
}

#[test]
fn replay_rejects_opening_with_different_time_or_sequence() -> Result<(), Box<dyn std::error::Error>>
{
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let mismatch = GateError::ReplayBoundGateMismatch { gate: 7 };

    let shifted_open = vec![
        bound_gate_opened_event(
            gate.clone(),
            OPENED_AT_MS
                .checked_add(1)
                .ok_or("fixture opening overflow")?,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone()),
    ];
    assert_eq!(
        replay_bound_gate_history(&shifted_open),
        Err(mismatch.clone())
    );
    assert_eq!(
        reconstruct_bound_open_gates(&shifted_open),
        Err(mismatch.clone())
    );

    let reseq_record = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ
            .checked_add(1)
            .ok_or("fixture sequence overflow")?,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;

    let reseq = vec![
        bound_gate_opened_event(
            gate.clone(),
            OPENED_AT_MS,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), reseq_record),
    ];
    assert_eq!(replay_bound_gate_history(&reseq), Err(mismatch.clone()));
    assert_eq!(reconstruct_bound_open_gates(&reseq), Err(mismatch));
    Ok(())
}
