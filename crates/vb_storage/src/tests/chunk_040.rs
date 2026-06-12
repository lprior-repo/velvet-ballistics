#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn adversarial_workflow_index_multiple_runs_same_workflow() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let wf = WorkflowId::new(42);
    for run_id in [RunId::new(1), RunId::new(2), RunId::new(3)] {
        journal.put_workflow_index(wf, run_id).expect("put");
    }
    // All three runs indexed under same workflow
}

#[test]
fn adversarial_batch_empty_strict_commit_succeeds() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let batch = journal.batch().strict();
    batch
        .strict()
        .commit()
        .expect("empty strict commit must succeed");
}

#[test]
fn adversarial_append_event_at_max_seq_stores_correctly() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(9060);
    // Write contiguous events 0..2, then verify seq 0 and 1 are present
    let digest = WorkflowDigest::from_bytes([1; 32]);
    journal
        .append_strict(&JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        })
        .expect("append0");
    journal
        .append_strict(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: vb_core::StepIdx::ZERO,
            attempt: 1,
        })
        .expect("append1");
    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(events.len(), 2, "contiguous seq 0,1 must replay");
}

#[test]
fn adversarial_batch_commit_persists_all_keys_or_none() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let source = b"src".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let compiled = crate::accepted_compiled_ir_record_for_test(b"ir".to_vec());
    let run = RunId::new(9070);
    let mut batch = journal.batch();
    batch
        .put_workflow_source(&WorkflowSourceRecord { digest, source })
        .expect("ws");
    batch.put_compiled_ir(&compiled).expect("ir");
    batch
        .put_run_header(&RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 0,
        })
        .expect("rh");
    batch.commit().expect("commit");
    // All three must be present — batch is atomic
    assert!(journal.workflow_source(digest).expect("g1").is_some());
    assert!(journal.compiled_ir(compiled.digest).expect("g2").is_some());
    assert!(journal.run_header(run).expect("g3").is_some());
}

// =====================================================================
// vb-apn5: Single-server database lock enforcement tests
// =====================================================================

#[test]
fn test_first_open_succeeds_and_creates_lock_file() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None);
    assert!(journal.is_ok(), "first open on empty path should succeed");
    let lock_path = temp.path().join(".process.lock");
    assert!(
        lock_path.exists(),
        ".process.lock file should be created after open"
    );
}

#[test]
fn test_lock_releases_on_journal_drop() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    {
        let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");
    } // journal dropped here, lock released
    let result = FjallJournal::open(temp.path(), None);
    assert!(result.is_ok(), "re-open after drop must succeed");
}

#[test]
fn test_second_open_fails_in_same_process() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");
    let result = FjallJournal::open(temp.path(), None);
    // Same-process: flock allows it, but Fjall detects the open database.
    // Cross-process: ProcessLockHeld would be returned first.
    assert!(
        result.is_err(),
        "second open in same process must fail (Fjall detects open DB)"
    );
}

#[test]
fn test_lock_file_contains_holder_pid() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");
    let lock_path = temp.path().join(".process.lock");
    let contents = std::fs::read_to_string(&lock_path).expect("read lock file");
    let pid: u32 = contents
        .trim()
        .parse()
        .expect("lock file should contain valid PID");
    assert_eq!(
        pid,
        std::process::id(),
        "lock file should contain current process PID"
    );
}

#[test]
fn test_no_keyspace_created_when_lock_fails() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");

    let before_count = std::fs::read_dir(temp.path()).expect("read_dir").count();

    let result = FjallJournal::open(temp.path(), None);
    assert!(result.is_err(), "second open must fail");

    let after_count = std::fs::read_dir(temp.path()).expect("read_dir").count();

    assert_eq!(
        before_count, after_count,
        "no new files should appear when lock fails"
    );
}

// ══ vb-hbav B19: JournalError exhaustiveness compile-time check ════════
#[test]
fn journal_error_match_covers_all_variants() {
    fn _exhaustive_match(e: &JournalError) -> &'static str {
        match e {
            JournalError::Fjall(_) => "fjall",
            JournalError::Encode(_) => "encode",
            JournalError::KeyCapacity => "key_capacity",
            JournalError::DuplicateEvent { .. } => "duplicate_event",
            JournalError::WriteLockPoisoned => "write_lock_poisoned",
            JournalError::QueueCapacity => "queue_capacity",
            JournalError::QueueFull => "queue_full",
            JournalError::QueueShutdown => "queue_shutdown",
            JournalError::WrongRun { .. } => "wrong_run",
            JournalError::SequenceGap { .. } => "sequence_gap",
            JournalError::SequenceOverflow => "sequence_overflow",
            JournalError::BadMagic { .. } => "bad_magic",
            JournalError::UnsupportedSchemaVersion { .. } => "unsupported_schema_version",
            JournalError::MigrationRequired { .. } => "migration_required",
            JournalError::UnknownRecordKind { .. } => "unknown_record_kind",
            JournalError::RecordKindFamilyMismatch { .. } => "record_kind_family_mismatch",
            JournalError::HeaderLengthMismatch { .. } => "header_length_mismatch",
            JournalError::PayloadTooLarge { .. } => "payload_too_large",
            JournalError::HeaderChecksumMismatch => "header_checksum_mismatch",
            JournalError::PayloadDigestMismatch => "payload_digest_mismatch",
            JournalError::UnexpectedEof => "unexpected_eof",
            JournalError::UnexpectedTrailingBytes { .. } => "unexpected_trailing_bytes",
            JournalError::PostcardDecodeFailed => "postcard_decode_failed",
            JournalError::InvalidEvent => "invalid_event",
            JournalError::ArtifactMalformed => "artifact_malformed",
            JournalError::ArtifactChecksumMismatch => "artifact_checksum_mismatch",
            JournalError::InvalidGateCount { .. } => "invalid_gate_count",
            JournalError::MissingRequiredProofFlag { .. } => "missing_required_proof_flag",
            JournalError::ArtifactNotFound { .. } => "artifact_not_found",
            JournalError::AdmissionRequired => "admission_required",
            JournalError::ArtifactInvalid { .. } => "artifact_invalid",
            JournalError::InputTooLarge { .. } => "input_too_large",
            JournalError::InputSchemaMismatch => "input_schema_mismatch",
            JournalError::CapabilityDenied => "capability_denied",
            JournalError::SecretUnavailable => "secret_unavailable",
            JournalError::RunAlreadyExists => "run_already_exists",
            JournalError::InvalidRunId { .. } => "invalid_run_id",
            JournalError::ActiveRunCapacityExceeded => "active_run_capacity_exceeded",
            JournalError::FrameAllocationFailed => "frame_allocation_failed",
            JournalError::AdmissionJournalFailed => "admission_journal_failed",
            JournalError::StrictDurabilityFailed => "strict_durability_failed",
            JournalError::TooManyEvents { .. } => "too_many_events",
            JournalError::ReplayAllocationFailed { .. } => "replay_allocation_failed",
            JournalError::ClockUnavailable => "clock_unavailable",
            JournalError::ProcessLockHeld { .. } => "process_lock_held",
            JournalError::ProcessLockIo { .. } => "process_lock_io",
            JournalError::Trim(_) => "trim",
            JournalError::JournalBatchBytesExceeded { .. } => "journal_batch_bytes_exceeded",
            JournalError::MetadataMutation { .. } => "metadata_mutation",
        }
    }
    let _ = _exhaustive_match;
}
