use super::*;
use vb_core::ids::{ActionId, RunId};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundManualGate, GateBinding, GateDecision, GateDecisionInput,
    GateDigest, GateId, GateOutcome, ManualGate, OperatorTrust, RiskClass, SeparationOfDuties,
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

fn bound_gate() -> BoundManualGate {
    BoundManualGate {
        gate: gate(),
        binding: GateBinding {
            artifact_digest: digest(4),
            policy_digest: digest(5),
            input_digest: digest(6),
            attempt: 1,
        },
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
    let record =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
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
        let record = evaluate_decision(&gate, 0, &person("bob"), &input(decision)).unwrap();
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
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert!(matches!(err, GateError::WrongRole { .. }));
}

#[test]
fn separation_of_duties_rejects_self_approval_for_high_risk() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.approver = person("bob");
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::SeparationOfDutiesViolation);
}

#[test]
fn separation_rule_extends_to_low_risk_when_gate_requires_it() {
    let mut gate = gate();
    gate.risk = RiskClass::Low;
    let mut bad = input(GateDecision::Approve);
    bad.approver = person("bob");
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::SeparationOfDutiesViolation);

    gate.separation_of_duties = SeparationOfDuties::Disabled;
    let ok = evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
    assert_eq!(ok.outcome, GateOutcome::Approved);
}

#[test]
fn evidence_digest_mismatch_is_rejected() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.evidence_bundle_digest = digest(99);
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::EvidenceDigestMismatch);
}

#[test]
fn evidence_not_shown_blocks_high_risk_non_compensatable_approval() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.evidence_shown = false;
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::EvidenceNotShown);

    let mut compensatable = gate.clone();
    compensatable.compensatable = true;
    let ok = evaluate_decision(&compensatable, 0, &person("bob"), &bad).unwrap();
    assert_eq!(ok.outcome, GateOutcome::Approved);
}

#[test]
fn approval_reason_required_is_enforced() {
    let gate = gate();
    let mut bad = input(GateDecision::Approve);
    bad.reason = Some("   ".to_string());
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::ReasonRequired);
    bad.reason = None;
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::ReasonRequired);
}

#[test]
fn delegate_requires_target_distinct_from_approver() {
    let gate = gate();
    let missing = input(GateDecision::Delegate);
    let err = evaluate_decision(&gate, 0, &person("bob"), &missing).unwrap_err();
    assert_eq!(err, GateError::DelegateTargetMissing);

    let mut self_target = input(GateDecision::Delegate);
    self_target.delegated_to = Some(person("alice"));
    let err = evaluate_decision(&gate, 0, &person("bob"), &self_target).unwrap_err();
    assert_eq!(err, GateError::DelegateTargetIsApprover);

    let mut ok_input = input(GateDecision::Delegate);
    ok_input.delegated_to = Some(person("carol"));
    let record = evaluate_decision(&gate, 0, &person("bob"), &ok_input).unwrap();
    assert_eq!(record.outcome, GateOutcome::Delegated);
    assert_eq!(record.delegated_to, Some(person("carol")));
    assert!(record.receipt.is_none());
}

#[test]
fn break_glass_requires_permission_and_justification_and_audits() {
    let gate = gate();
    let mut no_justification = input(GateDecision::BreakGlass);
    let err = evaluate_decision(&gate, 0, &person("bob"), &no_justification).unwrap_err();
    assert_eq!(err, GateError::BreakGlassJustificationRequired);

    no_justification.break_glass_justification = Some("prod is down".to_string());
    // Break-glass waives the role check: a viewer may break glass.
    no_justification.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
    let record = evaluate_decision(&gate, 0, &person("bob"), &no_justification).unwrap();
    assert_eq!(record.outcome, GateOutcome::BreakGlassApproved);
    assert!(record.break_glass);
    assert_eq!(
        record.break_glass_justification,
        Some("prod is down".to_string())
    );
    assert!(record.receipt.is_some());

    let mut disallowed = gate.clone();
    disallowed.break_glass_allowed = false;
    let err = evaluate_decision(&disallowed, 0, &person("bob"), &no_justification).unwrap_err();
    assert_eq!(err, GateError::BreakGlassNotAllowed);
}

#[test]
fn break_glass_still_enforces_separation_for_high_risk() {
    let gate = gate();
    let mut bad = input(GateDecision::BreakGlass);
    bad.break_glass_justification = Some("prod is down".to_string());
    bad.approver = person("bob");
    let err = evaluate_decision(&gate, 0, &person("bob"), &bad).unwrap_err();
    assert_eq!(err, GateError::SeparationOfDutiesViolation);
}

#[test]
fn decision_after_deadline_is_rejected_and_timeout_escalates() {
    let gate = gate();
    let mut late = input(GateDecision::Approve);
    late.decided_at_ms = 1_001;
    let err = evaluate_decision(&gate, 0, &person("bob"), &late).unwrap_err();
    assert!(matches!(err, GateError::DecisionAfterTimeout { .. }));

    // Exactly at the deadline is still on time.
    late.decided_at_ms = 1_000;
    assert!(evaluate_decision(&gate, 0, &person("bob"), &late).is_ok());

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
    let record =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
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
fn replay_rejects_duplicate_approval_receipts() {
    let gate = gate();
    let approved =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Approve)).unwrap();
    let events = vec![
        gate_decision_event(gate.run, vb_storage::EventSeq::new(1), approved.clone()),
        gate_decision_event(gate.run, vb_storage::EventSeq::new(2), approved),
    ];
    let err = replay_gate_history(&events).unwrap_err();
    assert_eq!(err, GateError::ReplayDuplicateReceipt { gate: 7 });
}

#[test]
fn reconstruct_rejects_requester_that_differs_from_durable_opening() {
    let gate = gate();
    let opened = gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let mut record =
        evaluate_decision(&gate, 0, &person("bob"), &input(GateDecision::Deny)).unwrap();
    record.requester = person("mallory");
    assert!(record.is_structurally_valid());
    let decision = gate_decision_event(gate.run, vb_storage::EventSeq::new(2), record);
    let err = reconstruct_open_gates(&[opened, decision]).unwrap_err();
    assert_eq!(err, GateError::RequesterMismatch);
}

#[test]
fn bound_approve_binds_binding_trust_and_v2_receipt() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::LocalSocketClaim,
        &input(GateDecision::Approve),
    )
    .unwrap();
    assert!(record.is_structurally_valid());
    assert!(record.matches_bound_gate(&gate));
    assert_eq!(record.binding, gate.binding);
    assert_eq!(record.operator_trust, Some(OperatorTrust::LocalSocketClaim));
    let receipt = record.bound_receipt.as_ref().unwrap();
    assert_eq!(receipt.binding, gate.binding);
    assert_eq!(receipt.requester, person("bob"));
    assert_eq!(receipt.risk, gate.gate.risk);
    assert_eq!(receipt.separation, gate.gate.separation_of_duties);
    assert_eq!(receipt.action, gate.gate.action);
    assert_ne!(receipt.digest, GateDigest::ZERO);
    assert!(verify_bound_receipt(receipt, &gate).unwrap());
    assert!(verify_receipt(&receipt.receipt, &gate.gate).unwrap());
}

#[test]
fn bound_break_glass_records_claimed_role_outside_required_role() {
    let gate = bound_gate();
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.approver_role = ApproverRole::new(String::from("viewer")).expect("viewer role");
    break_glass.break_glass_justification = Some(String::from("prod is down"));
    break_glass.evidence_shown = true;
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &break_glass,
    )
    .expect("break-glass must accept a claimed role outside the required role");
    assert_eq!(record.record.outcome, GateOutcome::BreakGlassApproved);
    assert_eq!(
        record
            .bound_receipt
            .as_ref()
            .expect("break-glass receipt")
            .approver_role
            .name(),
        "viewer"
    );
    assert!(verify_bound_receipt(record.bound_receipt.as_ref().expect("receipt"), &gate).unwrap());
}

#[test]
fn bound_receipt_digest_is_deterministic_and_field_sensitive() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let base = record.bound_receipt.as_ref().unwrap();
    let base_digest = compute_bound_receipt_digest(base).unwrap();
    assert_eq!(
        base_digest,
        compute_bound_receipt_digest(base).unwrap(),
        "same receipt must hash identically"
    );
    let mut changed = Vec::new();
    let mut variant = base.clone();
    variant.binding.artifact_digest = digest(90);
    changed.push(variant);
    let mut variant = base.clone();
    variant.binding.policy_digest = digest(91);
    changed.push(variant);
    let mut variant = base.clone();
    variant.binding.input_digest = digest(92);
    changed.push(variant);
    let mut variant = base.clone();
    variant.binding.attempt = 2;
    changed.push(variant);
    let mut variant = base.clone();
    variant.requester = person("carol");
    changed.push(variant);
    let mut variant = base.clone();
    variant.approver_role = ApproverRole::new(String::from("viewer")).unwrap();
    changed.push(variant);
    let mut variant = base.clone();
    variant.risk = RiskClass::Low;
    changed.push(variant);
    let mut variant = base.clone();
    variant.separation = SeparationOfDuties::Disabled;
    changed.push(variant);
    let mut variant = base.clone();
    variant.operator_trust = OperatorTrust::LocalSocketClaim;
    changed.push(variant);
    let mut variant = base.clone();
    variant.action = ActionId::new(10);
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.decided_at_ms = 501;
    changed.push(variant);
    for variant in changed {
        let changed_digest = compute_bound_receipt_digest(&variant).unwrap();
        assert_ne!(base_digest, changed_digest);
        assert!(!verify_bound_receipt(&variant, &gate).unwrap());
    }
    let mut tampered = base.clone();
    tampered.digest = digest(99);
    assert!(!verify_bound_receipt(&tampered, &gate).unwrap());
}

#[test]
fn bound_receipt_rejects_wrong_role_even_with_recomputed_digest() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let mut forged = record.bound_receipt.clone().expect("approval receipt");
    forged.approver_role = ApproverRole::new(String::from("viewer")).expect("viewer role");
    forged.digest = compute_bound_receipt_digest(&forged).expect("recomputed digest");
    assert!(!verify_bound_receipt(&forged, &gate).unwrap());
}

#[test]
fn bound_replay_rejects_corrupted_receipt_digest() {
    let gate = bound_gate();
    let mut record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    record
        .bound_receipt
        .as_mut()
        .expect("approval receipt")
        .digest = digest(99);
    let events = vec![
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record),
    ];
    assert_eq!(
        replay_bound_gate_history(&events).unwrap_err(),
        GateError::BoundReceiptMismatch
    );
}

#[test]
fn bound_trust_direction_is_structurally_enforced() {
    let gate = bound_gate();
    let mut denied = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Deny),
    )
    .unwrap();
    denied.operator_trust = None;
    assert!(!denied.is_structurally_valid());

    let mut escalated = evaluate_bound_timeout(&gate, &person("bob"), 0, 2_000).unwrap();
    escalated.operator_trust = Some(OperatorTrust::InProcessCaller);
    assert!(!escalated.is_structurally_valid());
}

#[test]
fn bound_replay_ignores_decision_without_opening() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let decision = bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(1), record);
    assert!(replay_bound_gate_history(&[decision]).unwrap().is_empty());
}

#[test]
fn bound_path_rejects_zero_binding_and_legacy_path_stays_legacy() {
    let mut invalid = bound_gate();
    invalid.binding.attempt = 0;
    let err = evaluate_bound_decision(
        &invalid,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap_err();
    assert_eq!(err, GateError::InvalidGate);

    // A bound gate is not a legacy ManualGate; the old evaluator cannot
    // silently approve it because the binding is not part of the old type.
    let legacy = evaluate_decision(
        &invalid.gate,
        0,
        &person("bob"),
        &input(GateDecision::Approve),
    );
    assert_eq!(legacy.unwrap().outcome, GateOutcome::Approved);
}

#[test]
fn bound_replay_allows_gate_id_reuse_after_closure() {
    let gate = bound_gate();
    let first = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let mut second_input = input(GateDecision::Approve);
    second_input.decided_at_ms = 600;
    let second = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &second_input,
    )
    .unwrap();
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
    let replay = replay_bound_gate_history(&events).unwrap();
    assert_eq!(replay.len(), 2);
    assert_eq!(replay[0].outcome, GateOutcome::Approved);
    assert_eq!(replay[1].outcome, GateOutcome::Approved);
}

#[test]
fn bound_replay_rejects_decision_that_does_not_match_opening() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let mut changed = gate.clone();
    changed.binding.input_digest = digest(99);
    let events = vec![
        bound_gate_opened_event(changed, 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone()),
    ];
    let err = replay_bound_gate_history(&events).unwrap_err();
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
    let err = replay_bound_gate_history(&requester_changed).unwrap_err();
    assert_eq!(err, GateError::ReplayBoundGateMismatch { gate: 7 });
}

#[test]
fn bound_receipt_rejects_stale_binding_and_duplicate_replay() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let mut stale = gate.clone();
    stale.binding.input_digest = digest(99);
    assert!(!verify_bound_receipt(record.bound_receipt.as_ref().unwrap(), &stale).unwrap());

    let events = vec![
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone()),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(3), record),
    ];
    let err = replay_bound_gate_history(&events).unwrap_err();
    assert_eq!(err, GateError::ReplayDuplicateBoundReceipt { gate: 7 });
}

#[test]
fn bound_events_round_trip_and_legacy_replay_ignores_them() {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )
    .unwrap();
    let opened =
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1));
    let decision =
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record.clone());
    for event in [&opened, &decision] {
        let bytes = postcard::to_allocvec(event).unwrap();
        let decoded: vb_storage::JournalEvent = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(&decoded, event);
    }
    assert!(
        replay_gate_history(&[opened.clone(), decision.clone()])
            .unwrap()
            .is_empty()
    );
    let bound = replay_bound_gate_history(&[opened, decision]).unwrap();
    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0].outcome, GateOutcome::Approved);
    assert_eq!(
        bound[0].bound_receipt_digest,
        record.bound_receipt.as_ref().map(|r| r.digest)
    );
}

#[test]
fn bound_break_glass_still_requires_evidence_display() {
    let gate = bound_gate();
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.break_glass_justification = Some("prod is down".to_string());
    break_glass.evidence_shown = false;
    let err = evaluate_bound_decision(
        &gate,
        0,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &break_glass,
    )
    .unwrap_err();
    assert_eq!(err, GateError::EvidenceNotShown);
}
