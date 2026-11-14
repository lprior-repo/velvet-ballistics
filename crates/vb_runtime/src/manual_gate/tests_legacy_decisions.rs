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
