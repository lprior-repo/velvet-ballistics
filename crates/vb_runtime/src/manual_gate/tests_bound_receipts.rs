#[test]
fn bound_approve_binds_binding_trust_and_v2_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::LocalSocketClaim,
        &input(GateDecision::Approve),
    )?;
    assert!(record.is_structurally_valid());
    assert!(record.matches_bound_gate(&gate));
    assert_eq!(record.binding, gate.binding);
    assert_eq!(record.operator_trust, Some(OperatorTrust::LocalSocketClaim));
    let receipt = record
        .bound_receipt
        .as_ref()
        .ok_or("approval receipt missing")?;
    assert_eq!(receipt.binding, gate.binding);
    assert_eq!(receipt.requester, person("bob"));
    assert_eq!(receipt.risk, gate.gate.risk);
    assert_eq!(receipt.separation, gate.gate.separation_of_duties);
    assert_eq!(receipt.action, gate.gate.action);
    assert_ne!(receipt.digest, GateDigest::ZERO);
    assert!(verify_bound_receipt(receipt, &gate)?);
    assert!(verify_receipt(&receipt.receipt, &gate.gate)?);
    Ok(())
}

#[test]
fn bound_break_glass_records_claimed_role_outside_required_role()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.approver_role =
        ApproverRole::new(String::from("viewer")).ok_or("viewer role invalid")?;
    break_glass.break_glass_justification = Some(String::from("prod is down"));
    break_glass.evidence_shown = true;
    let record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &break_glass,
    )?;
    assert_eq!(record.record.outcome, GateOutcome::BreakGlassApproved);
    let receipt = record
        .bound_receipt
        .as_ref()
        .ok_or("break-glass receipt missing")?;
    assert_eq!(receipt.approver_role.name(), "viewer");
    assert!(verify_bound_receipt(receipt, &gate)?);
    Ok(())
}

#[test]
fn bound_receipt_digest_is_deterministic_and_field_sensitive()
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
    let base = record
        .bound_receipt
        .as_ref()
        .ok_or("approval receipt missing")?;
    let base_digest = compute_bound_receipt_digest(base)?;
    assert_eq!(
        base_digest,
        compute_bound_receipt_digest(base)?,
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
    variant.approver_role =
        ApproverRole::new(String::from("viewer")).ok_or("viewer role invalid")?;
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
    variant.receipt.gate = GateId::new(8);
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.run = RunId::new(4);
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.decision = GateDecision::Deny;
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.approver = person("carol");
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.evidence_bundle_digest = digest(93);
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.prompt_digest = digest(94);
    changed.push(variant);
    let mut variant = base.clone();
    variant.opening_seq = 2;
    changed.push(variant);
    let mut variant = base.clone();
    variant.receipt.decided_at_ms = 501;
    changed.push(variant);
    for variant in changed {
        let changed_digest = compute_bound_receipt_digest(&variant)?;
        assert_ne!(base_digest, changed_digest);
        assert!(!verify_bound_receipt(&variant, &gate)?);
    }
    let mut tampered = base.clone();
    tampered.digest = digest(99);
    assert!(!verify_bound_receipt(&tampered, &gate)?);
    Ok(())
}

#[test]
fn bound_receipt_rejects_wrong_role_even_with_recomputed_digest()
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
    let mut forged = record
        .bound_receipt
        .clone()
        .ok_or("approval receipt missing")?;
    forged.approver_role =
        ApproverRole::new(String::from("viewer")).ok_or("viewer role invalid")?;
    forged.digest = compute_bound_receipt_digest(&forged)?;
    assert!(!verify_bound_receipt(&forged, &gate)?);
    Ok(())
}

#[test]
fn bound_replay_rejects_corrupted_receipt_digest() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut record = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    record
        .bound_receipt
        .as_mut()
        .ok_or("approval receipt missing")?
        .digest = digest(99);
    let events = vec![
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), record),
    ];
    assert_eq!(
        replay_bound_gate_history(&events)
            .err()
            .ok_or("corrupt receipt replay accepted")?,
        GateError::BoundReceiptMismatch
    );
    Ok(())
}

#[test]
fn bound_trust_direction_is_structurally_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut denied = evaluate_bound_decision(
        &gate,
        0,
        1,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Deny),
    )?;
    denied.operator_trust = None;
    assert!(!denied.is_structurally_valid());

    let mut escalated = evaluate_bound_timeout(&gate, &person("bob"), 0, 1, 2_000)?;
    escalated.operator_trust = Some(OperatorTrust::InProcessCaller);
    assert!(!escalated.is_structurally_valid());
    Ok(())
}
