#![forbid(unsafe_code)]
//! Emergency-deny marker storage seam tests (vb-ct31x).
//!
//! Validates the dedicated `emergency_deny` Fjall keyspace for the
//! monotonic emergency-deny overlay marker. Tests cover:
//! - New store is Inactive
//! - Activation writes and returns Active state
//! - Idempotent repeated activation
//! - Persistence across journal drop and reopen (cold restart)
//! - Store isolation
//! - Malformed marker detection (wrong length)
//! - Unsupported version detection
//! - Failed persist returns error, not success
//! - Original run event history unchanged after activation

use crate::events::JournalEvent;
use crate::error::JournalError;
use crate::journal::core::EmergencyDenyState;
use crate::journal::FjallJournal;
use std::sync::atomic::Ordering;

fn test_journal() -> Result<FjallJournal, JournalError> {
    let temp = tempfile::tempdir().map_err(|e| {
        JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("tempdir failed: {}", e),
        )))
    })?;
    let journal = FjallJournal::open(temp.path(), None)?;
    Ok(journal)
}

#[test]
fn new_store_is_inactive() {
    let journal = test_journal().expect("setup: journal");
    let state = journal.emergency_deny_state().expect("state read");
    assert_eq!(state, EmergencyDenyState::Inactive);
}

#[test]
fn activate_returns_active_state() {
    let journal = test_journal().expect("setup: journal");
    let state = journal.activate_emergency_deny().expect("activate");
    match state {
        EmergencyDenyState::Active { seq } => assert!(seq > 0, "sequence should be positive"),
        _ => panic!("expected Active state"),
    }
    // Read back the same state
    let read = journal.emergency_deny_state().expect("state read");
    assert_eq!(read, state);
}

#[test]
fn repeated_activation_is_idempotent() {
    let journal = test_journal().expect("setup: journal");
    let first = journal.activate_emergency_deny().expect("first activate");
    let second = journal.activate_emergency_deny().expect("second activate");
    let third = journal.activate_emergency_deny().expect("third activate");

    // All should be Active
    match (first, second, third) {
        (
            EmergencyDenyState::Active { seq: s1 },
            EmergencyDenyState::Active { seq: s2 },
            EmergencyDenyState::Active { seq: s3 },
        ) => {
            // Each activation should increment the sequence (monotonic)
            assert!(s1 < s2, "sequence should be monotonic");
            assert!(s2 < s3, "sequence should be monotonic");
        }
        _ => panic!("all activations should return Active"),
    }

    // Final read shows latest active state
    let read = journal.emergency_deny_state().expect("state read");
    assert_eq!(read, third);
}

#[test]
fn marker_persists_across_journal_close() {
    // Cannot test true close without owning journal through caller,
    // so test via dropping and re-opening same directory.
    let temp = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("open");
    let state = journal.activate_emergency_deny().expect("activate");

    // Close the journal
    let path = temp.path().to_owned();
    drop(journal);

    // Re-open and check state persisted
    let reopened = FjallJournal::open(path, None).expect("reopen");
    let read = reopened.emergency_deny_state().expect("state read");
    assert_eq!(read, state, "state should persist across close/reopen");
}

#[test]
fn separate_stores_are_isolated() {
    let temp1 = tempfile::tempdir().expect("setup: tempdir1");
    let journal1 = FjallJournal::open(temp1.path(), None).expect("open1");

    let temp2 = tempfile::tempdir().expect("setup: tempdir2");
    let journal2 = FjallJournal::open(temp2.path(), None).expect("open2");

    journal1.activate_emergency_deny().expect("activate1");

    // journal2 should still be inactive
    let state2 = journal2.emergency_deny_state().expect("state2");
    assert_eq!(state2, EmergencyDenyState::Inactive);
}

#[test]
fn malformed_marker_wrong_length_errors() {
    let temp = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("open");

    // Write a 5-byte value (wrong length, should be 9)
    let bad_value = [1u8, 0, 0, 0, 0];
    journal
        .emergency_deny_keyspace()
        .insert(b"active", bad_value)
        .expect("insert bad marker");

    let result = journal.emergency_deny_state();
    assert!(result.is_err(), "wrong length should error");
    match result {
        Err(JournalError::EmergencyDenyMarkerMalformed) => {}
        other => panic!("expected Malformed error, got {:?}", other),
    }
}

#[test]
fn unsupported_version_errors() {
    let temp = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("open");

    // Write version 99 (unsupported) with correct length
    let mut bad_marker = [0u8; 9];
    bad_marker[0] = 99;
    journal
        .emergency_deny_keyspace()
        .insert(b"active", bad_marker)
        .expect("insert bad marker");

    let result = journal.emergency_deny_state();
    assert!(result.is_err(), "unsupported version should error");
    match result {
        Err(JournalError::EmergencyDenyMarkerUnsupportedVersion { version }) => {
            assert_eq!(version, 99);
        }
        other => panic!("expected UnsupportedVersion error, got {:?}", other),
    }
}

#[test]
fn failed_persist_returns_error_not_success() {
    let journal = test_journal().expect("setup: journal");

    // Inject fault: force batch commit to fail
    journal.fail_next_batch_commit_for_test();

    let result = journal.activate_emergency_deny();
    assert!(result.is_err(), "failed persist should return error");

    // Verify state is still Inactive (activation didn't succeed)
    let read = journal.emergency_deny_state().expect("state read");
    assert_eq!(read, EmergencyDenyState::Inactive, "state should be Inactive after failed persist");
}

#[test]
fn activation_does_not_corrupt_run_events() {
    let journal = test_journal().expect("setup: journal");

    use crate::EventSeq;
    use vb_core::RunId;

    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0; 32]),
    };

    // Append a run event before activation
    journal
        .append_journaled(&event)
        .expect("append event");

    // Activate emergency deny
    journal.activate_emergency_deny().expect("activate");

    // Verify run event is still intact
    let events = journal.events_for_run(run).expect("replay events");
    assert_eq!(events.len(), 1, "run event should still be present");
}
