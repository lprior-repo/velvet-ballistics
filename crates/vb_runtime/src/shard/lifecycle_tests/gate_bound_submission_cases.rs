#[test]
fn open_bound_gate_rejects_each_zero_digest_before_journal()
-> Result<(), Box<dyn std::error::Error>> {
    for field in 0..3 {
        let run = RunId::new(94010 + field);
        let journal = BoundJournal::shared();
        let mut shard = Shard::new_with_journal(small_config(), journal.clone());
        let workflow = require_workflow("chain", chain_workflow())?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: CapabilitySet::empty(),
            action_contracts: Box::from([]),
            inputs: Box::from([]),
        })?;
        shard.tick()?;
        let mut gate = bound_gate(run);
        match field {
            0 => gate.binding.artifact_digest = GateDigest::from_bytes([0; 32]),
            1 => gate.binding.policy_digest = GateDigest::from_bytes([0; 32]),
            _ => gate.binding.input_digest = GateDigest::from_bytes([0; 32]),
        }
        shard.enqueue(ShardCommand::OpenBoundGate {
            gate,
            opened_at_ms: 0,
            requester: person("bob"),
        })?;
        let err = shard.tick().unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGateOpenedRecord));
        assert!(journal.opened()?.is_empty());
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    }
    Ok(())
}

#[test]
fn resume_rejects_bound_gate_without_decision() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94009);
    let (mut shard, journal) = make_shard(run)?;
    let err = shard
        .handle_resume(run)
        .expect_err("bound gate must block resume");
    assert!(matches!(
        err,
        crate::shard::types::ResumeError::GateOpen { gate, .. } if gate == GateId::new(7)
    ));
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    assert!(journal.decisions()?.is_empty(), "resume must not journal");
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_open_and_approve_journals_bound_record_and_closes_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94001);
    let (mut shard, journal) = make_shard(run)?;
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));

    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Approve),
        operator_trust: OperatorTrust::LocalSocketClaim,
    })?;
    shard.tick()?;

    let decisions = journal.decisions()?;
    assert_eq!(decisions.len(), 1);
    let decision = decisions.first().ok_or("decision record missing")?;
    assert!(decision.is_structurally_valid());
    assert_eq!(decision.record.outcome, GateOutcome::Approved);
    let openings = journal.opened()?;
    let opening = openings.first().ok_or("opening missing")?.3;
    assert_eq!(decision.opening_seq, opening);
    assert_eq!(
        decision
            .bound_receipt
            .as_ref()
            .ok_or("receipt missing")?
            .opening_seq,
        opening
    );
    assert_eq!(
        decision.operator_trust,
        Some(OperatorTrust::LocalSocketClaim)
    );
    assert_eq!(decision.binding, bound_gate_for_state(&shard, run)?.binding);
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn legacy_and_bound_paths_reject_each_others_gates() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94002);
    let (mut shard, _journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::SubmitGateDecision {
        run,
        input: input(GateDecision::Approve),
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::BoundGateRequired { gate: 7 }
        }
    ));
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    let resume_error = shard
        .handle_resume(run)
        .err()
        .ok_or("bound gate resumed without bound decision")?;
    assert!(matches!(
        resume_error,
        crate::shard::types::ResumeError::GateOpen { gate, .. } if gate == GateId::new(7)
    ));
    Ok(())
}

#[test]
fn bound_path_rejects_legacy_gate_without_binding() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94004);
    let journal = BoundJournal::shared();
    let mut shard = Shard::new_with_journal(small_config(), journal);
    let workflow = require_workflow("chain", chain_workflow())?;
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts: Box::from([]),
        inputs: Box::from([]),
    })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::OpenGate {
        gate: bound_gate(run).gate,
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Approve),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::LegacyGateWithoutBinding { gate: 7 }
        }
    ));
    Ok(())
}

#[test]
fn bound_deny_fails_run_without_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94005);
    let (mut shard, journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    let record = &journal.decisions()?[0];
    assert_eq!(record.record.outcome, GateOutcome::Denied);
    assert!(record.bound_receipt.is_none());
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_more_evidence_keeps_gate_and_original_deadline() -> Result<(), Box<dyn std::error::Error>>
{
    let run = RunId::new(94006);
    let (mut shard, journal) = make_shard(run)?;
    let mut more = input(GateDecision::RequestMoreEvidence);
    more.decided_at_ms = 1_000;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: more,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    let mut late = input(GateDecision::Approve);
    late.decided_at_ms = 1_001;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: late,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::DecisionAfterTimeout { .. }
        }
    ));
    assert_eq!(
        journal.decisions()?.len(),
        1,
        "late decision is not journaled"
    );
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_delegate_restricts_next_approver() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94007);
    let (mut shard, _journal) = make_shard(run)?;
    let mut delegate = input(GateDecision::Delegate);
    delegate.delegated_to = Some(person("carol"));
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: delegate,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    let wrong = input(GateDecision::Approve);
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: wrong,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(err, RuntimeError::GateWrongDelegate { .. }));
    let mut right = input(GateDecision::Approve);
    right.approver = person("carol");
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: right,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_timeout_escalation_fails_run_without_trust_or_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94008);
    let (mut shard, journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::EscalateBoundGate {
        run,
        gate: GateId::new(7),
        now_ms: 1_001,
    })?;
    shard.tick()?;
    let record = &journal.decisions()?[0];
    assert_eq!(record.record.outcome, GateOutcome::Escalated);
    assert!(record.operator_trust.is_none());
    assert!(record.bound_receipt.is_none());
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_break_glass_requires_evidence_before_journal() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94003);
    let (mut shard, journal) = make_shard(run)?;
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.break_glass_justification = Some("prod is down".to_string());
    break_glass.evidence_shown = false;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: break_glass,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::EvidenceNotShown
        }
    ));
    assert!(journal.decisions()?.is_empty());
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}
