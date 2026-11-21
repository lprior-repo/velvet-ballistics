fn assert_recovers_closed(
    gate: &BoundManualGate,
    decided: vb_core::manual_gate::BoundGateOutcomeRecord,
    expected_outcome: GateOutcome,
    expected_decision: GateDecision,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_digest = decided
        .bound_receipt
        .as_ref()
        .map(|receipt| receipt.digest)
        .ok_or("decision receipt missing")?;
    let live_journal = vec![
        bound_gate_opened_event(gate.clone(), 0, person("bob"), vb_storage::EventSeq::new(1)),
        bound_gate_decision_event(gate.gate.run, vb_storage::EventSeq::new(2), decided),
    ];
    // Only the serialized journal survives the simulated crash; the live
    // in-memory events are dropped before recovery runs.
    let durable = live_journal
        .iter()
        .map(postcard::to_allocvec)
        .collect::<Result<Vec<Vec<u8>>, _>>()?;
    drop(live_journal);
    let recovered = durable
        .iter()
        .map(|bytes| postcard::from_bytes::<vb_storage::JournalEvent>(bytes))
        .collect::<Result<Vec<_>, _>>()?;

    assert!(
        reconstruct_bound_open_gates(&recovered)?.is_empty(),
        "decided gate must be closed after cold recovery"
    );
    let replay = replay_bound_gate_history(&recovered)?;
    assert_eq!(replay.len(), 1);
    let outcome = replay.first().ok_or("decision replay missing")?;
    assert_eq!(outcome.outcome, expected_outcome);
    assert_eq!(outcome.decision, Some(expected_decision));
    assert_eq!(outcome.bound_receipt_digest, Some(expected_digest));

    // Recovery is a pure fold over the journal: repeating it over the same
    // bytes yields identical state and produces no additional outcome.
    assert!(reconstruct_bound_open_gates(&recovered)?.is_empty());
    assert_eq!(replay_bound_gate_history(&recovered)?, replay);
    Ok(())
}

#[test]
fn bound_approval_recovers_closed_from_durable_bytes_alone()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let approved = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::LocalSocketClaim,
        &input(GateDecision::Approve),
    )?;
    assert_recovers_closed(
        &gate,
        approved,
        GateOutcome::Approved,
        GateDecision::Approve,
    )
}

#[test]
fn bound_break_glass_recovers_closed_from_durable_bytes_alone()
-> Result<(), Box<dyn std::error::Error>> {
    let gate = bound_gate();
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.break_glass_justification = Some(String::from("prod is down"));
    break_glass.evidence_shown = true;
    let broken = evaluate_bound_decision(
        &gate,
        OPENED_AT_MS,
        OPENING_SEQ,
        &person("bob"),
        OperatorTrust::LocalSocketClaim,
        &break_glass,
    )?;
    assert_recovers_closed(
        &gate,
        broken,
        GateOutcome::BreakGlassApproved,
        GateDecision::BreakGlass,
    )
}
