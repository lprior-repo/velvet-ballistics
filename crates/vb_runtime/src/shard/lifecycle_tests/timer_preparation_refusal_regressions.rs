#[test]
fn timer_preparation_capacity_refusal_restores_checked_out_run_diagnostic() -> Result<(), String> {
    let run = RunId::new(86_030);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let timer = shard.pending_timer_remove(run).ok_or("initial timer")?;
    // Synthetic orphan timer fills the private boundary table while leaving
    // room to restore the real run. This is not public admission or an OOM test.
    let synthetic = RunId::new(86_031);
    shard
        .checked_out_run_insert(synthetic)
        .map_err(|error| error.to_string())?;
    shard
        .pending_timer_insert(synthetic, timer)
        .map_err(|error| error.to_string())?;
    shard.checked_out_run_remove(synthetic);
    shard.max_active_runs = 1;
    let state = shard.run_state_get(run).ok_or("waiting run")?.clone();
    let status = shard.runtime_state_get(run);
    let timers = shard.pending_timer_clone();
    let sequence = shard.journal_seq_get(run);
    let counters = shard.counters().snapshot();
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    let checked_out = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        shard.await_timer(run, checked_out, PendingTimerKind::Wait),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    ));
    assert_eq!(shard.run_state_get(run), Some(&state));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), status);
    assert_eq!(shard.pending_timer_clone(), timers);
    assert_eq!(shard.journal_seq_get(run), sequence);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum TimerPreflightDiagnostic {
    AskCapacity,
    SequenceOverflow,
    ChargedBytesOverflow,
}

// These tests inject private boundary conditions into an admitted wait fixture
// and call the internal transition directly. They establish restoration on the
// error branches, not public workflow reachability, allocator failure, or OOM.
fn timer_preflight_restoration_diagnostic(case: TimerPreflightDiagnostic) -> Result<(), String> {
    let run = RunId::new(86_040);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    let kind = match case {
        TimerPreflightDiagnostic::AskCapacity => {
            let step = shard.run_state_get(run).ok_or("waiting run")?.frame.pc();
            // Deliberately orphaned authority fills the ask table. Ask on this
            // wait fixture is an internal branch probe, not a valid public ask.
            shard.record_pending_ask_authority(
                RunId::new(86_041),
                step,
                vb_storage::EventSeq::ZERO,
            );
            shard.max_active_runs = 1;
            PendingTimerKind::Ask
        }
        TimerPreflightDiagnostic::SequenceOverflow => {
            shard
                .seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))
                .map_err(|error| error.to_string())?;
            PendingTimerKind::Wait
        }
        TimerPreflightDiagnostic::ChargedBytesOverflow => {
            shard
                .run_state_get_mut(run)
                .ok_or("waiting run")?
                .journal_bytes_written = u64::MAX;
            PendingTimerKind::Wait
        }
    };
    let state = shard.run_state_get(run).ok_or("waiting run")?.clone();
    let status = shard.runtime_state_get(run);
    let timers = shard.pending_timer_clone();
    let authorities = shard.pending_ask_authorities.clone();
    let sequence = shard.journal_seq_get(run);
    let counters = shard.counters().snapshot();
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    let checked_out = shard
        .take_run_state(run)
        .map_err(|error| error.to_string())?;
    let result = shard.await_timer(run, checked_out, kind);
    match case {
        TimerPreflightDiagnostic::AskCapacity => assert!(matches!(
            result,
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        )),
        TimerPreflightDiagnostic::SequenceOverflow => assert!(matches!(
            result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow)
        )),
        TimerPreflightDiagnostic::ChargedBytesOverflow => {
            assert!(matches!(result, Err(RuntimeError::EncodeFailed)));
        }
    }
    assert_eq!(shard.run_state_get(run), Some(&state));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), status);
    assert_eq!(shard.pending_timer_clone(), timers);
    assert_eq!(shard.pending_ask_authorities, authorities);
    assert_eq!(shard.journal_seq_get(run), sequence);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    Ok(())
}

#[test]
fn timer_ask_capacity_refusal_restores_owned_state_diagnostic() -> Result<(), String> {
    timer_preflight_restoration_diagnostic(TimerPreflightDiagnostic::AskCapacity)
}

#[test]
fn timer_sequence_overflow_restores_owned_state_diagnostic() -> Result<(), String> {
    timer_preflight_restoration_diagnostic(TimerPreflightDiagnostic::SequenceOverflow)
}

#[test]
fn timer_charged_bytes_overflow_restores_owned_state_diagnostic() -> Result<(), String> {
    timer_preflight_restoration_diagnostic(TimerPreflightDiagnostic::ChargedBytesOverflow)
}

#[test]
fn timer_preflight_rollback_collision_retains_both_errors_diagnostic() -> Result<(), String> {
    let run = RunId::new(86_042);
    let (mut shard, journal) = timer_due_regression_fixture(run)?;
    // Deliberately violate checkout ownership using a clone of a still-retained
    // run, and inject sequence exhaustion. This probes diagnostic error chaining,
    // not a publicly reachable duplicate-state or sequence-exhaustion scenario.
    shard
        .seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))
        .map_err(|error| error.to_string())?;
    let state = shard.run_state_get(run).ok_or("waiting run")?.clone();
    let status = shard.runtime_state_get(run);
    let timers = shard.pending_timer_clone();
    let authorities = shard.pending_ask_authorities.clone();
    let sequence = shard.journal_seq_get(run);
    let counters = shard.counters().snapshot();
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    assert!(!shard.checked_out_run_contains(run));
    let error = match shard.await_timer(run, state.clone(), PendingTimerKind::Wait) {
        Err(error) => error,
        Ok(_) => return Err("injected sequence exhaustion must fail".into()),
    };
    let RuntimeError::RollbackFailed {
        operation,
        primary,
        rollback,
    } = error
    else {
        return Err(format!(
            "expected both preflight and rollback errors: {error:?}"
        ));
    };
    assert_eq!(operation, "await timer preflight");
    assert!(
        matches!(primary.as_ref(), RuntimeError::StorageJournalAppend { source }
        if matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow))
    );
    assert!(matches!(rollback.as_ref(), RuntimeError::RunAlreadyExists));
    assert_eq!(shard.run_state_get(run), Some(&state));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), status);
    assert_eq!(shard.pending_timer_clone(), timers);
    assert_eq!(shard.pending_ask_authorities, authorities);
    assert_eq!(shard.journal_seq_get(run), sequence);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    Ok(())
}
