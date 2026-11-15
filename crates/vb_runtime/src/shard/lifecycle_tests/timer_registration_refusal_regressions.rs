struct TimerScheduleFailsOnce {
    inner: std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    armed: std::sync::atomic::AtomicBool,
}

impl TimerScheduleFailsOnce {
    fn refuse(&self, events: &[RuntimeJournalEvent]) -> crate::RuntimeResult<()> {
        if events.iter().any(|event| {
            matches!(
                event,
                RuntimeJournalEvent::WaitScheduled { .. }
                    | RuntimeJournalEvent::AskScheduled { .. }
            )
        }) && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(RuntimeError::StorageJournalAppend {
                source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
            });
        }
        Ok(())
    }
}

impl crate::journal::RuntimeJournal for TimerScheduleFailsOnce {
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
fn private_timer_registration_append_refusal_preserves_old_and_peer_then_retries_once()
-> Result<(), String> {
    let target = RunId::new(86_100);
    let peer = RunId::new(86_101);
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let wrapper = std::sync::Arc::new(TimerScheduleFailsOnce {
        inner: journal.clone(),
        armed: std::sync::atomic::AtomicBool::new(false),
    });
    let mut shard = Shard::new_with_journal(small_config(), wrapper.clone())
        .map_err(|error| error.to_string())?;
    let future = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or("shared future deadline")?;
    for run in [target, peer] {
        timer_due_regression_submit_wait(&mut shard, run)?;
        timer_due_regression_set_deadline(&mut shard, run, future)?;
    }
    let before = [target, peer]
        .into_iter()
        .map(|run| {
            Ok((
                run,
                authority_snapshot(&shard, run, &journal)?,
                shard.timer_entry(run),
                shard.runtime_state_get(run),
                shard.journal_seq_get(run),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let target_timer = shard.pending_timer_get(target).ok_or("target timer")?;
    let peer_timer = shard.pending_timer_get(peer).ok_or("peer timer")?;
    let target_token = shard.timer_entry(target).ok_or("target token")?;
    let peer_token = shard.timer_entry(peer).ok_or("peer token")?;
    wrapper
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let state = shard
        .take_run_state(target)
        .map_err(|error| error.to_string())?;
    // Private registration retry from an actual submitted waiting run; not cold recovery or OOM.
    assert!(
        matches!(shard.await_timer(target, state, PendingTimerKind::Wait),
        Err(RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned))
    );
    assert!(!shard.checked_out_run_contains(target));
    assert_eq!(
        shard.run_aggregate.pending_timer_due_peek(future),
        Some(target_token)
    );
    for (run, snapshot, token, status, sequence) in &before {
        assert_eq!(authority_snapshot(&shard, *run, &journal)?, *snapshot);
        assert_eq!(shard.timer_entry(*run), *token);
        assert_eq!(shard.runtime_state_get(*run), *status);
        assert_eq!(shard.journal_seq_get(*run), *sequence);
    }
    assert_eq!(shard.pending_timer_get(target), Some(target_timer));
    assert_eq!(shard.pending_timer_get(peer), Some(peer_timer));
    let state = shard
        .take_run_state(target)
        .map_err(|error| error.to_string())?;
    shard
        .await_timer(target, state, PendingTimerKind::Wait)
        .map_err(|error| error.to_string())?;
    let replacement = shard.pending_timer_get(target).ok_or("replacement timer")?;
    assert_eq!(
        replacement.generation,
        target_timer
            .generation
            .checked_add(1)
            .ok_or("next generation")?
    );
    assert!(replacement.deadline < future);
    assert_eq!(shard.pending_timer_count(), 2);
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(target));
    assert_eq!(shard.run_state_get(target), None);
    assert_eq!(shard.pending_timer_get(target), None);
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.pending_timer_get(peer), Some(peer_timer));
    let (_, peer_before, _, peer_status, peer_sequence) = before.last().ok_or("peer snapshot")?;
    assert_eq!(
        shard.run_aggregate.pending_timer_due_peek(future),
        Some(peer_token)
    );
    assert_eq!(shard.run_state_get(peer), Some(&peer_before.run_state));
    assert_eq!(shard.runtime_state_get(peer), *peer_status);
    assert_eq!(shard.journal_seq_get(peer), *peer_sequence);
    let after = journal.snapshot().map_err(|error| error.to_string())?;
    assert_eq!(
        after.get(..peer_before.journal_events.len()),
        Some(peer_before.journal_events.as_slice())
    );
    assert_eq!(wait_resolution_count(&after, target), 1);
    assert_eq!(wait_resolution_count(&after, peer), 0);
    let target_schedule_count = |events: &[RuntimeJournalEvent]| {
        events.iter().filter(|event|
        matches!(event, RuntimeJournalEvent::WaitScheduled { run, .. } if *run == target)
    ).count()
    };
    assert_eq!(
        target_schedule_count(&after),
        target_schedule_count(&peer_before.journal_events)
            .checked_add(1)
            .ok_or("schedule count")?
    );
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    let counters = shard.counters().snapshot();
    let boundary = shard.pending_boundary_snapshot(0, 4);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_boundary_snapshot(0, 4), boundary);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        after
    );
    Ok(())
}
