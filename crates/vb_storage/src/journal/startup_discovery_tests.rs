use super::*;
use crate::constants::{MAGIC_INDEX_RECORD, MAX_RUN_HEADER_BYTES, RUN_ONLY_KEY_BYTES};
use crate::keys::{run_event_key, run_header_key};
use crate::{EventSeq, JournalEvent, RecordKind, RunHeaderRecord, encode_record};
use vb_core::{StepIdx, WorkflowDigest, WorkflowId};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn header(id: u64) -> RunHeaderRecord {
    RunHeaderRecord {
        run: RunId::new(id),
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([7; 32]),
        status: 0,
        accepted_at_ms: 0,
    }
}

fn accepted(id: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(id),
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([7; 32]),
    }
}

fn nonzero(value: usize) -> TestResult<NonZeroUsize> {
    NonZeroUsize::new(value).ok_or_else(|| "nonzero fixture limit required".into())
}

fn header_bytes(item: &RunHeaderRecord, kind: RecordKind, sequence: u64) -> TestResult<Vec<u8>> {
    Ok(encode_record(
        MAGIC_INDEX_RECORD,
        kind,
        sequence,
        item,
        MAX_RUN_HEADER_BYTES,
    )?)
}

fn event_bytes(item: &JournalEvent, kind: RecordKind, sequence: u64) -> TestResult<Vec<u8>> {
    Ok(encode_record(
        MAGIC_JOURNAL_EVENT,
        kind,
        sequence,
        item,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?)
}

fn collect(journal: &FjallJournal, rows: usize, bytes: usize) -> TestResult<Vec<u64>> {
    let mut seen = Vec::new();
    journal.try_for_each_startup_run_bounded(nonzero(rows)?, nonzero(bytes)?, |run| {
        seen.push(run.get());
        Ok::<(), JournalError>(())
    })?;
    Ok(seen)
}

#[test]
fn startup_union_includes_header_only_event_only_and_overlap_once_in_order() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(3))?;
    journal.put_run_header(&header(1))?;
    journal.append_strict(&accepted(2))?;
    journal.append_strict(&accepted(3))?;
    journal.append_strict(&JournalEvent::StepStarted {
        run: RunId::new(2),
        seq: EventSeq::new(1),
        step: StepIdx::ZERO,
        attempt: 1,
    })?;
    assert_eq!(collect(&journal, 5, 4096)?, [1, 2, 3]);
    Ok(())
}

#[test]
fn combined_row_and_byte_limits_count_duplicate_ids_before_visiting() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let h = header_bytes(&header(1), RecordKind::RunHeader, 1)?;
    let e = event_bytes(&accepted(1), RecordKind::RunAccepted, 0)?;
    let total = h
        .len()
        .checked_add(e.len())
        .ok_or("fixture byte total overflow")?;
    journal
        .run_header
        .insert(run_header_key(RunId::new(1))?.to_vec(), h)?;
    journal
        .events
        .insert(run_event_key(RunId::new(1), EventSeq::ZERO)?.to_vec(), e)?;
    assert_eq!(collect(&journal, 2, total)?, [1]);
    for row_limited in [true, false] {
        let mut called = 0usize;
        let rows = if row_limited { 1 } else { 2 };
        let bytes = if row_limited {
            total
        } else {
            total
                .checked_sub(1)
                .ok_or("fixture byte total must be positive")?
        };
        let result =
            journal.try_for_each_startup_run_bounded(nonzero(rows)?, nonzero(bytes)?, |_run| {
                called = called
                    .checked_add(1)
                    .ok_or(JournalError::SequenceOverflow)?;
                Ok::<(), JournalError>(())
            });
        if row_limited {
            assert!(matches!(
                result,
                Err(StartupDiscoveryError::RowLimitExceeded { limit: 1 })
            ));
        } else {
            assert!(
                matches!(result, Err(StartupDiscoveryError::ByteLimitExceeded { limit }) if limit == bytes)
            );
        }
        assert_eq!(called, 0);
    }
    Ok(())
}

#[test]
fn corrupt_header_or_event_prevents_all_callbacks_after_valid_candidates() -> TestResult {
    for case in 0u8..=10 {
        let dir = tempfile::tempdir()?;
        let journal = FjallJournal::open(dir.path(), None)?;
        journal.put_run_header(&header(1))?;
        journal.append_strict(&accepted(1))?;
        if case <= 4 {
            let key = match case {
                0 => vec![0x7f],
                1 => vec![0xff; RUN_ONLY_KEY_BYTES],
                _ => run_header_key(RunId::new(2))?.to_vec(),
            };
            let item = header(if case == 2 { 3 } else { 2 });
            let kind = if case == 4 {
                RecordKind::RunInputs
            } else {
                RecordKind::RunHeader
            };
            let sequence = if case == 3 { 9 } else { item.run.get() };
            journal
                .run_header
                .insert(key, header_bytes(&item, kind, sequence)?)?;
        } else {
            let key = match case {
                5 => vec![0x7f],
                6 => vec![0xff; JOURNAL_KEY_BYTES],
                8 => run_event_key(RunId::new(2), EventSeq::new(1))?.to_vec(),
                _ => run_event_key(RunId::new(2), EventSeq::ZERO)?.to_vec(),
            };
            let item = accepted(if case == 7 { 3 } else { 2 });
            let kind = if case == 10 {
                RecordKind::RunFinished
            } else {
                RecordKind::RunAccepted
            };
            let sequence = if case == 9 { 9 } else { 0 };
            journal
                .events
                .insert(key, event_bytes(&item, kind, sequence)?)?;
        }
        let mut called = 0usize;
        let result =
            journal.try_for_each_startup_run_bounded(nonzero(3)?, nonzero(4096)?, |_run| {
                called = called
                    .checked_add(1)
                    .ok_or(JournalError::SequenceOverflow)?;
                Ok::<(), JournalError>(())
            });
        match case {
            0 | 1 | 5 | 6 => assert!(matches!(
                result,
                Err(StartupDiscoveryError::Storage(
                    JournalError::MalformedKeyspaceRow { .. }
                ))
            )),
            2 | 7 => assert!(
                matches!(result, Err(StartupDiscoveryError::Storage(JournalError::WrongRun { expected, actual })) if expected == RunId::new(2) && actual == RunId::new(3))
            ),
            3 => assert!(
                matches!(result, Err(StartupDiscoveryError::Storage(JournalError::ReplayEnvelopeSequenceMismatch { run, envelope_seq: 9, payload_seq: 2 })) if run == RunId::new(2))
            ),
            4 => assert!(
                matches!(result, Err(StartupDiscoveryError::Storage(JournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind })) if envelope_kind == RecordKind::RunInputs.id() && payload_kind == RecordKind::RunHeader.id())
            ),
            8 => assert!(
                matches!(result, Err(StartupDiscoveryError::Storage(JournalError::ReplayKeyMismatch { run, .. })) if run == RunId::new(2))
            ),
            9 => assert!(
                matches!(result, Err(StartupDiscoveryError::Storage(JournalError::ReplayEnvelopeSequenceMismatch { run, envelope_seq: 9, payload_seq: 0 })) if run == RunId::new(2))
            ),
            10 => assert!(
                matches!(result, Err(StartupDiscoveryError::Storage(JournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind })) if envelope_kind == RecordKind::RunFinished.id() && payload_kind == RecordKind::RunAccepted.id())
            ),
            _ => return Err("unexpected corruption case".into()),
        }
        assert_eq!(called, 0);
    }
    Ok(())
}

#[test]
fn visitors_observe_complete_pinned_union_despite_callback_writes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(1))?;
    journal.append_strict(&accepted(2))?;
    let mut seen = Vec::new();
    journal.try_for_each_startup_run_bounded(nonzero(2)?, nonzero(4096)?, |run| {
        if run == RunId::new(1) {
            journal.put_run_header(&header(3))?;
            journal.append_strict(&accepted(4))?;
        }
        seen.push(run.get());
        Ok::<(), JournalError>(())
    })?;
    assert_eq!(seen, [1, 2]);
    assert_eq!(collect(&journal, 4, 4096)?, [1, 2, 3, 4]);
    Ok(())
}

#[test]
fn visitor_refusal_is_returned_without_visiting_later_runs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(1))?;
    journal.append_strict(&accepted(2))?;
    let mut seen = Vec::new();
    let result = journal.try_for_each_startup_run_bounded(nonzero(2)?, nonzero(4096)?, |run| {
        seen.push(run.get());
        Err::<(), JournalError>(JournalError::SequenceOverflow)
    });
    assert!(matches!(
        result,
        Err(StartupDiscoveryError::Visitor(
            JournalError::SequenceOverflow
        ))
    ));
    assert_eq!(seen, [1]);
    Ok(())
}
