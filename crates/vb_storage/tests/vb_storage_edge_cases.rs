#![forbid(unsafe_code)]
//! Edge-case tests for vb_storage recovery and journal append boundaries.
//!
//! Covers three verified gap areas that lack dedicated tests:
//! 1. Non-contiguous sequence validation in replay
//! 2. Duplicate events with mismatched payloads (corruption scenario)
//! 3. Header-only run recovery (RunHeader with zero events)
//!
//! Run with:
//!   cargo test -p vb_storage --test vb_storage_edge_cases

use vb_core::{RunId, WorkflowDigest};
use vb_storage::constants::DIGEST_BYTES;
use vb_storage::recovery::{replay_events, recover_runtime_frame_seed, ActionReplayTracker};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, JournalError, RunHeaderRecord};
use vb_storage::recovery::RecoveryError;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("temp_journal: tempdir creation must succeed");
    let journal =
        FjallJournal::open(temp.path(), None).expect("temp_journal: journal open must succeed");
    (temp, journal)
}

fn run_accepted(run: RunId, seq: u64, workflow: [u8; DIGEST_BYTES]) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes(workflow),
    }
}

fn step_started(run: RunId, seq: u64, step: u16, attempt: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: vb_core::StepIdx::new(step),
        attempt,
    }
}

/// Run `replay_events` against the given event slice with no ABI digests.
/// Returns the observed result.
fn replay_with_tracker(events: Vec<JournalEvent>) -> Result<Vec<JournalEvent>, RecoveryError> {
    let mut tracker = ActionReplayTracker::new();
    replay_events(&events, &mut tracker, &[])
}

// ===========================================================================
// GAP 1: validate_contiguous_sequences — non-contiguous sequence holes
// ===========================================================================

/// A sequence with a single hole (0 → 2) must be rejected as a
/// `RecoveryError::ReplayDivergence`. All existing recovery tests use
/// fully contiguous [0, 1, 2, 3] so this gap was never covered.
#[test]
fn validate_contiguous_sequences_rejects_hole_at_position_1() {
    let run = RunId::new(10);
    let events: Vec<JournalEvent> = vec![
        run_accepted(run, 0, [0xAA; DIGEST_BYTES]),
        // seq 1 is intentionally skipped — hole at position 1
        step_started(run, 2, 0, 1),
        step_started(run, 3, 1, 1),
    ];

    let result = replay_with_tracker(events);

    assert!(
        result.is_err(),
        "non-contiguous sequence must be rejected, but got Ok"
    );
    let err = result.unwrap_err();
    match &err {
        RecoveryError::ReplayDivergence { detail, .. } => {
            assert!(
                detail.contains("sequence violation"),
                "expected ReplayDivergence with sequence violation detail, got detail = {detail:?}"
            );
        }
        other => panic!("expected ReplayDivergence, got {:?}", other),
    }
}

/// When the sequence overflows (u64::MAX followed by any next seq), the
/// validator detects the overflow via `checked_add` and returns
/// `RecoveryError::ReplayDivergence` with an overflow description.
#[test]
fn validate_contiguous_sequences_rejects_u64_max_overflow() {
    let run = RunId::new(20);
    let events: Vec<JournalEvent> = vec![
        run_accepted(run, u64::MAX - 1, [0xBB; DIGEST_BYTES]),
        run_accepted(run, u64::MAX, [0xCC; DIGEST_BYTES]),
        // This would be seq = u64::MAX + 1, which overflows
        step_started(run, 0, 0, 1),
    ];

    let result = replay_with_tracker(events);

    assert!(
        result.is_err(),
        "overflow sequence must be rejected, but got Ok"
    );
    let err = result.unwrap_err();
    match &err {
        RecoveryError::ReplayDivergence { detail, .. } => {
            assert!(
                detail.contains("overflow"),
                "expected ReplayDivergence with overflow detail, got detail = {detail:?}"
            );
        }
        other => panic!("expected ReplayDivergence, got {:?}", other),
    }
}

/// Two events sharing the same sequence number must be rejected as a
/// `RecoveryError::ReplayDivergence` with a sequence violation detail.
/// The duplicate detection happens in `validate_contiguous_sequences`
/// before any per-event idempotency checks.
#[test]
fn validate_contiguous_sequences_rejects_duplicate_sequence() {
    let run = RunId::new(30);
    let events: Vec<JournalEvent> = vec![
        run_accepted(run, 0, [0xDD; DIGEST_BYTES]),
        run_accepted(run, 1, [0xEE; DIGEST_BYTES]),
        run_accepted(run, 1, [0xFF; DIGEST_BYTES]), // duplicate seq=1
    ];

    let result = replay_with_tracker(events);

    assert!(
        result.is_err(),
        "duplicate sequence must be rejected, but got Ok"
    );
    let err = result.unwrap_err();
    match &err {
        RecoveryError::ReplayDivergence { detail, .. } => {
            assert!(
                detail.contains("sequence violation"),
                "expected ReplayDivergence with sequence violation detail, got detail = {detail:?}"
            );
        }
        other => panic!("expected ReplayDivergence, got {:?}", other),
    }
}

// ===========================================================================
// GAP 2: accept_equal_duplicate — mismatched payload (corruption scenario)
// ===========================================================================

/// When a duplicate event has the *same* sequence but a *different* payload
/// (e.g. corrupted workflow digest), `accept_equal_duplicate` returns
/// `JournalError::DuplicateEvent`. This is the actual corruption case:
/// the event was appended once, then the same (run, seq) is retried with
/// different content.
#[test]
fn duplicate_event_with_mismatched_payload_returns_duplicate_error() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let workflow_a = [0x01; DIGEST_BYTES];
    let workflow_b = [0x02; DIGEST_BYTES];

    // First append: succeeds
    let first = run_accepted(run, 0, workflow_a);
    let result1 = journal.append_journaled(&first);
    assert!(
        result1.is_ok(),
        "first append of RunAccepted must succeed, got {:?}",
        result1
    );

    // Second append: same seq, different workflow digest
    let second = run_accepted(run, 0, workflow_b);
    let result2 = journal.append_journaled(&second);

    // Must fail with DuplicateEvent — not a success and not a different error
    assert!(
        result2.is_err(),
        "second append with mismatched payload must fail, got {:?}",
        result2
    );
    assert!(
        matches!(result2, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == run && s == EventSeq::new(0)),
        "expected DuplicateEvent for mismatched duplicate, got {:?}",
        result2
    );

    // Verify: the original (matching) event is still readable
    let events = journal.events_for_run(run).expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "exactly one event must be stored");
    match &events[0] {;
        JournalEvent::RunAccepted { workflow, .. } => {
            assert_eq!(
                *workflow,
                WorkflowDigest::from_bytes(workflow_a),
                "stored event must have original payload, not corrupted one"
            );
        }
        other => panic!("expected RunAccepted, got {:?}", other),
    }
}

// ===========================================================================
// GAP 3: recover_runtime_frame_seed — header-only run (no events)
// ===========================================================================

/// A run that has a `RunHeader` record but zero journal events must
/// return `RecoveryError::NoRecoveryData` when `recover_runtime_frame_seed`
/// is called. The function checks `events.is_empty()` before attempting
/// any summary or frame seed construction.
#[test]
fn header_only_run_returns_no_recovery_data() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let workflow_id = vb_core::WorkflowId::new(42);
    let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

    // Write only a header — no events appended
    let header = RunHeaderRecord {
        run,
        workflow_id,
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 1_715_555_000_000,
    };
    journal
        .put_run_header(&header)
        .expect("put_run_header must succeed");

    // Verify: events_for_run returns empty for this run
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert!(
        events.is_empty(),
        "header-only run must have zero journal events"
    );

    // Now attempt recovery — must fail with NoRecoveryData
    let result = recover_runtime_frame_seed(&journal, run);

    assert!(
        result.is_err(),
        "header-only run recovery must fail, but got Ok"
    );
    assert!(
        matches!(result, Err(RecoveryError::NoRecoveryData { run: r }) if r == run),
        "expected NoRecoveryData for header-only run, got {:?}",
        result
    );
}
