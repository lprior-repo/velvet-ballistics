#[path = "events_filters/oracles.rs"]
mod oracles;
#[path = "events_filters/support.rs"]
mod support;
#[path = "events_filters/wire.rs"]
mod wire;

use support::{Fixture, TestResult};

#[test]
fn actual_issued_ask_journal_preserves_full_output_without_filters() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.assert_selection(&[], &[0, 1, 2, 3, 4, 5, 6, 7, 8])
}

#[test]
fn each_status_selects_current_trace_categories_without_reordering() -> TestResult {
    let fixture = Fixture::new()?;
    for (status, expected) in [
        ("pending", vec![0]),
        ("active", vec![4, 7]),
        ("waiting_answer", vec![8]),
        ("completed", vec![5, 6]),
        ("cancelled", vec![]),
        ("failed", vec![]),
    ] {
        fixture.assert_selection(&["--status", status], &expected)?;
    }
    Ok(())
}

#[test]
fn status_selection_precedes_maximum_count_and_zero_selection_succeeds() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.assert_selection(&["--status", "completed", "--limit", "1"], &[5])?;
    fixture.assert_selection(&["--status", "waiting_answer", "--limit", "1"], &[8])?;
    fixture.assert_selection(&["--status", "completed", "--limit", "0"], &[])?;
    fixture.assert_selection(&["--status", "failed", "--limit", "1"], &[])
}

#[test]
fn count_bounds_preserve_existing_negative_unbounded_behavior() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.assert_selection(&["--limit", "0"], &[])?;
    fixture.assert_selection(&["--limit", "1"], &[0])?;
    fixture.assert_selection(&["--limit", "9"], &[0, 1, 2, 3, 4, 5, 6, 7, 8])?;
    fixture.assert_selection(
        &["--limit", "9223372036854775807"],
        &[0, 1, 2, 3, 4, 5, 6, 7, 8],
    )?;
    fixture.assert_selection(&["--limit", "-1"], &[0, 1, 2, 3, 4, 5, 6, 7, 8])?;
    fixture.assert_selection(
        &["--status", "active", "--limit", "-9223372036854775808"],
        &[4, 7],
    )
}

#[test]
fn original_empty_run_remains_not_found_even_with_zero_limit() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("empty");
    drop(vb_storage::FjallJournal::open(&path, None)?);
    for emit in ["text", "yaml", "postcard"] {
        let output = support::events(&path, "77", emit, &["--limit", "0"])?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        oracles::assert_no_data(&output.stderr, emit)?;
    }
    assert!(
        vb_storage::FjallJournal::open(&path, None)?
            .events_for_run(vb_core::RunId::new(77))?
            .is_empty()
    );
    Ok(())
}

#[test]
fn invalid_run_and_missing_directory_remain_exact_errors_without_creation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("absent");
    for emit in ["text", "yaml", "postcard"] {
        let invalid = support::events(&path, "0", emit, &["--limit", "0"])?;
        oracles::assert_error(
            invalid,
            emit,
            2,
            "ValidationFailed",
            "invalid run_id '0': run_id must be non-zero",
        )?;
        let missing = support::events(&path, "1", emit, &["--status", "failed"])?;
        oracles::assert_error(
            missing,
            emit,
            5,
            "StorageError",
            &format!("journal directory does not exist: {}", path.display()),
        )?;
        assert!(!path.exists());
    }
    Ok(())
}

#[test]
fn real_storage_open_error_preserves_sentinel_bytes_and_failure_channel() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("sentinel");
    std::fs::write(&path, b"unchanged filter open-error sentinel")?;
    let error = vb_storage::FjallJournal::open(&path, None)
        .err()
        .ok_or("file opened as journal")?;
    assert!(matches!(&error, vb_storage::JournalError::Fjall(_)));
    let message = format!("error opening journal at {}: {error}", path.display());
    for emit in ["text", "yaml", "postcard"] {
        let output = support::events(&path, "1", emit, &["--limit", "0"])?;
        oracles::assert_error(output, emit, 5, "StorageError", &message)?;
        assert_eq!(
            std::fs::read(&path)?,
            b"unchanged filter open-error sentinel"
        );
    }
    Ok(())
}

#[test]
fn typed_terminal_evidence_has_nonempty_failed_and_cancelled_selections() -> TestResult {
    use vb_storage::{EventSeq, JournalEvent};
    let run = vb_core::RunId::new(1);
    let fixture = Fixture::new()?;
    let journal = vb_storage::FjallJournal::open(&fixture.path, None)?;
    journal.append_strict(&JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(9),
        reason: None,
        attempt: 1,
    })?;
    journal.append_strict(&JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(10),
        attempt: 1,
    })?;
    drop(journal);
    for (status, index) in [("cancelled", 9), ("failed", 10)] {
        fixture.assert_selection(&["--status", status], &[index])?;
        fixture.assert_selection(&["--status", status, "--limit", "1"], &[index])?;
        fixture.assert_selection(&["--status", status, "--limit", "0"], &[])?;
    }
    Ok(())
}

#[test]
fn process_lock_guard_preserves_existing_available_surface_and_records() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.records()?;
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .open(fixture.path.join(".process.lock"))?;
    lock.try_lock().map_err(|error| format!("{error:?}"))?;
    for emit in ["text", "yaml", "postcard"] {
        let output = support::events(
            &fixture.path,
            "1",
            emit,
            &["--status", "failed", "--limit", "0"],
        )?;
        oracles::assert_process_lock(output, emit)?;
    }
    drop(lock);
    assert_eq!(fixture.records()?, before);
    Ok(())
}

#[test]
fn active_fjall_writer_keeps_storage_error_before_filtering() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.records()?;
    let journal = vb_storage::FjallJournal::open(&fixture.path, None)?;
    let error = vb_storage::FjallJournal::open(&fixture.path, None)
        .err()
        .ok_or("second database writer opened")?;
    assert!(matches!(&error, vb_storage::JournalError::Fjall(_)));
    let message = format!(
        "error opening journal at {}: {error}",
        fixture.path.display()
    );
    for emit in ["text", "yaml", "postcard"] {
        let output = support::events(&fixture.path, "1", emit, &["--limit", "0"])?;
        oracles::assert_error(output, emit, 5, "StorageError", &message)?;
    }
    drop(journal);
    assert_eq!(fixture.records()?, before);
    Ok(())
}

#[test]
fn corrupt_owned_record_fails_after_open_even_when_limit_is_zero() -> TestResult {
    let fixture = Fixture::new()?;
    let journal = vb_storage::FjallJournal::open(&fixture.path, None)?;
    journal.inject_raw_event(
        vb_core::RunId::new(1),
        vb_storage::EventSeq::new(9),
        vb_storage::RecordKind::RunAccepted,
        &[],
    )?;
    journal.persist_strict()?;
    let error = journal
        .events_for_run(vb_core::RunId::new(1))
        .err()
        .ok_or("corrupt event was decoded")?;
    assert!(
        matches!(
            &error,
            vb_storage::JournalError::PostcardDecodeFailed(postcard::Error::SerdeDeCustom)
        ),
        "actual journal error: {error:?}"
    );
    drop(journal);
    let before = fixture.records()?;
    for emit in ["text", "yaml", "postcard"] {
        let output = support::events(
            &fixture.path,
            "1",
            emit,
            &["--status", "failed", "--limit", "0"],
        )?;
        oracles::assert_read_error(
            output,
            emit,
            &format!("error reading events for run 1: {error}"),
        )?;
        assert_eq!(fixture.records()?, before);
    }
    Ok(())
}
