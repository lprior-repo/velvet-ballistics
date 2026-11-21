#[test]
fn escalation_stamped_past_deadline_replays_as_recorded() -> Result<(), Box<dyn std::error::Error>>
{
    let gate = bound_gate();
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let now = deadline
        .checked_add(1)
        .ok_or("fixture deadline is u64::MAX")?;
    let escalation = evaluate_bound_timeout(&gate, &person("bob"), OPENED_AT_MS, OPENING_SEQ, now)?;
    assert_eq!(escalation.record.outcome, GateOutcome::Escalated);
    let journal = vec![
        bound_gate_opened_event(
            gate.clone(),
            OPENED_AT_MS,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), escalation),
    ];
    let replays = replay_bound_gate_history(&journal)?;
    assert_eq!(replays.len(), 1);
    assert_eq!(
        replays.first().map(|r| r.outcome),
        Some(GateOutcome::Escalated)
    );
    assert!(reconstruct_bound_open_gates(&journal)?.is_empty());
    Ok(())
}

#[test]
fn escalation_after_delegation_replays_without_delegate_approver()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut delegation = input(GateDecision::Delegate);
    delegation.delegated_to = Some(person("carol"));
    let delegated = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &delegation,
    )?;
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let now = deadline
        .checked_add(1)
        .ok_or("fixture deadline is u64::MAX")?;
    let escalation = evaluate_bound_timeout(&gate, &person("bob"), OPENED_AT_MS, OPENING_SEQ, now)?;
    let journal = vec![
        bound_gate_opened_event(
            gate.clone(),
            OPENED_AT_MS,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), delegated),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(3), escalation),
    ];
    let replays = replay_bound_gate_history(&journal)?;
    assert_eq!(
        replays.iter().map(|r| r.outcome).collect::<Vec<_>>(),
        vec![GateOutcome::Delegated, GateOutcome::Escalated]
    );
    assert!(reconstruct_bound_open_gates(&journal)?.is_empty());
    Ok(())
}

#[test]
fn escalation_stamped_at_deadline_is_rejected_on_replay() -> Result<(), Box<dyn std::error::Error>>
{
    let gate = bound_gate();
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let now = deadline
        .checked_add(1)
        .ok_or("fixture deadline is u64::MAX")?;
    let mut escalation =
        evaluate_bound_timeout(&gate, &person("bob"), OPENED_AT_MS, OPENING_SEQ, now)?;
    // Live timeout admission rejects an at-deadline escalation, so the journal row is
    // forged by moving a valid post-deadline escalation back onto the deadline.
    escalation.record.decided_at_ms = deadline;
    let journal = vec![
        bound_gate_opened_event(
            gate.clone(),
            OPENED_AT_MS,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), escalation),
    ];
    assert_eq!(
        replay_bound_gate_history(&journal),
        Err(GateError::NotTimedOut)
    );
    assert_eq!(
        reconstruct_bound_open_gates(&journal),
        Err(GateError::NotTimedOut)
    );
    Ok(())
}

#[test]
fn escalation_exactly_at_deadline_is_rejected_as_not_timed_out()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let err = evaluate_bound_timeout(&gate, &person("bob"), OPENED_AT_MS, OPENING_SEQ, deadline)
        .err()
        .ok_or("escalation at deadline was accepted")?;
    assert_eq!(err, GateError::NotTimedOut);
    Ok(())
}

#[test]
fn decision_at_opening_instant_is_accepted_and_one_before_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut at_open = input(GateDecision::Approve);
    at_open.decided_at_ms = LATE_OPENED_AT_MS;
    let accepted = evaluate_bound_decision(
        &gate,
        LATE_OPENED_AT_MS,
        LATE_OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &at_open,
    )?;
    assert_eq!(accepted.record.outcome, GateOutcome::Approved);

    let mut before_open = input(GateDecision::Approve);
    before_open.decided_at_ms = LATE_OPENED_AT_MS
        .checked_sub(1)
        .ok_or("fixture opening underflow")?;
    let err = evaluate_bound_decision(
        &gate,
        LATE_OPENED_AT_MS,
        LATE_OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &before_open,
    )
    .err()
    .ok_or("decision before opening was accepted")?;
    assert_eq!(
        err,
        GateError::DecisionBeforeOpen {
            opened_at_ms: LATE_OPENED_AT_MS,
            decided_at_ms: LATE_OPENED_AT_MS
                .checked_sub(1)
                .ok_or("fixture opening underflow")?,
        }
    );
    Ok(())
}

#[test]
fn replay_accepts_approval_at_deadline_and_opening_instant()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let deadline = OPENED_AT_MS
        .checked_add(gate.gate.timeout_ms)
        .ok_or("fixture deadline overflow")?;
    let cases = [
        (OPENED_AT_MS, OPENING_SEQ, deadline, 2),
        (LATE_OPENED_AT_MS, LATE_OPENING_SEQ, LATE_OPENED_AT_MS, 4),
    ];
    for (opened_at, opening_seq, decided_at, decision_seq) in cases {
        let mut decision = input(GateDecision::Approve);
        decision.decided_at_ms = decided_at;
        let approved = evaluate_bound_decision(
            &gate,
            opened_at,
            opening_seq,
            &person("bob"),
            OperatorTrust::InProcessCaller,
            &decision,
        )?;
        let journal = vec![
            bound_gate_opened_event(
                gate.clone(),
                opened_at,
                person("bob"),
                vb_storage::EventSeq::new(opening_seq),
            ),
            bound_gate_decision_event(
                gate.gate.run,
                vb_storage::EventSeq::new(decision_seq),
                approved,
            ),
        ];
        let replays = replay_bound_gate_history(&journal)?;
        assert_eq!(
            replays.first().map(|r| r.outcome),
            Some(GateOutcome::Approved),
            "opened_at {opened_at} decided_at {decided_at}"
        );
    }
    Ok(())
}
