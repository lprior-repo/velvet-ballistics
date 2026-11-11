use std::sync::atomic::{AtomicBool, Ordering};

struct OneShotWaitFailureJournal {
    inner: std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    armed: AtomicBool,
}

impl OneShotWaitFailureJournal {
    fn new(inner: std::sync::Arc<crate::journal::VolatileRuntimeJournal>) -> Self {
        Self {
            inner,
            armed: AtomicBool::new(false),
        }
    }

    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    fn fail_wait_resolution(&self, event: &crate::journal::RuntimeJournalEvent) -> bool {
        matches!(
            event,
            crate::journal::RuntimeJournalEvent::WaitResolved { .. }
        ) && self.armed.swap(false, Ordering::SeqCst)
    }

    fn fail_wait_batch(&self, events: &[crate::journal::RuntimeJournalEvent]) -> bool {
        events.iter().any(|event| {
            matches!(
                event,
                crate::journal::RuntimeJournalEvent::WaitResolved { .. }
            )
        }) && self.armed.swap(false, Ordering::SeqCst)
    }

    fn append_error() -> crate::RuntimeError {
        crate::RuntimeError::StorageJournalAppend {
            source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
        }
    }
}

impl crate::journal::RuntimeJournal for OneShotWaitFailureJournal {
    fn append(&self, event: crate::journal::RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        if self.fail_wait_resolution(&event) {
            return Err(Self::append_error());
        }
        self.inner.append(event)
    }

    fn append_sequenced(
        &self,
        event: crate::journal::RuntimeJournalEvent,
        seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        if self.fail_wait_resolution(&event) {
            return Err(Self::append_error());
        }
        self.inner.append_sequenced(event, seq)
    }

    fn append_sequenced_batch(
        &self,
        events: &[crate::journal::RuntimeJournalEvent],
        start_seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        if self.fail_wait_batch(events) {
            return Err(Self::append_error());
        }
        self.inner.append_sequenced_batch(events, start_seq)
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        self.inner.probe()
    }
}

fn wait_failure_due_deadline() -> Result<std::time::Instant, String> {
    let now = std::time::Instant::now();
    now.checked_sub(std::time::Duration::from_secs(30))
        .ok_or_else(|| String::from("due deadline must be representable"))
}

struct TimerRetryFixture {
    shard: Shard,
    journal: std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    low_run: RunId,
    high_run: RunId,
}

struct TimerRetrySnapshot {
    low: AuthoritySnapshot,
    high: AuthoritySnapshot,
    low_sequence: Option<vb_storage::EventSeq>,
    high_sequence: Option<vb_storage::EventSeq>,
}

fn timer_retry_fixture() -> Result<TimerRetryFixture, String> {
    let low_run = RunId::new(84_020);
    let high_run = RunId::new(84_021);
    let inner_journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let wrapper = std::sync::Arc::new(OneShotWaitFailureJournal::new(inner_journal.clone()));
    let shared: crate::journal::SharedRuntimeJournal = wrapper.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    timer_due_regression_submit_wait(&mut shard, low_run)?;
    timer_due_regression_submit_wait(&mut shard, high_run)?;
    let due = wait_failure_due_deadline()?;
    timer_due_regression_set_deadline(&mut shard, high_run, due)?;
    timer_due_regression_set_deadline(&mut shard, low_run, due)?;
    wrapper.arm();
    Ok(TimerRetryFixture {
        shard,
        journal: inner_journal,
        low_run,
        high_run,
    })
}

fn timer_retry_snapshot(fixture: &TimerRetryFixture) -> Result<TimerRetrySnapshot, String> {
    Ok(TimerRetrySnapshot {
        low: authority_snapshot(&fixture.shard, fixture.low_run, &fixture.journal)?,
        high: authority_snapshot(&fixture.shard, fixture.high_run, &fixture.journal)?,
        low_sequence: fixture
            .shard
            .journal_sequences
            .get(&fixture.low_run)
            .copied(),
        high_sequence: fixture
            .shard
            .journal_sequences
            .get(&fixture.high_run)
            .copied(),
    })
}

fn assert_wait_append_failure(result: &crate::RuntimeResult<bool>) {
    assert!(matches!(
        result,
        Err(crate::RuntimeError::StorageJournalAppend { source })
            if matches!(**source, vb_storage::JournalError::WriteLockPoisoned)
    ));
}

fn assert_failure_authorities_unchanged(
    fixture: &TimerRetryFixture,
    before: &TimerRetrySnapshot,
) -> Result<(), String> {
    assert_eq!(
        authority_snapshot(&fixture.shard, fixture.low_run, &fixture.journal)?,
        before.low
    );
    assert_eq!(
        authority_snapshot(&fixture.shard, fixture.high_run, &fixture.journal)?,
        before.high
    );
    Ok(())
}

fn assert_failure_sequences_unchanged(fixture: &TimerRetryFixture, before: &TimerRetrySnapshot) {
    assert_eq!(
        fixture
            .shard
            .journal_sequences
            .get(&fixture.low_run)
            .copied(),
        before.low_sequence
    );
    assert_eq!(
        fixture
            .shard
            .journal_sequences
            .get(&fixture.high_run)
            .copied(),
        before.high_sequence
    );
}

fn assert_first_failure_preserves_both(
    fixture: &mut TimerRetryFixture,
    before: &TimerRetrySnapshot,
) -> Result<(), String> {
    let result = fixture.shard.tick();
    assert_wait_append_failure(&result);
    assert_failure_authorities_unchanged(fixture, before)?;
    assert_failure_sequences_unchanged(fixture, before);
    assert_eq!(fixture.shard.pending_timer_count(), 2);
    Ok(())
}

fn assert_low_retry_preserves_high(
    fixture: &mut TimerRetryFixture,
    high_state: &crate::shard::RunState,
    high_timer: &crate::shard::PendingTimer,
) -> Result<(), String> {
    assert_eq!(fixture.shard.tick(), Ok(true));
    assert_eq!(fixture.shard.pending_timer_count(), 1);
    assert!(fixture.shard.terminal_runs_contains(fixture.low_run));
    assert_eq!(
        collect_wait_resolved_runs(&fixture.journal)?,
        vec![fixture.low_run]
    );
    assert_eq!(
        fixture.shard.run_state_get(fixture.high_run),
        Some(high_state)
    );
    assert_eq!(
        fixture.shard.pending_timer_get(fixture.high_run),
        Some(high_timer.clone())
    );
    Ok(())
}

fn assert_high_retry_is_exactly_once(fixture: &mut TimerRetryFixture) -> Result<(), String> {
    assert_eq!(fixture.shard.tick(), Ok(true));
    assert_eq!(fixture.shard.pending_timer_count(), 0);
    assert!(fixture.shard.terminal_runs_contains(fixture.high_run));
    assert_eq!(
        collect_wait_resolved_runs(&fixture.journal)?,
        vec![fixture.low_run, fixture.high_run]
    );
    let events = fixture
        .journal
        .snapshot()
        .map_err(|error| format!("{error:?}"))?;
    assert_idle_tick_preserves_wait_journal(
        &mut fixture.shard,
        &fixture.journal,
        fixture.high_run,
        &events,
    )?;
    Ok(())
}

#[test]
fn timer_wheel_due_handler_error_preserves_ownership_for_retry() -> Result<(), String> {
    let mut fixture = timer_retry_fixture()?;
    let before = timer_retry_snapshot(&fixture)?;
    let high_state = before.high.run_state.clone();
    let high_timer = fixture
        .shard
        .pending_timer_get(fixture.high_run)
        .ok_or("high timer must exist before failure")?;
    assert_first_failure_preserves_both(&mut fixture, &before)?;
    assert_low_retry_preserves_high(&mut fixture, &high_state, &high_timer)?;
    assert_high_retry_is_exactly_once(&mut fixture)?;
    Ok(())
}
