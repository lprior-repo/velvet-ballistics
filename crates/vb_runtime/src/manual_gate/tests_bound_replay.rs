#[test]
fn bound_replay_ignores_decision_without_opening() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let decision = bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(1), record);
    assert!(replay_bound_gate_history(&[decision])?.is_empty());
    Ok(())
}

#[test]
fn bound_path_rejects_zero_binding_and_legacy_path_stays_legacy()
-> Result<(), Box<dyn std::error::Error>> {
    let mut invalid = bound_gate();
    invalid.binding.attempt = 0;
    let err = evaluate_bound_decision(
        &invalid,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .err()
    .ok_or("zero attempt was accepted")?;
    assert_eq!(err, GateError::InvalidGate);

    // A bound gate is not a legacy ManualGate; the old evaluator cannot
    // silently approve it because the binding is not part of the old type.
    let legacy = evaluate_decision(
        &invalid.gate,
        0,
        &person("bob"),
        &input(GateDecision::Approve),
    );
    assert_eq!(legacy?.outcome, GateOutcome::Approved);
    Ok(())
}

#[test]
fn bound_replay_allows_gate_id_reuse_after_closure() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let first = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let mut second_input = input(GateDecision::Approve);
    second_input.decided_at_ms = 800;
    let second = evaluate_bound_decision(
        &gate,
        700,
        3,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &second_input,
    )?;
    let events = vec![
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), first),
        bound_gate_opened_event(
            gate.clone(),
            700,
            person("bob"),
            vb_storage::EventSeq::new(3),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(4), second),
    ];
    let replay = replay_bound_gate_history(&events)?;
    assert_eq!(replay.len(), 2);
    assert_eq!(
        replay.first().ok_or("first replay missing")?.outcome,
        GateOutcome::Approved
    );
    assert_eq!(
        replay.get(1).ok_or("second replay missing")?.outcome,
        GateOutcome::Approved
    );
    Ok(())
}

#[test]
fn stale_decision_cannot_close_same_timestamp_reopened_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut deny = input(GateDecision::Deny);
    deny.decided_at_ms = 500;
    let stale = evaluate_bound_decision(
        &gate,
        0,
        10,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &deny,
    )?;
    let events = vec![
        bound_gate_opened_event(
            gate.clone(),
            0,
            person("bob"),
            vb_storage::EventSeq::new(10),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(11), stale.clone()),
        bound_gate_opened_event(
            gate.clone(),
            0,
            person("bob"),
            vb_storage::EventSeq::new(12),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(13), stale),
    ];
    assert_eq!(
        replay_bound_gate_history(&events),
        Err(GateError::ReplayBoundGateMismatch { gate: 7 })
    );
    assert_eq!(
        crate::manual_gate::reconstruct_bound_open_gates(&events),
        Err(GateError::ReplayBoundGateMismatch { gate: 7 })
    );
    Ok(())
}

#[test]
fn legacy_approval_cannot_close_a_bound_gate_during_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let bound = bound_gate();
    let legacy = evaluate_decision(
        &bound.gate,
        0,
        &person("bob"),
        &input(GateDecision::Approve),
    )?;
    let events = vec![
        bound_gate_opened_event(
            bound.clone(),
            0,
            person("bob"),
            vb_storage::EventSeq::new(1),
        ),
        gate_decision_event(bound.gate.run, vb_storage::EventSeq::new(2), legacy),
    ];
    assert!(replay_bound_gate_history(&events)?.is_empty());
    let restored = crate::manual_gate::reconstruct_bound_open_gates(&events)?;
    assert_eq!(restored.len(), 1);
    let open = restored.first().ok_or("recovered bound gate missing")?;
    assert_eq!(open.gate, bound);
    assert_eq!(open.opening_seq, 1);
    Ok(())
}

#[test]
fn bound_replay_rejects_decision_that_does_not_match_opening()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let mut changed = gate.clone();
    changed.binding.input_digest = digest(99);
    let events = vec![
        bound_gate_opened_event(changed, 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone()),
    ];
    let err = replay_bound_gate_history(&events)
        .err()
        .ok_or("mismatched opening was accepted")?;
    assert_eq!(err, GateError::ReplayBoundGateMismatch { gate: 7 });
    let requester_changed = vec![
        bound_gate_opened_event(
            gate.clone(),
            0,
            person("carol"),
            vb_storage::EventSeq::new(1),
        ),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record),
    ];
    let err = replay_bound_gate_history(&requester_changed)
        .err()
        .ok_or("mismatched requester was accepted")?;
    assert_eq!(err, GateError::ReplayBoundGateMismatch { gate: 7 });
    Ok(())
}

#[test]
fn bound_receipt_rejects_stale_binding_and_duplicate_replay()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let mut stale = gate.clone();
    stale.binding.input_digest = digest(99);
    assert!(!verify_bound_receipt(
        record
            .bound_receipt
            .as_ref()
            .ok_or("approval receipt missing")?,
        &stale
    )?);

    let events = vec![
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone()),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(3), record),
    ];
    let err = replay_bound_gate_history(&events)
        .err()
        .ok_or("duplicate receipt replay was accepted")?;
    assert_eq!(err, GateError::ReplayDuplicateBoundReceipt { gate: 7 });
    assert_eq!(
        crate::manual_gate::reconstruct_bound_open_gates(&events),
        Err(GateError::ReplayDuplicateBoundReceipt { gate: 7 })
    );
    Ok(())
}

#[test]
fn bound_events_round_trip_and_legacy_replay_ignores_them() -> Result<(), Box<dyn std::error::Error>>
{
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let opened =
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let decision =
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone());
    for event in [&opened, &decision] {
        let bytes = postcard::to_allocvec(event)?;
        let decoded: vb_storage::JournalEvent = postcard::from_bytes(&bytes)?;
        assert_eq!(&decoded, event);
    }
    assert!(replay_gate_history(&[opened.clone(), decision.clone()])?.is_empty());
    let bound = replay_bound_gate_history(&[opened, decision])?;
    assert_eq!(bound.len(), 1);
    let replay = bound.first().ok_or("bound replay missing")?;
    assert_eq!(replay.outcome, GateOutcome::Approved);
    assert_eq!(
        replay.bound_receipt_digest,
        record.bound_receipt.as_ref().map(|r| r.digest)
    );
    Ok(())
}

#[test]
fn bound_break_glass_still_requires_evidence_display() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.break_glass_justification = Some("prod is down".to_string());
    break_glass.evidence_shown = false;
    let err = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &break_glass,
    )
    .err()
    .ok_or("break-glass without evidence was accepted")?;
    assert_eq!(err, GateError::EvidenceNotShown);
    Ok(())
}
