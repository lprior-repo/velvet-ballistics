#[test]
fn deny_is_terminal_without_approval_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let denied = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Deny),
    )?;
    assert_eq!(denied.record.outcome, GateOutcome::Denied);
    assert!(denied.record.receipt.is_none());
    assert!(denied.bound_receipt.is_none());
    let mut journal = opened_journal(&gate);
    journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        denied,
    ));
    // Denial closes the gate as denied; it never recovers as an approval.
    assert!(reconstruct_bound_open_gates(&journal)?.is_empty());
    let replay = replay_bound_gate_history(&journal)?;
    assert_eq!(replay.len(), 1);
    let outcome = replay.first().ok_or("deny replay missing")?;
    assert_eq!(outcome.outcome, GateOutcome::Denied);
    assert_eq!(outcome.bound_receipt_digest, None);
    Ok(())
}

#[test]
fn request_more_evidence_keeps_gate_pending_without_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let more = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::RequestMoreEvidence),
    )?;
    assert_eq!(more.record.outcome, GateOutcome::MoreEvidenceRequested);
    assert!(more.record.receipt.is_none());
    assert!(more.bound_receipt.is_none());
    let mut journal = opened_journal(&gate);
    journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        more,
    ));
    assert_opening_still_pending(&journal)?;
    let replay = replay_bound_gate_history(&journal)?;
    assert_eq!(replay.len(), 1);
    let outcome = replay.first().ok_or("more-evidence replay missing")?;
    assert_eq!(outcome.outcome, GateOutcome::MoreEvidenceRequested);
    assert_eq!(outcome.bound_receipt_digest, None);
    Ok(())
}

#[test]
fn delegate_records_target_but_leaves_no_approval_receipt() -> Result<(), Box<dyn std::error::Error>>
{
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
    assert_eq!(delegated.record.outcome, GateOutcome::Delegated);
    assert!(delegated.record.receipt.is_none());
    assert!(delegated.bound_receipt.is_none());
    let mut journal = opened_journal(&gate);
    journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        delegated,
    ));
    let open = reconstruct_bound_open_gates(&journal)?;
    assert_eq!(open.len(), 1, "delegation must not close the gate");
    assert_eq!(
        open.first()
            .and_then(|pending| pending.delegated_to.clone()),
        Some(person("carol"))
    );
    let replay = replay_bound_gate_history(&journal)?;
    assert_eq!(replay.len(), 1);
    let outcome = replay.first().ok_or("delegation replay missing")?;
    assert_eq!(outcome.outcome, GateOutcome::Delegated);
    assert_eq!(outcome.bound_receipt_digest, None);
    Ok(())
}

#[test]
fn replay_rejects_approval_by_non_delegate_and_accepts_delegate()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let gate_id = gate.gate.id.get();
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
    let mut outsider = input(GateDecision::Approve);
    outsider.approver = person("dave");
    let by_outsider = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &outsider,
    )?;
    let mut delegate = input(GateDecision::Approve);
    delegate.approver = person("carol");
    let by_delegate = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &delegate,
    )?;

    let mut outsider_journal = opened_journal(&gate);
    outsider_journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        delegated.clone(),
    ));
    outsider_journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(3),
        by_outsider,
    ));
    assert_eq!(
        replay_bound_gate_history(&outsider_journal).err(),
        Some(GateError::ReplayWrongDelegate { gate: gate_id })
    );

    let mut delegate_journal = opened_journal(&gate);
    delegate_journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        delegated,
    ));
    delegate_journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(3),
        by_delegate,
    ));
    assert_eq!(replay_bound_gate_history(&delegate_journal)?.len(), 2);
    Ok(())
}

#[test]
fn foreign_role_approval_is_rejected_and_gate_stays_pending()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut foreign = input(GateDecision::Approve);
    foreign.approver_role = ApproverRole::new("viewer".to_string()).ok_or("viewer role invalid")?;
    let err = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::LocalSocketClaim,
        &foreign,
    )
    .err()
    .ok_or("foreign-role approval was accepted")?;
    assert_eq!(
        err,
        GateError::WrongRole {
            required: "deploy-approver".to_string(),
            actual: "viewer".to_string(),
        }
    );
    let journal = opened_journal(&gate);
    assert_opening_still_pending(&journal)?;
    assert!(replay_bound_gate_history(&journal)?.is_empty());
    Ok(())
}

#[test]
fn zero_binding_is_rejected_at_decision_and_recovery() {
    let mut zero_artifact = bound_gate();
    zero_artifact.binding.artifact_digest = GateDigest::ZERO;
    let mut zero_policy = bound_gate();
    zero_policy.binding.policy_digest = GateDigest::ZERO;
    let mut zero_input = bound_gate();
    zero_input.binding.input_digest = GateDigest::ZERO;
    let mut zero_attempt = bound_gate();
    zero_attempt.binding.attempt = 0;

    for invalid in [zero_artifact, zero_policy, zero_input, zero_attempt] {
        assert_eq!(
            evaluate_bound_decision(
                &invalid,
                OPENED_AT_MS,
                OPENING_SEQ,
                &person("bob"),
                OperatorTrust::InProcessCaller,
                &input(GateDecision::Approve),
            )
            .err(),
            Some(GateError::InvalidGate)
        );
        let opened = bound_gate_opened_event(
            invalid.clone(),
            0,
            person("bob"),
            vb_storage::EventSeq::new(1),
        );
        let slice = std::slice::from_ref(&opened);
        assert_eq!(
            reconstruct_bound_open_gates(slice).err(),
            Some(GateError::InvalidGate)
        );
        assert_eq!(
            replay_bound_gate_history(slice).err(),
            Some(GateError::InvalidGate)
        );
    }
}
