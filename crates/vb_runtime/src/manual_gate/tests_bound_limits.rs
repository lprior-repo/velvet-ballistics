#[test]
fn largest_attempt_is_a_valid_binding_for_deny_and_approve()
-> Result<(), Box<dyn std::error::Error>> {
    // `attempt` is a u16 with no overflow error; the boundary is u16::MAX.
    // It is accepted: a denial there mints no receipt, and an approval there
    // mints a receipt whose binding and replayed digest carry u16::MAX.
    let mut gate = bound_gate();
    gate.binding.attempt = u16::MAX;
    let denied = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Deny),
    )?;
    assert!(denied.bound_receipt.is_none());
    let mut deny_journal = opened_journal(&gate);
    deny_journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        denied,
    ));
    let deny_replay = replay_bound_gate_history(&deny_journal)?;
    assert_eq!(
        deny_replay
            .first()
            .map(|outcome| (outcome.outcome, outcome.bound_receipt_digest.is_none())),
        Some((GateOutcome::Denied, true))
    );
    let approved = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Approve),
    )?;
    let receipt = approved
        .bound_receipt
        .as_ref()
        .ok_or("approval at u16::MAX minted no receipt")?;
    assert_eq!(receipt.binding.attempt, u16::MAX);
    let expected_digest = receipt.digest.clone();
    let mut approve_journal = opened_journal(&gate);
    approve_journal.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        approved,
    ));
    let approve_replay = replay_bound_gate_history(&approve_journal)?;
    let replayed = approve_replay.first().ok_or("approval replay missing")?;
    assert_eq!(replayed.bound_receipt_digest, Some(expected_digest));
    Ok(())
}

#[test]
fn overflowing_deadline_opening_is_rejected_at_admission_recovery_and_replay()
-> Result<(), Box<dyn std::error::Error>> {
    // An opening whose opened_at + timeout_ms overflows has no decision window.
    // It must fail closed when journal state is rebuilt, never recover as open.
    let gate = bound_gate();
    let opened_at = u64::MAX
        .checked_sub(100)
        .ok_or("fixture opening underflow")?;
    assert_eq!(
        evaluate_bound_decision(
            &gate,
            opened_at,
            OPENING_SEQ,
            &person("bob"),
            OperatorTrust::InProcessCaller,
            &input(GateDecision::Approve),
        )
        .err(),
        Some(GateError::DeadlineOverflow)
    );
    assert_eq!(
        evaluate_bound_timeout(&gate, &person("bob"), opened_at, 1, u64::MAX).err(),
        Some(GateError::DeadlineOverflow)
    );
    let journal = vec![bound_gate_opened_event(
        gate.clone(),
        opened_at,
        person("bob"),
        vb_storage::EventSeq::new(1),
    )];
    assert!(matches!(
        reconstruct_bound_open_gates(&journal),
        Err(GateError::DeadlineOverflow)
    ));
    assert_eq!(
        replay_bound_gate_history(&journal).err(),
        Some(GateError::DeadlineOverflow)
    );
    Ok(())
}

#[test]
fn deadline_exact_fit_recovers_and_one_past_fails_closed() -> Result<(), Box<dyn std::error::Error>>
{
    // deadline = opened_at + timeout_ms; the latest opening with a representable
    // deadline is u64::MAX - timeout_ms, and one millisecond later must fail closed.
    let gate = bound_gate();
    let timeout = gate.gate.timeout_ms;
    let exact = u64::MAX
        .checked_sub(timeout)
        .ok_or("fixture timeout exceeds u64::MAX")?;
    let one_past = exact
        .checked_add(1)
        .ok_or("exact opening already u64::MAX")?;

    // The decision lands at the inclusive deadline, the latest instant the window admits.
    let mut decision = input(GateDecision::Approve);
    decision.decided_at_ms = u64::MAX;
    let approved = evaluate_bound_decision(
        &gate,
        exact,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::InProcessCaller,
        &decision,
    )?;
    assert_eq!(approved.record.outcome, GateOutcome::Approved);
    let mut fits = vec![bound_gate_opened_event(
        gate.clone(),
        exact,
        person("bob"),
        vb_storage::EventSeq::new(1),
    )];
    assert_eq!(reconstruct_bound_open_gates(&fits)?.len(), 1);
    fits.push(bound_gate_decision_event(
        gate.gate.run,
        vb_storage::EventSeq::new(2),
        approved,
    ));
    let replay = replay_bound_gate_history(&fits)?;
    assert_eq!(
        replay.first().map(|outcome| outcome.outcome),
        Some(GateOutcome::Approved)
    );

    assert_eq!(
        evaluate_bound_decision(
            &gate,
            one_past,
            OPENING_SEQ,
            &person("bob"),
            OperatorTrust::InProcessCaller,
            &input(GateDecision::Approve),
        )
        .err(),
        Some(GateError::DeadlineOverflow)
    );
    let overflowing = vec![bound_gate_opened_event(
        gate.clone(),
        one_past,
        person("bob"),
        vb_storage::EventSeq::new(1),
    )];
    assert!(matches!(
        reconstruct_bound_open_gates(&overflowing),
        Err(GateError::DeadlineOverflow)
    ));
    assert_eq!(
        replay_bound_gate_history(&overflowing).err(),
        Some(GateError::DeadlineOverflow)
    );
    Ok(())
}

#[test]
fn empty_requester_identity_is_rejected_before_any_decision_is_minted()
-> Result<(), Box<dyn std::error::Error>> {
    // `ApproverId::new` refuses empty identities, but the field is public and
    // serde can produce one; the decision path must refuse it, not journal it.
    let gate = bound_gate();
    let empty_requester = ApproverId {
        value: String::new(),
    };
    assert_eq!(
        evaluate_bound_decision(
            &gate,
            OPENED_AT_MS,
            OPENING_SEQ,
            &empty_requester,
            OperatorTrust::InProcessCaller,
            &input(GateDecision::Approve),
        )
        .err(),
        Some(GateError::InvalidRequester)
    );
    Ok(())
}
