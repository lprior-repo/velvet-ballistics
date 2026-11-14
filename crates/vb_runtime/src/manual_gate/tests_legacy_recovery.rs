#[test]
fn reconstruct_open_gates_rejects_duplicate_open_and_invalid_gate() {
    let gate = gate();
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let again = gate_opened_event(gate.clone(), 5, person("bob"), vb_storage::EventSeq::new(2));
    let err = reconstruct_open_gates(&[opened, again]).unwrap_err();
    assert_eq!(err, GateError::ReplayDuplicateOpen { gate: 7 });

    let mut invalid = gate.clone();
    invalid.id = GateId::new(0);
    let bad = gate_opened_event(invalid, 0, person("bob"), vb_storage::EventSeq::new(1));
    let err = reconstruct_open_gates(std::slice::from_ref(&bad)).unwrap_err();
    assert_eq!(err, GateError::ReplayInvalidGate);

    let bad_requester = gate_opened_event(
        gate.clone(),
        0,
        ApproverId {
            value: String::new(),
        },
        vb_storage::EventSeq::new(1),
    );
    let err = reconstruct_open_gates(std::slice::from_ref(&bad_requester)).unwrap_err();
    assert_eq!(err, GateError::ReplayInvalidGate);
}

#[test]
fn reconstruct_open_gates_ignores_decisions_for_unknown_gates() {
    let gate = gate();
    let denied = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(1),
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap(),
    );
    assert!(
        reconstruct_open_gates(std::slice::from_ref(&denied))
            .unwrap()
            .is_empty(),
        "a decision for a gate opened before the window is not an error"
    );

    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let denied = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(2),
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap(),
    );
    let reopened = gate_opened_event(
        gate.clone(),
        3_000,
        person("bob"),
        vb_storage::EventSeq::new(3),
    );
    let open = reconstruct_open_gates(&[opened, denied, reopened]).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].opened_at_ms, 3_000);
}

#[test]
fn reconstruct_open_gates_rejects_invalid_decision_record() {
    let gate = gate();
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let mut denied =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap();
    denied.receipt = evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve))
        .unwrap()
        .receipt;
    assert!(!denied.is_structurally_valid());
    let bad = gate_decision_event(gate.run, vb_storage::EventSeq::new(2), denied);
    let err = reconstruct_open_gates(&[opened, bad]).unwrap_err();
    assert_eq!(err, GateError::InvalidRecord);
}

#[test]
fn gate_id_mismatch_and_invalid_identities_are_rejected() {
    let gate = gate();
    let mut wrong_gate = input(GateDecision::Approve);
    wrong_gate.gate = GateId::new(8);
    let err = evaluate_decision(&gate, 0, &person("bob"), &wrong_gate).unwrap_err();
    assert!(matches!(err, GateError::GateIdMismatch { .. }));

    let mut bad_approver = input(GateDecision::Approve);
    bad_approver.approver = ApproverId {
        value: String::new(),
    };
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad_approver).unwrap_err();
    assert_eq!(err, GateError::InvalidApprover);
}

#[test]
fn requester_spoofing_cannot_evade_separation_of_duties() {
    let gate = gate();
    // The approver is the durable requester, but the submission claims a
    // different requester to try to pass the separation check.
    let mut spoofed = input(GateDecision::Approve);
    spoofed.approver = person("bob");
    spoofed.requester = person("mallory");
    let err = evaluate_decision(&gate, 0, &person("bob"), &spoofed).unwrap_err();
    assert_eq!(err, GateError::RequesterMismatch);
}

#[test]
fn break_glass_requires_evidence_display_for_high_risk_non_compensatable() {
    let gate = gate();
    let mut input = input(GateDecision::BreakGlass);
    input.break_glass_justification = Some("prod is down".to_string());
    input.evidence_shown = false;
    let err = evaluate_decision(&gate, 0, &person("bob"), &input).unwrap_err();
    assert_eq!(err, GateError::EvidenceNotShown);

    input.evidence_shown = true;
    let record = evaluate_decision(&gate, 0, &person("bob"), &input).unwrap();
    assert_eq!(record.outcome, GateOutcome::BreakGlassApproved);
}

#[test]
fn replay_keeps_duplicate_legacy_approval_receipts_in_audit_history() {
    let gate = gate();
    let approved =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
    let events = vec![
        gate_decision_event(gate.run, vb_storage::EventSeq::new(1), approved.clone()),
        gate_decision_event(gate.run, vb_storage::EventSeq::new(2), approved),
    ];
    let history = replay_gate_history(&events).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].receipt_digest, history[1].receipt_digest);
}

#[test]
fn reconstruct_preserves_legacy_requester_divergence() {
    let gate = gate();
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let mut record =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap();
    record.requester = person("mallory");
    assert!(record.is_structurally_valid());
    let decision = gate_decision_event(gate.run, vb_storage::EventSeq::new(2), record);
    assert!(
        reconstruct_open_gates(&[opened, decision])
            .unwrap()
            .is_empty()
    );
}
