#[test]
fn bound_deny_journal_failure_keeps_gate_open_and_run_live()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94006);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut shard, journal) = make_shard(run)?;
    journal
        .reject_failing_run
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let submit = shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    });
    submit?;
    let err = shard.tick().err().ok_or("tick must fail")?;
    assert!(matches!(
        err,
        RuntimeError::UnsupportedOperation {
            operation: "injected_failing_run_batch"
        }
    ));
    assert!(journal.decisions()?.is_empty());
    assert_eq!(journal.run_failed_count(run)?, 0);
    assert!(shard.run_state_contains(run));
    assert!(!shard.terminal_runs_contains(run));
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_deny_budget_exceeds_fails_run_with_run_failed_only()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94007);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut shard, journal) = make_shard(run)?;
    let (written, attempt) = {
        let state = shard.run_state_get(run).ok_or("run state")?;
        (state.journal_bytes_written, state.run_attempt.get())
    };
    let failed_len = shard.journal_event_bytes(&RuntimeJournalEvent::RunFailed { run, attempt })?;
    let state = shard.run_state_get_mut(run).ok_or("run state")?;
    let admission = state.admission.take().ok_or("run admission")?;
    state.admission =
        Some(admission.with_attached_budget(journal_bytes_budget(written + failed_len)));
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().err().ok_or("tick must fail")?;
    assert!(matches!(
        err,
        RuntimeError::GateJournalBudgetExceeded { gate } if gate == GateId::new(7)
    ));
    assert!(journal.decisions()?.is_empty());
    assert_eq!(journal.run_failed_count(run)?, 1);
    assert!(journal.events()?.iter().any(|event| matches!(
        event,
        RuntimeJournalEvent::RunFailed { run: r, attempt: a } if *r == run && *a == attempt
    )));
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_deny_retry_after_injected_failure_records_once()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94008);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut shard, journal) = make_shard(run)?;
    journal
        .reject_failing_run
        .store(true, std::sync::atomic::Ordering::SeqCst);
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().err().ok_or("tick must fail")?;
    assert!(matches!(
        err,
        RuntimeError::UnsupportedOperation {
            operation: "injected_failing_run_batch"
        }
    ));
    journal
        .reject_failing_run
        .store(false, std::sync::atomic::Ordering::SeqCst);
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    assert_eq!(journal.decisions()?.len(), 1);
    assert_eq!(journal.run_failed_count(run)?, 1);
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_timeout_journal_failure_keeps_gate_open_and_run_live()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94012);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_001);
    let (mut shard, journal) = make_shard(run)?;
    journal
        .reject_failing_run
        .store(true, std::sync::atomic::Ordering::SeqCst);
    shard.enqueue(ShardCommand::EscalateBoundGate {
        run,
        gate: GateId::new(7),
        now_ms: 1_001,
    })?;
    let err = shard.tick().err().ok_or("tick must fail")?;
    assert!(matches!(
        err,
        RuntimeError::UnsupportedOperation {
            operation: "injected_failing_run_batch"
        }
    ));
    assert!(journal.decisions()?.is_empty());
    assert_eq!(journal.run_failed_count(run)?, 0);
    assert!(shard.run_state_contains(run));
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_timeout_budget_exceeds_fails_run_with_run_failed_only()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94013);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_001);
    let (mut shard, journal) = make_shard(run)?;
    let (written, attempt) = {
        let state = shard.run_state_get(run).ok_or("run state")?;
        (state.journal_bytes_written, state.run_attempt.get())
    };
    let failed_len = shard.journal_event_bytes(&RuntimeJournalEvent::RunFailed { run, attempt })?;
    let state = shard.run_state_get_mut(run).ok_or("run state")?;
    let admission = state.admission.take().ok_or("run admission")?;
    state.admission =
        Some(admission.with_attached_budget(journal_bytes_budget(written + failed_len)));
    shard.enqueue(ShardCommand::EscalateBoundGate {
        run,
        gate: GateId::new(7),
        now_ms: 1_001,
    })?;
    let err = shard.tick().err().ok_or("tick must fail")?;
    assert!(matches!(
        err,
        RuntimeError::GateJournalBudgetExceeded { gate } if gate == GateId::new(7)
    ));
    assert!(journal.decisions()?.is_empty());
    assert_eq!(journal.run_failed_count(run)?, 1);
    assert!(journal.events()?.iter().any(|event| matches!(
        event,
        RuntimeJournalEvent::RunFailed { run: r, attempt: a } if *r == run && *a == attempt
    )));
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_open_budget_exceeds_refuses_before_journal_or_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94014);
    let (mut shard, journal) = running_shard(run)?;
    let written = shard
        .run_state_get(run)
        .ok_or("run state")?
        .journal_bytes_written;
    let state = shard.run_state_get_mut(run).ok_or("run state")?;
    let admission = state.admission.take().ok_or("run admission")?;
    state.admission = Some(admission.with_attached_budget(journal_bytes_budget(written)));
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate: bound_gate_for_state(&shard, run)?,
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    let err = shard.tick().err().ok_or("tick must fail")?;
    assert!(matches!(
        err,
        RuntimeError::GateJournalBudgetExceeded { gate } if gate == GateId::new(7)
    ));
    assert!(journal.opened()?.is_empty());
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert!(shard.run_state_contains(run));
    assert_eq!(journal.run_failed_count(run)?, 0);
    Ok(())
}

#[test]
fn bound_failure_strict_budget_exceeded_writes_nothing_and_keeps_sequence()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94080);
    let (mut shard, journal) = make_shard(run)?;
    let (written, attempt) = {
        let state = shard.run_state_get(run).ok_or("run state")?;
        (state.journal_bytes_written, state.run_attempt.get())
    };
    let failed_len = shard.journal_event_bytes(&RuntimeJournalEvent::RunFailed { run, attempt })?;
    let state = shard.run_state_get_mut(run).ok_or("run state")?;
    let admission = state.admission.take().ok_or("run admission")?;
    let budget = written.checked_add(failed_len).ok_or("budget overflow")?;
    state.admission = Some(admission.with_attached_budget(journal_bytes_budget(budget)));
    let open = shard
        .bound_open_gates
        .get(&GateId::new(7))
        .ok_or("gate open")?;
    let record = crate::manual_gate::evaluate_bound_decision(
        &open.gate,
        open.opened_at_ms,
        open.opening_seq,
        &open.requester,
        OperatorTrust::InProcessCaller,
        &input(GateDecision::Deny),
    )?;
    let sequence_before = shard.journal_sequence_for(run);
    assert_eq!(shard.append_bound_gate_failure_strict(record)?, None);
    assert_eq!(shard.journal_sequence_for(run), sequence_before);
    assert!(journal.decisions()?.is_empty());
    assert_eq!(journal.run_failed_count(run)?, 0);
    let state = shard.run_state_get(run).ok_or("run state removed")?;
    assert_eq!(state.journal_bytes_written, written);
    assert_eq!(state.run_attempt.get(), attempt);
    assert!(!shard.terminal_runs_contains(run));
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}
