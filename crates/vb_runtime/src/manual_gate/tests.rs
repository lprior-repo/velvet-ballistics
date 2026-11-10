use super::*;
use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, GateDecision, GateDecisionInput, GateDigest, GateId, GateOutcome,
    ManualGate, RiskClass, SeparationOfDuties,
};

fn digest(byte: u8) -> GateDigest {
    GateDigest::from_bytes([byte; 32])
}

fn role() -> ApproverRole {
    ApproverRole::new("deploy-approver".to_string()).unwrap()
}

fn person(value: &str) -> ApproverId {
    ApproverId::new(value.to_string()).unwrap()
}

fn gate() -> ManualGate {
    ManualGate {
        id: GateId::new(7),
        run: RunId::new(3),
        action: ActionId::new(9),
        risk: RiskClass::High,
        compensatable: false,
        required_role: role(),
        separation_of_duties: SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: digest(1),
        prompt_digest: digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn input(decision: GateDecision) -> GateDecisionInput {
    GateDecisionInput {
        gate: GateId::new(7),
        decision,
        approver: person("alice"),
        approver_role: role(),
        requester: person("bob"),
        reason: Some("checked the rollout plan".to_string()),
        evidence_shown: true,
        evidence_bundle_digest: digest(1),
        delegated_to: None,
        break_glass_justification: None,
        decided_at_ms: 500,
    }
}

#[test]
fn approve_happy_path_yields_receipt_bound_to_gate() {
    let gate = gate();
    let record = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
    assert_eq!(record.outcome, GateOutcome::Approved);
    assert_eq!(record.decision, Some(GateDecision::Approve));
    let receipt = record.receipt.as_ref().unwrap();
    assert_eq!(receipt.gate, gate.id);
    assert_eq!(receipt.run, gate.run);
    assert_eq!(receipt.evidence_bundle_digest, gate.evidence_bundle_digest);
    assert_eq!(receipt.prompt_digest, gate.prompt_digest);
    assert!(verify_receipt(receipt, &gate).unwrap());
    assert!(record.is_structurally_valid());
    assert!(record.matches_gate(&gate));
}

#[test]
fn deny_and_more_evidence_carry_no_receipt() {
    let gate = gate();
    for (decision, outcome) in [
        (GateDecision::Deny, GateOutcome::Denied),
        (
            GateDecision::RequestMoreEvidence,
            GateOutcome::MoreEvidenceRequested,
        ),
    ] {
        let record = evaluate_decision(&gate, 0, &input(decision)).unwrap();
        assert_eq!(record.outcome, outcome);
        assert!(record.receipt.is_none());
        assert!(record.is_structurally_valid());
    }
}

#[test]
fn wrong_role_is_rejected() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert!(matches!(err, GateError::WrongRole { .. }));
}

#[test]
fn separation_of_duties_rejects_self_approval_for_high_risk() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.approver = person("bob");
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::SeparationOfDutiesViolation);
}

#[test]
fn separation_rule_extends_to_low_risk_when_gate_requires_it() {
    let mut gate = gate();
    gate.risk = RiskClass::Low;
    let mut bad = input(GateDecision::Approve);
    bad.approver = person("bob");
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::SeparationOfDutiesViolation);

    gate.separation_of_duties = SeparationOfDuties::Disabled;
    let ok = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
    assert_eq!(ok.outcome, GateOutcome::Approved);
}

#[test]
fn evidence_digest_mismatch_is_rejected() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.evidence_bundle_digest = digest(99);
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::EvidenceDigestMismatch);
}

#[test]
fn evidence_not_shown_blocks_high_risk_non_compensatable_approval() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.evidence_shown = false;
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::EvidenceNotShown);

    let mut compensatable = gate.clone();
    compensatable.compensatable = true;
    let ok = evaluate_decision(&compensatable, 0, &bad).unwrap();
    assert_eq!(ok.outcome, GateOutcome::Approved);
}

#[test]
fn approval_reason_required_is_enforced() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.reason = Some("   ".to_string());
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::ReasonRequired);
    bad.reason = None;
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::ReasonRequired);
}

#[test]
fn delegate_requires_target_distinct_from_approver() {
    let gate = gate();
    let missing = input(GateDecision::Delegate);
    let err = evaluate_decision(&gate, 0, &missing).unwrap_err();
    assert_eq!(err, GateError::DelegateTargetMissing);

    let mut self_target = input(GateDecision::Delegate);
    self_target.delegated_to = Some(person("alice"));
    let err = evaluate_decision(&gate, 0, &self_target).unwrap_err();
    assert_eq!(err, GateError::DelegateTargetIsApprover);

    let mut ok_input = input(GateDecision::Delegate);
    ok_input.delegated_to = Some(person("carol"));
    let record = evaluate_decision(&gate, 0, &ok_input).unwrap();
    assert_eq!(record.outcome, GateOutcome::Delegated);
    assert_eq!(record.delegated_to, Some(person("carol")));
    assert!(record.receipt.is_none());
}

#[test]
fn break_glass_requires_permission_and_justification_and_audits() {
    let gate = gate();
    let mut no_justification = input(GateDecision::BreakGlass);
    let err = evaluate_decision(&gate, 0, &no_justification).unwrap_err();
    assert_eq!(err, GateError::BreakGlassJustificationRequired);

    no_justification.break_glass_justification = Some("prod is down".to_string());
    // Break-glass waives the role check: a viewer may break glass.
    no_justification.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
    let record = evaluate_decision(&gate, 0, &no_justification).unwrap();
    assert_eq!(record.outcome, GateOutcome::BreakGlassApproved);
    assert!(record.break_glass);
    assert_eq!(
        record.break_glass_justification,
        Some("prod is down".to_string())
    );
    assert!(record.receipt.is_some());

    let mut disallowed = gate.clone();
    disallowed.break_glass_allowed = false;
    let err = evaluate_decision(&disallowed, 0, &no_justification).unwrap_err();
    assert_eq!(err, GateError::BreakGlassNotAllowed);
}

#[test]
fn break_glass_still_enforces_separation_for_high_risk() {
    let gate = gate();
    let mut bad = input(GateDecision::BreakGlass);
    bad.break_glass_justification = Some("prod is down".to_string());
    bad.approver = person("bob");
    let err = evaluate_decision(&gate, 0, &bad).unwrap_err();
    assert_eq!(err, GateError::SeparationOfDutiesViolation);
}

#[test]
fn decision_after_deadline_is_rejected_and_timeout_escalates() {
    let gate = gate();
    let mut late = input(GateDecision::Approve);
    late.decided_at_ms = 1_001;
    let err = evaluate_decision(&gate, 0, &late).unwrap_err();
    assert!(matches!(err, GateError::DecisionAfterTimeout { .. }));

    // Exactly at the deadline is still on time.
    late.decided_at_ms = 1_000;
    assert!(evaluate_decision(&gate, 0, &late).is_ok());

    // Timeout evaluation before the deadline fails closed.
    let err = evaluate_timeout(&gate, &person("bob"), 0, 1_000).unwrap_err();
    assert_eq!(err, GateError::NotTimedOut);

    let record = evaluate_timeout(&gate, &person("bob"), 0, 1_001).unwrap();
    assert_eq!(record.outcome, GateOutcome::Escalated);
    assert_eq!(record.decision, None);
    assert!(record.receipt.is_none());
    assert_eq!(
        record.escalation_policy_digest,
        gate.escalation_policy_digest
    );
    assert!(record.is_structurally_valid());
}

#[test]
fn receipt_digest_is_deterministic_and_sensitive() {
    let gate = gate();
    let first = compute_receipt_digest(
        gate.id.get(),
        gate.run,
        GateDecision::Approve,
        &person("alice"),
        gate.evidence_bundle_digest,
        gate.prompt_digest,
        500,
    )
    .unwrap();
    let again = compute_receipt_digest(
        gate.id.get(),
        gate.run,
        GateDecision::Approve,
        &person("alice"),
        gate.evidence_bundle_digest,
        gate.prompt_digest,
        500,
    )
    .unwrap();
    assert_eq!(first, again);
    let changed_evidence = compute_receipt_digest(
        gate.id.get(),
        gate.run,
        GateDecision::Approve,
        &person("alice"),
        digest(99),
        gate.prompt_digest,
        500,
    )
    .unwrap();
    assert_ne!(first, changed_evidence);
    let changed_time = compute_receipt_digest(
        gate.id.get(),
        gate.run,
        GateDecision::Approve,
        &person("alice"),
        gate.evidence_bundle_digest,
        gate.prompt_digest,
        501,
    )
    .unwrap();
    assert_ne!(first, changed_time);
}

#[test]
fn receipt_verification_fails_on_tampered_digest() {
    let gate = gate();
    let record = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
    let mut receipt = record.receipt.unwrap();
    assert!(verify_receipt(&receipt, &gate).unwrap());
    receipt.digest = digest(77);
    assert!(!verify_receipt(&receipt, &gate).unwrap());
    receipt.digest = compute_receipt_digest(
        gate.id.get(),
        gate.run,
        GateDecision::Approve,
        &receipt.approver,
        digest(99),
        gate.prompt_digest,
        receipt.decided_at_ms,
    )
    .unwrap();
    // Digest recomputed over tampered evidence no longer matches gate binding.
    receipt.evidence_bundle_digest = digest(99);
    assert!(!verify_receipt(&receipt, &gate).unwrap());
}

#[test]
fn replay_reconstructs_history_without_effects() {
    let gate = gate();
    let approved = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
    let denied = evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap();
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
    let record = evaluate_decision(&gate, 0, &input(GateDecision::Approve)).unwrap();
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
        evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap(),
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
            evaluate_decision(&gate, 0, &decision_input).unwrap(),
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
        evaluate_decision(&gate, 0, &input(GateDecision::RequestMoreEvidence)).unwrap(),
    );
    let open = reconstruct_open_gates(&[opened.clone(), more]).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].delegated_to, None);

    let mut delegate_input = input(GateDecision::Delegate);
    delegate_input.delegated_to = Some(person("carol"));
    let delegated = gate_decision_event(
        gate.run,
        vb_storage::EventSeq::new(3),
        evaluate_decision(&gate, 0, &delegate_input).unwrap(),
    );
    let open = reconstruct_open_gates(&[opened, delegated]).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].delegated_to, Some(person("carol")));
}

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
        evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap(),
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
        evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap(),
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
    let mut denied = evaluate_decision(&gate, 0, &input(GateDecision::Deny)).unwrap();
    denied.receipt = evaluate_decision(&gate, 0, &input(GateDecision::Approve))
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
    let err = evaluate_decision(&gate, 0, &wrong_gate).unwrap_err();
    assert!(matches!(err, GateError::GateIdMismatch { .. }));

    let mut bad_approver = input(GateDecision::Approve);
    bad_approver.approver = ApproverId {
        value: String::new(),
    };
    let err = evaluate_decision(&gate, 0, &bad_approver).unwrap_err();
    assert_eq!(err, GateError::InvalidApprover);
}
