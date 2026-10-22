#![forbid(unsafe_code)]
//! Emergency-deny marker storage seam tests (vb-ct31x).
//!
//! Validates the dedicated `emergency_deny` Fjall keyspace for the
//! emergency-deny overlay marker. Tests cover:
//! - New store is Inactive
//! - Activation writes and returns Active state
//! - Idempotent repeated activation (exact equality)
//! - Persistence across journal drop and reopen (cold restart)
//! - Store isolation
//! - Malformed marker detection (wrong length)
//! - Unsupported version detection
//! - Failed persist returns error, not success
//! - Retry after injected failure
//! - Already Active retry still performs strict persistence
//! - Original run event history unchanged after activation
//!
//! All tests use Result return and ?/let-else patterns.
//! Zero .expect(), .unwrap(), .panic(), unchecked indexing, or unchecked arithmetic.

use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::journal::EmergencyDenyState;
use crate::journal::FjallJournal;
use std::path::PathBuf;

fn test_journal() -> Result<(tempfile::TempDir, FjallJournal), JournalError> {
    let temp = tempfile::tempdir().map_err(|e| {
        JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("tempdir failed: {}", e),
        )))
    })?;
    let journal = FjallJournal::open(temp.path(), None)?;
    Ok((temp, journal))
}

#[test]
fn new_store_is_inactive() -> Result<(), JournalError> {
    let (_temp, journal) = test_journal()?;
    let state = journal.emergency_deny_state()?;
    assert_eq!(state, EmergencyDenyState::Inactive);
    Ok(())
}

#[test]
fn activate_returns_active_state() -> Result<(), JournalError> {
    let (_temp, journal) = test_journal()?;
    let state = journal.activate_emergency_deny()?;
    assert_eq!(state, EmergencyDenyState::Active);
    // Read back the same state
    let read = journal.emergency_deny_state()?;
    assert_eq!(read, state);
    Ok(())
}

#[test]
fn repeated_activation_is_idempotent() -> Result<(), JournalError> {
    let (_temp, journal) = test_journal()?;
    let first = journal.activate_emergency_deny()?;
    let second = journal.activate_emergency_deny()?;
    let third = journal.activate_emergency_deny()?;

    // All should be Active with exact equality
    assert_eq!(first, EmergencyDenyState::Active);
    assert_eq!(second, EmergencyDenyState::Active);
    assert_eq!(third, EmergencyDenyState::Active);
    assert_eq!(first, second);
    assert_eq!(second, third);

    // Final read shows active state
    let read = journal.emergency_deny_state()?;
    assert_eq!(read, EmergencyDenyState::Active);
    Ok(())
}

#[test]
fn marker_persists_across_journal_close() -> Result<(), JournalError> {
    let (temp, journal) = test_journal()?;
    let state = journal.activate_emergency_deny()?;
    assert_eq!(state, EmergencyDenyState::Active);

    // Close the journal by dropping
    let path = temp.path().to_owned();
    drop(journal);

    // Re-open and check state persisted
    let reopened = FjallJournal::open(path, None)?;
    let read = reopened.emergency_deny_state()?;
    assert_eq!(read, EmergencyDenyState::Active);
    Ok(())
}

#[test]
fn separate_stores_are_isolated() -> Result<(), JournalError> {
    let (temp1, journal1) = test_journal()?;
    let (temp2, journal2) = test_journal()?;
    let _ = temp1;

    journal1.activate_emergency_deny()?;

    // journal2 should still be inactive
    let state2 = journal2.emergency_deny_state()?;
    assert_eq!(state2, EmergencyDenyState::Inactive);
    Ok(())
}

#[test]
fn malformed_marker_wrong_length_errors() -> Result<(), JournalError> {
    let (temp, journal) = test_journal()?;
    let _ = temp;

    // Write a 5-byte value (wrong length, should be 1)
    let bad_value = [1u8, 0, 0, 0, 0];
    journal
        .emergency_deny
        .insert(b"active", bad_value)
        .map_err(JournalError::Fjall)?;

    // State read should error
    let result = journal.emergency_deny_state();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "wrong length should error",
            ))));
        }
    };
    match err {
        JournalError::EmergencyDenyMarkerMalformed => {}
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("expected Malformed error, got {:?}", other),
            ))));
        }
    }

    // Activation should also reject corrupt state without overwriting
    let result = journal.activate_emergency_deny();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "activation should reject corrupt state",
            ))));
        }
    };
    match err {
        JournalError::EmergencyDenyMarkerMalformed => {}
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("expected Malformed error on activation, got {:?}", other),
            ))));
        }
    }

    // Verify raw bad bytes preserved (not silently overwritten)
    let stored = journal
        .emergency_deny
        .get(b"active")
        .map_err(JournalError::Fjall)?;
    if stored.is_none() {
        return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "expected bad marker to still be present",
        ))));
    }
    let stored_bytes = stored.as_ref().map(|v| v.to_vec());
    assert_eq!(
        stored_bytes.as_deref().map(|b| b.len()),
        Some(5),
        "expected 5 bytes preserved"
    );
    Ok(())
}

#[test]
fn unsupported_version_errors() -> Result<(), JournalError> {
    let (temp, journal) = test_journal()?;
    let _ = temp;

    // Write version 99 (unsupported) with correct length (1 byte)
    let bad_marker = [99u8];
    journal
        .emergency_deny
        .insert(b"active", bad_marker)
        .map_err(JournalError::Fjall)?;

    // State read should error with exact version
    let result = journal.emergency_deny_state();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unsupported version should error",
            ))));
        }
    };
    match err {
        JournalError::EmergencyDenyMarkerUnsupportedVersion { version } => {
            assert_eq!(version, 99, "expected version 99");
        }
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("expected UnsupportedVersion error, got {:?}", other),
            ))));
        }
    }

    // Activation should also reject unsupported version without overwriting
    let result = journal.activate_emergency_deny();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "activation should reject unsupported version",
            ))));
        }
    };
    match err {
        JournalError::EmergencyDenyMarkerUnsupportedVersion { version } => {
            assert_eq!(version, 99, "expected version 99 on activation");
        }
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "expected UnsupportedVersion error on activation, got {:?}",
                    other
                ),
            ))));
        }
    }

    // Verify raw bad bytes preserved
    let stored = journal
        .emergency_deny
        .get(b"active")
        .map_err(JournalError::Fjall)?;
    if stored.is_none() {
        return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "expected bad marker to still be present",
        ))));
    }
    let stored_bytes = stored.as_ref().map(|v| v.to_vec());
    assert_eq!(
        stored_bytes.as_deref().map(|b| b.len()),
        Some(1),
        "expected 1 byte preserved"
    );
    if let Some(bytes) = stored_bytes {
        assert_eq!(bytes.first(), Some(&99), "expected [99] preserved");
    }
    Ok(())
}

#[test]
fn failed_persist_returns_error_not_success() -> Result<(), JournalError> {
    let (temp, journal) = test_journal()?;
    let _ = temp;

    // Inject fault: force batch commit to fail (PRECOMMIT failure, not fsync)
    journal.fail_next_batch_commit_for_test();

    let result = journal.activate_emergency_deny();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed persist should return error",
            ))));
        }
    };
    match err {
        JournalError::Fjall(_) => {}
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("expected Fjall error, got {:?}", other),
            ))));
        }
    }

    // Verify state is still Inactive (activation didn't succeed)
    let read = journal.emergency_deny_state()?;
    assert_eq!(read, EmergencyDenyState::Inactive);
    Ok(())
}

#[test]
fn retry_after_injected_failure() -> Result<(), JournalError> {
    let (temp, journal) = test_journal()?;
    let _ = temp;

    // Inject fault: force batch commit to fail
    journal.fail_next_batch_commit_for_test();

    // First activation fails
    let result = journal.activate_emergency_deny();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "first activation should fail",
            ))));
        }
    };
    match err {
        JournalError::Fjall(_) => {}
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("expected Fjall error, got {:?}", other),
            ))));
        }
    }

    // Retry succeeds
    let retry_result = journal.activate_emergency_deny()?;
    assert_eq!(retry_result, EmergencyDenyState::Active);

    // Verify state persisted
    let read = journal.emergency_deny_state()?;
    assert_eq!(read, EmergencyDenyState::Active);
    Ok(())
}

#[test]
fn already_active_retry_performs_strict_persistence() -> Result<(), JournalError> {
    let (temp, journal) = test_journal()?;
    let _ = temp;

    // First activation succeeds
    let first = journal.activate_emergency_deny()?;
    assert_eq!(first, EmergencyDenyState::Active);

    // Inject fault for retry activation
    journal.fail_next_batch_commit_for_test();

    // Retry activation should still attempt persistence (not shortcut)
    let result = journal.activate_emergency_deny();
    let err = match result {
        Err(e) => e,
        Ok(_) => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "retry activation should fail on injected fault",
            ))));
        }
    };
    match err {
        JournalError::Fjall(_) => {}
        other => {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("expected Fjall error on retry, got {:?}", other),
            ))));
        }
    }

    // Real retry after fault clears
    let retry = journal.activate_emergency_deny()?;
    assert_eq!(retry, EmergencyDenyState::Active);

    // Verify state still Active after reopen
    let path = temp.path().to_owned();
    drop(journal);
    let reopened = FjallJournal::open(path, None)?;
    let read = reopened.emergency_deny_state()?;
    assert_eq!(read, EmergencyDenyState::Active);
    Ok(())
}

#[test]
fn activation_does_not_corrupt_run_events() -> Result<(), JournalError> {
    use crate::EventSeq;
    use vb_core::RunId;

    let (temp, journal) = test_journal()?;
    let _ = temp;

    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    };

    // Append a run event before activation
    journal.append_journaled(&event)?;

    // Activate emergency deny
    journal.activate_emergency_deny()?;

    // Verify run event is still intact with exact event vector
    let events = journal.events_for_run(run)?;
    assert_eq!(events.len(), 1, "expected 1 event");
    assert_eq!(
        events.first(),
        Some(&event),
        "expected exact event identity"
    );
    Ok(())
}
