use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use vb_core::{RunId, RuntimePolicy, WorkflowDigest};
use vb_storage::{EventSeq, JournalError};

use crate::journal::{RuntimeJournal, RuntimeJournalEvent};
use crate::shard::{Shard, ShardConfig};
use crate::{RuntimeError, RuntimeResult};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
const MAX_RECORDED_EVENTS: usize = 4;

#[derive(Clone, Copy)]
enum AppendMode {
    Accept,
    RejectBeforeWrite,
    RecordThenError,
}

struct RecordingJournal {
    mode: AppendMode,
    calls: AtomicUsize,
    records: Mutex<Vec<(EventSeq, RuntimeJournalEvent)>>,
    source: Arc<JournalError>,
}

impl RecordingJournal {
    fn new(mode: AppendMode) -> Self {
        Self {
            mode,
            calls: AtomicUsize::new(0),
            records: Mutex::new(Vec::with_capacity(MAX_RECORDED_EVENTS)),
            source: Arc::new(JournalError::WriteLockPoisoned),
        }
    }

    fn failure(&self) -> RuntimeError {
        RuntimeError::StorageJournalAppend {
            source: self.source.clone(),
        }
    }

    fn snapshot(&self) -> RuntimeResult<Vec<(EventSeq, RuntimeJournalEvent)>> {
        let records = self
            .records
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        Ok(records.clone())
    }
}

impl RuntimeJournal for RecordingJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Err(RuntimeError::UnsupportedOperation {
            operation: "sequence_test_requires_sequenced_append",
        })
    }

    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(self.mode, AppendMode::RejectBeforeWrite) {
            return Err(self.failure());
        }
        let mut records = self
            .records
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        if records.len() >= MAX_RECORDED_EVENTS {
            return Err(RuntimeError::JournalFull {
                capacity: MAX_RECORDED_EVENTS,
            });
        }
        records
            .try_reserve(1)
            .map_err(|_| RuntimeError::JournalFull {
                capacity: MAX_RECORDED_EVENTS,
            })?;
        records.push((seq, event));
        drop(records);
        if matches!(self.mode, AppendMode::RecordThenError) {
            return Err(self.failure());
        }
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }
}

fn sequence_shard(mode: AppendMode) -> TestResult<(Shard, Arc<RecordingJournal>)> {
    let journal = Arc::new(RecordingJournal::new(mode));
    let config = ShardConfig::new(4, 16, 16, 1, RuntimePolicy::Relaxed)?;
    Ok((Shard::new_with_journal(config, journal.clone()), journal))
}

fn event(run: RunId) -> RuntimeJournalEvent {
    RuntimeJournalEvent::RunSubmitted {
        run,
        workflow: WorkflowDigest::from_bytes([0x7d; 32]),
    }
}

fn assert_original_adapter_error(
    result: RuntimeResult<()>,
    journal: &RecordingJournal,
) -> TestResult {
    match result {
        Err(RuntimeError::StorageJournalAppend { source }) => {
            assert!(Arc::ptr_eq(&source, &journal.source));
            Ok(())
        }
        other => Err(format!("expected the original adapter error, got {other:?}").into()),
    }
}

#[test]
fn new_run_appends_at_zero_then_publishes_one() -> TestResult {
    let (mut shard, journal) = sequence_shard(AppendMode::Accept)?;
    let run = RunId::new(8401);
    let event = event(run);
    assert!(shard.journal_sequences.is_empty());
    shard.append_journal_event(event.clone())?;
    assert_eq!(journal.calls.load(Ordering::SeqCst), 1);
    assert_eq!(journal.snapshot()?, vec![(EventSeq::ZERO, event)]);
    assert_eq!(shard.journal_sequences.get(&run), Some(&EventSeq::new(1)));
    assert_eq!(shard.journal_sequences.len(), 1);
    Ok(())
}

#[test]
fn existing_run_at_map_capacity_appends_then_advances() -> TestResult {
    let (mut shard, journal) = sequence_shard(AppendMode::Accept)?;
    let run = RunId::new(8402);
    shard.seed_journal_sequence(run, EventSeq::new(7))?;
    assert_eq!(shard.journal_sequences.len(), shard.max_active_runs);
    let event = event(run);
    shard.append_journal_event(event.clone())?;
    assert_eq!(journal.calls.load(Ordering::SeqCst), 1);
    assert_eq!(journal.snapshot()?, vec![(EventSeq::new(7), event)]);
    assert_eq!(shard.journal_sequences.get(&run), Some(&EventSeq::new(8)));
    assert_eq!(shard.journal_sequences.len(), 1);
    Ok(())
}

#[test]
fn successor_overflow_precedes_adapter_errors_without_an_append() -> TestResult {
    for mode in [AppendMode::Accept, AppendMode::RejectBeforeWrite] {
        let (mut shard, journal) = sequence_shard(mode)?;
        let run = RunId::new(8403);
        shard.seed_journal_sequence(run, EventSeq::new(u64::MAX))?;
        let before = shard.journal_sequences.clone();
        let result = shard.append_journal_event(event(run));
        assert!(matches!(
            result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), JournalError::SequenceOverflow)
        ));
        assert_eq!(journal.calls.load(Ordering::SeqCst), 0);
        assert!(journal.snapshot()?.is_empty());
        assert_eq!(shard.journal_sequences, before);
    }
    Ok(())
}

#[test]
fn full_map_new_run_refusal_precedes_adapter_errors() -> TestResult {
    for mode in [AppendMode::Accept, AppendMode::RejectBeforeWrite] {
        let (mut shard, journal) = sequence_shard(mode)?;
        let existing_run = RunId::new(8404);
        let new_run = RunId::new(8405);
        shard.seed_journal_sequence(existing_run, EventSeq::new(3))?;
        let before = shard.journal_sequences.clone();
        assert_eq!(shard.journal_sequences.len(), shard.max_active_runs);
        assert_eq!(
            shard.append_journal_event(event(new_run)),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
        assert_eq!(journal.calls.load(Ordering::SeqCst), 0);
        assert!(journal.snapshot()?.is_empty());
        assert_eq!(shard.journal_sequences, before);
        assert!(!shard.journal_sequences.contains_key(&new_run));
    }
    Ok(())
}

#[test]
fn pre_write_adapter_error_preserves_error_and_absent_sequence() -> TestResult {
    let (mut shard, journal) = sequence_shard(AppendMode::RejectBeforeWrite)?;
    let run = RunId::new(8406);
    assert_original_adapter_error(shard.append_journal_event(event(run)), &journal)?;
    assert_eq!(journal.calls.load(Ordering::SeqCst), 1);
    assert!(journal.snapshot()?.is_empty());
    // Reservation may grow backing capacity; no logical entry may be published.
    assert!(shard.journal_sequences.is_empty());
    assert!(!shard.journal_sequences.contains_key(&run));
    Ok(())
}

#[test]
fn append_then_error_keeps_record_without_advancing_sequence() -> TestResult {
    let (mut shard, journal) = sequence_shard(AppendMode::RecordThenError)?;
    let run = RunId::new(8407);
    shard.seed_journal_sequence(run, EventSeq::new(7))?;
    let before = shard.journal_sequences.clone();
    let event = event(run);
    assert_original_adapter_error(shard.append_journal_event(event.clone()), &journal)?;
    assert_eq!(journal.calls.load(Ordering::SeqCst), 1);
    assert_eq!(journal.snapshot()?, vec![(EventSeq::new(7), event)]);
    assert_eq!(shard.journal_sequences, before);
    // The recorded event is intentionally retained; this does not authorize retry.
    Ok(())
}
