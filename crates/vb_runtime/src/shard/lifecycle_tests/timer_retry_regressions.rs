struct TimerResolutionFailsOnce {
    inner: std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    armed: std::sync::atomic::AtomicBool,
}

impl TimerResolutionFailsOnce {
    fn refuse(&self, events: &[RuntimeJournalEvent]) -> crate::RuntimeResult<()> {
        if events
            .iter()
            .any(|event| matches!(event, RuntimeJournalEvent::WaitResolved { .. }))
            && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(RuntimeError::StorageJournalAppend {
                source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
            });
        }
        Ok(())
    }
}

impl crate::journal::RuntimeJournal for TimerResolutionFailsOnce {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        self.refuse(std::slice::from_ref(&event))?;
        self.inner.append(event)
    }

    fn append_sequenced(
        &self,
        event: RuntimeJournalEvent,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        self.refuse(std::slice::from_ref(&event))?;
        self.inner.append_sequenced(event, seq)
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        self.refuse(events)?;
        self.inner.append_sequenced_batch(events, seq)
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        self.inner.probe()
    }
}

#[test]
fn due_wait_append_refusal_preserves_both_runs_then_retries_exactly_once() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let wrapper = std::sync::Arc::new(TimerResolutionFailsOnce {
        inner: journal.clone(),
        armed: std::sync::atomic::AtomicBool::new(false),
    });
    let mut shard = Shard::new_with_journal(small_config(), wrapper.clone()).expect("valid test configuration");
    let runs = [RunId::new(84_020), RunId::new(84_021)];
    let due = std::time::Instant::now();
    runs.iter().try_for_each(|run| {
        timer_due_regression_submit_wait(&mut shard, *run)?;
        timer_due_regression_set_deadline(&mut shard, *run, due).map(|_| ())
    })?;
    let before = runs
        .iter()
        .map(|run| {
            Ok((
                shard.run_state_get(*run).ok_or("waiting state")?.clone(),
                shard.pending_timer_get(*run),
                shard.journal_seq_get(*run),
                shard.runtime_state_get(*run),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let counters = shard.counters().snapshot();
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    wrapper
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(
        matches!(shard.tick(), Err(RuntimeError::StorageJournalAppend { source })
        if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned))
    );
    for (run, (state, timer, seq, status)) in runs.iter().zip(before.iter()) {
        assert_eq!(shard.run_state_get(*run), Some(state));
        assert_eq!(shard.pending_timer_get(*run), *timer);
        assert_eq!(shard.journal_seq_get(*run), *seq);
        assert_eq!(shard.runtime_state_get(*run), *status);
        assert!(!shard.terminal_runs_contains(*run));
    }
    assert_eq!(shard.pending_timer_count(), 2);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 1);
    for (run, (state, timer, seq, status)) in runs.iter().zip(before.iter()) {
        if let Some(survivor) = shard.run_state_get(*run) {
            assert_eq!(survivor, state);
            assert_eq!(shard.pending_timer_get(*run), *timer);
            assert_eq!(shard.journal_seq_get(*run), *seq);
            assert_eq!(shard.runtime_state_get(*run), *status);
            assert!(!shard.terminal_runs_contains(*run));
        }
    }
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_timer_count(), 0);
    let resolved = journal.snapshot().map_err(|error| error.to_string())?;
    for run in runs {
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.run_state_get(run), None);
        assert_eq!(resolved.iter().filter(|event| matches!(event,
            RuntimeJournalEvent::WaitResolved { run: found, step } if *found == run && *step == StepIdx::new(1))).count(), 1);
    }
    let counters = shard.counters().snapshot();
    assert_eq!(counters.runs_completed, 2);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        resolved
    );
    assert_eq!(shard.counters().snapshot(), counters);
    Ok(())
}
