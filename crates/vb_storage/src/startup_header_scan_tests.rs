use super::*;
use std::num::NonZeroUsize;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn header(id: u64) -> RunHeaderRecord {
    RunHeaderRecord {
        run: vb_core::RunId::new(id),
        workflow_id: vb_core::WorkflowId::new(1),
        compiled_digest: vb_core::WorkflowDigest::from_bytes([7; 32]),
        status: 0,
        accepted_at_ms: 0,
    }
}

fn limit() -> Result<NonZeroUsize, Box<dyn std::error::Error>> {
    NonZeroUsize::new(2).ok_or_else(|| "nonzero test limit".into())
}

#[test]
fn startup_scan_accepts_exact_cap_and_visits_key_order() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(2))?;
    journal.put_run_header(&header(1))?;
    let mut runs = Vec::new();
    journal.try_for_each_run_header_bounded(limit()?, |item| {
        runs.push(item.run.get());
        Ok::<(), JournalError>(())
    })?;
    assert_eq!(runs, [1, 2]);
    Ok(())
}

#[test]
fn startup_scan_cap_plus_one_calls_no_visitor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    for id in 1..=3 {
        journal.put_run_header(&header(id))?;
    }
    let mut called = false;
    let result = journal.try_for_each_run_header_bounded(limit()?, |_item| {
        called = true;
        Ok::<(), JournalError>(())
    });
    assert!(matches!(
        result,
        Err(RunHeaderScanError::LimitExceeded { limit: 2 })
    ));
    assert!(!called);
    Ok(())
}

#[test]
fn startup_scan_validates_all_key_header_identities_before_visiting() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(1))?;
    let mismatched = header(3);
    let bytes = crate::encode_record(
        MAGIC_INDEX_RECORD,
        crate::RecordKind::RunHeader,
        3,
        &mismatched,
        crate::constants::MAX_RUN_HEADER_BYTES,
    )?;
    journal
        .run_header
        .insert(run_header_key(vb_core::RunId::new(2))?.to_vec(), bytes)?;
    let mut called = false;
    let result = journal.try_for_each_run_header_bounded(limit()?, |_item| {
        called = true;
        Ok::<(), JournalError>(())
    });
    assert!(
        matches!(result, Err(RunHeaderScanError::Storage(JournalError::WrongRun { expected, actual })) if expected.get() == 2 && actual.get() == 3)
    );
    assert!(!called);
    Ok(())
}

#[test]
fn startup_scan_repeats_the_same_pinned_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(1))?;
    journal.put_run_header(&header(2))?;
    let mut seen = Vec::new();
    journal.try_for_each_run_header_bounded(limit()?, |item| {
        if item.run.get() == 1 {
            let mut changed = header(2);
            changed.compiled_digest = vb_core::WorkflowDigest::from_bytes([9; 32]);
            journal.put_run_header(&changed)?;
            journal.put_run_header(&header(3))?;
        }
        seen.push(item);
        Ok::<(), JournalError>(())
    })?;
    assert_eq!(seen, [header(1), header(2)]);
    assert_eq!(journal.run_headers()?.len(), 3);
    Ok(())
}

#[test]
fn startup_scan_rejects_wrong_kind_before_any_visitor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    journal.put_run_header(&header(1))?;
    let bytes = crate::encode_record(
        MAGIC_INDEX_RECORD,
        crate::RecordKind::RunInputs,
        2,
        &header(2),
        crate::constants::MAX_RUN_HEADER_BYTES,
    )?;
    journal
        .run_header
        .insert(run_header_key(vb_core::RunId::new(2))?.to_vec(), bytes)?;
    let mut callbacks = 0usize;
    let result = journal.try_for_each_run_header_bounded(limit()?, |_item| {
        callbacks = callbacks
            .checked_add(1)
            .ok_or(JournalError::SequenceOverflow)?;
        Ok::<(), JournalError>(())
    });
    assert!(matches!(result,
        Err(RunHeaderScanError::Storage(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        })) if envelope_kind == crate::RecordKind::RunInputs.id()
            && payload_kind == crate::RecordKind::RunHeader.id()
    ));
    assert_eq!(callbacks, 0);
    Ok(())
}
