
#[test]
fn bound_more_evidence_keeps_gate_and_original_deadline() -> Result<(), Box<dyn std::error::Error>>
{
    let run = RunId::new(94006);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
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
    crate::shard::helpers::timer::install_wall_clock_millis_for_test(1_001);
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
fn bound_decision_before_opening_is_rejected_before_journal()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94032);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut shard, journal) = running_shard(run)?;
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate: bound_gate_for_state(&shard, run)?,
        opened_at_ms: 500,
        requester: person("bob"),
    })?;
    shard.tick()?;
    let mut early = input(GateDecision::Approve);
    early.decided_at_ms = 499;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: early,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::DecisionBeforeOpen {
                opened_at_ms: 500,
                decided_at_ms: 499,
            }
        }
    ));
    assert!(journal.decisions()?.is_empty(), "pre-open decision is not journaled");
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_delegate_restricts_next_approver() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94007);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut shard, journal) = make_shard(run)?;
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
    assert_eq!(
        journal.decisions()?.len(),
        1,
        "wrong-delegate approval is not journaled"
    );
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
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
    let decisions = journal.decisions()?;
    let record = decisions.first().ok_or("expected one escalation record")?;
    assert_eq!(record.record.outcome, GateOutcome::Escalated);
    assert!(record.operator_trust.is_none());
    assert!(record.bound_receipt.is_none());
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert_eq!(journal.run_failed_count(run)?, 1);
    Ok(())
}

#[test]
fn bound_break_glass_requires_evidence_before_journal() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94003);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
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

#[test]
fn open_bound_gate_rejects_zero_attempt_before_journal() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94030);
    let (shard, journal) = running_shard(run)?;
    let mut gate = bound_gate(run);
    gate.binding.attempt = 0;
    let err = shard
        .enqueue(ShardCommand::OpenBoundGate {
            gate,
            opened_at_ms: 0,
            requester: person("bob"),
        })
        .expect_err("zero attempt must be rejected at enqueue");
    assert!(matches!(err, RuntimeError::InvalidGateOpenedRecord));
    assert_eq!(shard.command_queue_len(), 0);
    assert!(journal.opened()?.is_empty());
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    assert!(shard.bound_open_gates.is_empty());
    Ok(())
}

#[test]
fn open_bound_gate_rejects_deadline_overflow_at_max_opened_at_before_journal()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94031);
    let (shard, journal) = running_shard(run)?;
    let err = shard
        .enqueue(ShardCommand::OpenBoundGate {
            gate: bound_gate(run),
            opened_at_ms: u64::MAX,
            requester: person("bob"),
        })
        .expect_err("deadline overflow must be rejected at enqueue");
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::DeadlineOverflow,
        }
    ));
    assert_eq!(shard.command_queue_len(), 0);
    assert!(journal.opened()?.is_empty());
    assert!(shard.bound_open_gates.is_empty());
    Ok(())
}

#[test]
fn bound_decision_stamped_after_runtime_clock_is_rejected_by_precheck()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94041);
    let (shard, journal) = make_shard(run)?;
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(999);
    let mut future = input(GateDecision::Approve);
    future.decided_at_ms = 1_000;
    let err = shard
        .validate_open_bound_gate_decision(run, &future)
        .expect_err("future-stamped decision must be rejected by the fail-fast precheck");
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::TimestampFromFuture {
                timestamp_ms: 1_000,
                clock_ms: 999,
            }
        }
    ));
    assert_eq!(shard.command_queue_len(), 0);
    assert!(journal.decisions()?.is_empty(), "future-stamped decision is not journaled");
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_decision_stamped_exactly_at_runtime_clock_passes_enqueue_check()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94042);
    let (shard, _journal) = make_shard(run)?;
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let mut at_clock = input(GateDecision::Approve);
    at_clock.decided_at_ms = 1_000;
    shard.validate_open_bound_gate_decision(run, &at_clock)?;
    Ok(())
}

#[test]
fn bound_escalation_stamped_in_future_is_rejected_before_journal()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94030);
    let (shard, journal) = make_shard(run)?;
    assert!(matches!(
        shard.enqueue(ShardCommand::EscalateBoundGate {
            run,
            gate: GateId::new(7),
            now_ms: u64::MAX,
        }),
        Err(RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::TimestampFromFuture { .. }
        })
    ));
    assert!(journal.decisions()?.is_empty());
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert!(shard.run_state_contains(run));
    Ok(())
}
