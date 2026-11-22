use crate::shard::lifecycle::tests::journal_bytes_budget;
#[test]
fn open_bound_gate_rejects_each_zero_digest_before_journal()
-> Result<(), Box<dyn std::error::Error>> {
    for field in 0..3 {
        let run = RunId::new(94010 + field);
        let journal = BoundJournal::shared();
        let mut shard = Shard::new_with_journal(small_config(), journal.clone()).expect("valid test configuration");
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
        let err = shard
            .enqueue(ShardCommand::OpenBoundGate {
                gate,
                opened_at_ms: 0,
                requester: person("bob"),
            })
            .expect_err("zero digest must be rejected at enqueue");
        assert!(matches!(err, RuntimeError::InvalidGateOpenedRecord));
        assert_eq!(shard.command_queue_len(), 0);
        assert!(journal.opened()?.is_empty());
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
        assert!(shard.pending_action_get(run).is_none());
        assert!(shard.bound_open_gates.is_empty());
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
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
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
    assert_eq!(decision.opening_seq.get(), opening);
    assert_eq!(
        decision
            .bound_receipt
            .as_ref()
            .ok_or("receipt missing")?
            .opening_seq.get(),
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
    let mut shard = Shard::new_with_journal(small_config(), journal).expect("valid test configuration");
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
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut shard, journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    let decisions = journal.decisions()?;
    let record = decisions.first().ok_or("decision record missing")?;
    assert_eq!(record.record.outcome, GateOutcome::Denied);
    assert!(record.bound_receipt.is_none());
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert_eq!(journal.run_failed_count(run)?, 1);
    Ok(())
}
