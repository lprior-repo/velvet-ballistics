#[test]
fn replay_reconstructs_history_without_effects() {
    let gate = gate();
    let approved =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
    let denied = evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap();
    let escalated = evaluate_timeout(&gate, &person("bob"), 0, 2_000).unwrap();

    let replay = replay_gate_outcome(&approved, &gate).unwrap();
    assert_eq!(replay.outcome, GateOutcome::Approved);
    assert_eq!(
        replay.receipt_digest,
        approved.receipt.as_ref().map(|r| r.digest)
    );
    assert!(!replay.escalated);

    let denied_replay = replay_gate_outcome(&denied, &gate).unwrap();
    assert_eq!(denied_replay.receipt_digest, None);

    let escalated_replay = replay_gate_outcome(&escalated, &gate).unwrap();
    assert!(escalated_replay.escalated);
    assert_eq!(escalated_replay.decision, None);

    // Replay rejects a record bound to different evidence.
    let mut tampered = approved.clone();
    tampered.evidence_bundle_digest = digest(99);
    let err = replay_gate_outcome(&tampered, &gate).unwrap_err();
    assert_eq!(err, GateError::RecordGateMismatch);

    // History replay over journal events preserves order and outcomes.
    let events = vec![
        gate_decision_event(gate.run, vb_storage::EventSeq::new(1), approved),
        gate_decision_event(gate.run, vb_storage::EventSeq::new(2), denied),
        gate_decision_event(gate.run, vb_storage::EventSeq::new(3), escalated),
    ];
    let history = replay_gate_history(&events).unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].outcome, GateOutcome::Approved);
    assert_eq!(history[1].outcome, GateOutcome::Denied);
    assert_eq!(history[2].outcome, GateOutcome::Escalated);
}

#[test]
fn journal_event_round_trips_through_postcard() {
    let gate = gate();
    let record =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
    let event = gate_decision_event(gate.run, vb_storage::EventSeq::new(1), record.clone());
    assert_eq!(event.run_id(), gate.run);
    assert_eq!(event.seq(), vb_storage::EventSeq::new(1));
    assert_eq!(event.record_kind(), vb_storage::RecordKind::GateDecision);
    let bytes = postcard::to_allocvec(&event).unwrap();
    let decoded: vb_storage::JournalEvent = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, event);
    // Replay from the decoded event reproduces the same outcome.
    let history = replay_gate_history(std::slice::from_ref(&decoded)).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].outcome, GateOutcome::Approved);
    assert_eq!(history[0].receipt_digest, record.receipt.map(|r| r.digest));
}

#[test]
fn gate_opened_event_round_trips_through_postcard() {
    let gate = gate();
    let event = gate_opened_event(
        gate.clone(),
        100,
        person("bob"),
        vb_storage::EventSeq::new(1),
    );
    assert_eq!(event.run_id(), gate.run);
    assert_eq!(event.seq(), vb_storage::EventSeq::new(1));
    assert_eq!(event.record_kind(), vb_storage::RecordKind::GateOpened);
    let bytes = postcard::to_allocvec(&event).unwrap();
    let decoded: vb_storage::JournalEvent = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, event);
    let open = reconstruct_open_gates(std::slice::from_ref(&decoded)).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].gate, gate);
    assert_eq!(open[0].opened_at_ms, 100);
    assert_eq!(open[0].requester, person("bob"));
    assert_eq!(open[0].delegated_to, None);
}

#[test]
fn reconstruct_open_gates_folds_open_and_close_in_order() {
    let gate = gate();
    let mut other = gate.clone();
    other.id = GateId::new(8);
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let other_opened = gate_opened_event(
        other.clone(),
        10,
        person("bob"),
        vb_storage::EventSeq::new(2),
    );
    let denied = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(3),
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap(),
    );

    let open = reconstruct_open_gates(&[opened.clone(), other_opened.clone()]).unwrap();
    assert_eq!(open.len(), 2);
    assert_eq!(open[0].gate, gate);
    assert_eq!(open[1].gate, other);

    let open = reconstruct_open_gates(&[opened, other_opened, denied]).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].gate, other);

    for decision in [
        GateDecision::Approve,
        GateDecision::BreakGlass,
        GateDecision::Deny,
    ] {
        let mut decision_input = input(decision);
        if decision == GateDecision::BreakGlass {
            decision_input.break_glass_justification = Some("prod is down".to_string());
        }
        let opened =
            gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
        let decided = gate_decision_event(
            gate.run,
            vb_storage::EventSeq::new(2),
            evaluate_decision(&gate, 0, &person("bob"), &decision_input).unwrap(),
        );
        assert!(
            reconstruct_open_gates(&[opened, decided])
                .unwrap()
                .is_empty(),
            "{decision:?} must close the gate"
        );
    }
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let escalated = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(2),
        evaluate_timeout(&gate, &person("bob"), 0, 2_000).unwrap(),
    );
    assert!(
        reconstruct_open_gates(&[opened, escalated])
            .unwrap()
            .is_empty(),
        "escalation must close the gate"
    );
}

#[test]
fn reconstruct_open_gates_keeps_gate_on_more_evidence_and_delegation() {
    let gate = gate();
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let more = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(2),
        evaluate_decision(
            &gate,
            0,
            &person("bob"),
            &input(GateDecision::RequestMoreEvidence),
        )
        .unwrap(),
    );
    let open = reconstruct_open_gates(&[opened.clone(), more]).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].delegated_to, None);

    let mut delegate_input = input(GateDecision::Delegate);
    delegate_input.delegated_to = Some(person("carol"));
    let delegated = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(3),
        evaluate_decision(&gate, 0, &person("bob"), &delegate_input).unwrap(),
    );
    let open = reconstruct_open_gates(&[opened, delegated]).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].delegated_to, Some(person("carol")));
}
