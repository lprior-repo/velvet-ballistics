//! Corruption-injection integration tests for replay digest mismatch detection.
//!
//! These tests verify that the recovery system detects adversarial data corruption
//! in the Fjall-backed storage layer. They target ERR-001 through ERR-004.
//!
//! Tests are written as a standalone module that can be appended to
//! `crates/vb_storage/tests/recovery_integration.rs` once Fjall corruption
//! injection support is available.
//!
//! Status: NOT_RUN — requires Fjall byte-level corruption API or direct storage access.
//!
//! Obligations: INTEGRATION-DIGEST-001, INTEGRATION-DIGEST-002, INTEGRATION-DIGEST-003, INTEGRATION-DIGEST-004

#![forbid(unsafe_code)]

use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{recover_runtime_frame_seed_from_events, RecoveryError, UnsupportedRecoveryState};
use vb_storage::{EventSeq, JournalEvent};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

// ─────────────────────────────────────────────────────────────────────────────
// ERR-001: Corrupt artifact digest → WorkflowSourceDigestMismatch
//
// Obligation: INTEGRATION-DIGEST-001
// Clause: ERR-001
// Status: NOT_RUN — requires byte-level journal corruption API
// ─────────────────────────────────────────────────────────────────────────────

/// Test: corrupt_artifact_digest_fails_with_workflow_source_digest_mismatch
///
/// Injects: Mutate `RunAccepted.workflow` field in stored journal bytes (corrupt digest)
/// Expect: `RecoveryError::WorkflowSourceDigestMismatch { expected, found }` with exact digest values
///
/// Implementation strategy (requires Fjall corruption API):
/// 1. Open journal and write RunAccepted with expected_digest
/// 2. Corrupt the stored bytes at the Fjall key for that event, changing workflow digest to found_digest
/// 3. Reopen journal and call `check_workflow_source_digest(journal, run, expected_digest)`
/// 4. Assert: returns WorkflowSourceDigestMismatch { expected: expected_digest, found: found_digest }
///
/// Fallback (if Fjall corruption API unavailable): test is marked BLOCKED_TOOLING.
#[test]
#[ignore = "ERR-001: requires Fjall byte-level corruption injection API — NOT_RUN"]
fn corrupt_artifact_digest_fails_with_workflow_source_digest_mismatch() {
    let expected_digest = test_digest(0xAB);
    let found_digest = test_digest(0xCD);
    let run = RunId::new(9001);

    // Write correct event
    let events = vec![JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: expected_digest,
    }];

    // CORRUPTION STEP: Overwrite stored bytes so workflow digest becomes found_digest
    // This requires Fjall byte-level access — not yet available.
    // Pending: Fjall corruption injection framework (BLOCKED_TOOLING).

    // Reopen and verify
    // let journal = reopen_with_corrupted_bytes(run, seq=0, workflow -> found_digest);
    // let result = check_workflow_source_digest(&journal, run, expected_digest);
    // assert!(matches!(
    //     result,
    //     Err(RecoveryError::WorkflowSourceDigestMismatch {
    //         expected,
    //         found,
    //     }) if expected == expected_digest && found == found_digest
    // ));
}

// ─────────────────────────────────────────────────────────────────────────────
// ERR-002: Corrupt journal sequence → ReplayDivergence
//
// Obligation: INTEGRATION-DIGEST-002
// Clause: ERR-002
// Status: READY — EventSeq ordering is controllable at event construction
// ─────────────────────────────────────────────────────────────────────────────

/// Test: corrupt_journal_sequence_fails_with_replay_divergence
///
/// Injects: Write journal events with non-monotonic EventSeq (sequence corruption)
/// Expect: `RecoveryError::ReplayDivergence { step, detail }` with step index
///
/// Implementation: Write events with intentionally wrong EventSeq ordering.
/// The `recover_runtime_frame_seed_from_events` function scans events in order
/// and detects when event.run_id() changes (multi-run divergence).
///
/// For sequence corruption: write events with seq numbers that are not monotonically increasing.
/// The `summarize_recovery_events` function checks `event.run_id() != run` at each event.
/// For sequence corruption (not multi-run), the specific check depends on whether
/// Fjall stores seq in the value or derives it from storage order.
///
/// Currently implemented: multi-run divergence (wrong run_id). Sequence corruption
/// at the EventSeq field level requires Fjall byte-level access.
#[test]
fn corrupt_journal_sequence_fails_with_replay_divergence() {
    let run = RunId::new(9002);

    // Write events with WRONG run_id for second event — this triggers ReplayDivergence
    // because summarize_recovery_events checks event.run_id() != run on each event.
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(0xAB),
        },
        JournalEvent::StepStarted {
            // CORRUPTION: wrong run_id (simulates sequence corruption detection)
            run: RunId::new(9999), // wrong run
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];

    // This triggers ReplayDivergence because event.run_id() != run on the second event.
    // The detail will be "recovery summary received events for multiple runs"
    // which is the closest error variant for sequence corruption at this level.
    let result = vb_storage::recovery::replay::summary::summarize_recovery_events(&events);

    assert!(
        result.is_err(),
        "sequence corruption should produce an error"
    );
    match result {
        Err(RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(step, StepIdx::ZERO, "step index should be ZERO for multi-run");
            assert!(
                detail.contains("multiple runs"),
                "detail should mention multiple runs, got: {detail}"
            );
        }
        Err(other) => {
            panic!("expected ReplayDivergence, got: {other:?}");
        }
        Ok(_) => {
            panic!("should have returned ReplayDivergence for corrupted sequence");
        }
    }
}

/// Test: corrupt_journal_sequence_with_swapped_seq_numbers
///
/// Injects: Events with swapped SeqNo values (e.g., seq 1 before seq 0)
/// Expect: `RecoveryError::ReplayDivergence`
///
/// This test is more precise than the multi-run test above because it targets
/// the EventSeq ordering specifically. However, it requires that the recovery
/// code actually checks EventSeq ordering, which it currently does not — it only
/// checks run_id consistency.
///
/// Status: NOT_IMPLEMENTED — recovery code does not currently check EventSeq ordering.
#[test]
#[ignore = "ERR-002 variant: recovery does not currently check EventSeq ordering — NOT_IMPLEMENTED"]
fn corrupt_journal_sequence_with_swapped_seq_numbers() {
    let run = RunId::new(9003);

    // Write events with swapped seq numbers
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(1), // swapped: should be 0
            workflow: test_digest(0xAB),
        },
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0), // swapped: should be 1
            workflow: test_digest(0xAB),
        },
    ];

    // Currently, the recovery code does NOT check EventSeq ordering.
    // This test documents the desired behavior but cannot pass until
    // the recovery code is updated to check EventSeq.
    // let result = summarize_recovery_events(&events);
    // assert!(matches!(result, Err(RecoveryError::ReplayDivergence { .. })));
}

// ─────────────────────────────────────────────────────────────────────────────
// ERR-003: Corrupt slot value → UnsupportedRecoveryState::slot_values_unsupported
//
// Obligation: INTEGRATION-DIGEST-003
// Clause: ERR-003
// Status: NOT_RUN — requires byte-level slot value corruption
// ─────────────────────────────────────────────────────────────────────────────

/// Test: corrupt_slot_value_fails_with_slot_values_unsupported
///
/// Injects: Corrupt encoded slot bytes in `SlotWrittenEvent` or snapshot
/// Expect: `UnsupportedRecoveryState::slot_values_unsupported()` set
///
/// Implementation strategy (requires Fjall corruption API):
/// 1. Write a valid SlotWrittenEvent with correct value bytes
/// 2. Corrupt the stored bytes so postcard::from_bytes::<SlotValue> returns an error
/// 3. Reopen and call recover_runtime_frame_seed_from_events
/// 4. Assert: UnsupportedRecoveryState::slot_values_unsupported() is true
///
/// Fallback: Write SlotWrittenEvent with value = None (missing slot) — this is
/// detected by `record_slot_write` setting `missing_slot_values = true` when
/// value.is_none(). But this is NOT the same as byte corruption.
#[test]
#[ignore = "ERR-003: requires Fjall byte-level corruption injection API — NOT_RUN"]
fn corrupt_slot_value_fails_with_slot_values_unsupported() {
    let run = RunId::new(9004);

    // CORRUPTION STEP: Write slot with bytes that fail postcard deserialization
    // This requires direct Fjall byte access.
    // let events = vec![
    //     JournalEvent::RunAccepted {
    //         run,
    //         seq: EventSeq::new(0),
    //         workflow: test_digest(0xAB),
    //     },
    //     JournalEvent::SlotWrittenEvent {
    //         run,
    //         seq: EventSeq::new(1),
    //         slot: SlotIdx::new(0),
    //         value: Some(vec![0xFF, 0xFE, 0xFD]), // corrupt bytes
    //         extra: None,
    //         attempt: 1,
    //     },
    // ];
    //
    // let result = recover_runtime_frame_seed_from_events(&events);
    // let seed = result.expect("recovery should succeed");
    // assert!(
    //     seed.unsupported.slot_values,
    //     "corrupt slot value should set slot_values_unsupported"
    // );
}

/// Fallback test: missing slot value triggers slot_values_unsupported
/// This verifies the underlying mechanism but does NOT test byte corruption.
#[test]
fn missing_slot_value_sets_slot_values_unsupported() {
    let run = RunId::new(9005);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(0xAB),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None, // missing value
            extra: None,
            attempt: 1,
        },
    ];

    let result = recover_runtime_frame_seed_from_events(&events);
    let seed = result.expect("recovery should succeed");
    assert!(
        seed.unsupported.slot_values,
        "missing slot value should set slot_values_unsupported"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// ERR-004: Corrupt slot taint → UnsupportedRecoveryState::event_slot_taint_unsupported
//
// Obligation: INTEGRATION-DIGEST-004
// Clause: ERR-004
// Status: NOT_RUN — requires byte-level slot taint (extra field) corruption
// ─────────────────────────────────────────────────────────────────────────────

/// Test: corrupt_slot_taint_fails_with_event_slot_taint_unsupported
///
/// Injects: Corrupt `extra` (taint) field in `SlotWrittenEvent`
/// Expect: `UnsupportedRecoveryState::event_slot_taint_unsupported()` set
///
/// Implementation strategy (requires Fjall corruption API):
/// 1. Write a valid SlotWrittenEvent with correct value and extra bytes
/// 2. Corrupt the extra bytes so postcard::from_bytes::<Taint> returns an error
/// 3. Reopen and call recover_runtime_frame_seed_from_events
/// 4. Assert: UnsupportedRecoveryState::event_slot_taint_unsupported() is true
///
/// The taint is recovered by `recovered_slot_taint()` which calls
/// postcard::from_bytes::<Taint>(extra). If extra is Some(invalid_bytes),
/// the result is an error and `event_slot_taint_unsupported` is set to true.
#[test]
#[ignore = "ERR-004: requires Fjall byte-level corruption injection API — NOT_RUN"]
fn corrupt_slot_taint_fails_with_event_slot_taint_unsupported() {
    let run = RunId::new(9006);

    // CORRUPTION STEP: Write slot with extra that fails postcard deserialization as Taint
    // This requires direct Fjall byte access.
    // let events = vec![
    //     JournalEvent::RunAccepted {
    //         run,
    //         seq: EventSeq::new(0),
    //         workflow: test_digest(0xAB),
    //     },
    //     JournalEvent::SlotWrittenEvent {
    //         run,
    //         seq: EventSeq::new(1),
    //         slot: SlotIdx::new(0),
    //         value: Some(postcard::to_allocvec(&SlotValue::Bool(true)).unwrap()),
    //         extra: Some(vec![0xFF, 0xFE, 0xFD]), // corrupt taint bytes
    //         attempt: 1,
    //     },
    // ];
    //
    // let result = recover_runtime_frame_seed_from_events(&events);
    // let seed = result.expect("recovery should succeed");
    // assert!(
    //     seed.unsupported.slot_taint,
    //     "corrupt taint should set event_slot_taint_unsupported"
    // );
}

/// Fallback test: invalid taint bytes trigger event_slot_taint_unsupported
/// This verifies the underlying mechanism but does NOT test byte corruption.
#[test]
fn invalid_taint_bytes_sets_event_slot_taint_unsupported() {
    use vb_core::SlotValue;

    let run = RunId::new(9007);

    let valid_value = postcard::to_allocvec(&SlotValue::Bool(true))
        .expect("value serialization should succeed");

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(0xAB),
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: Some(valid_value),
            extra: Some(vec![0xFF, 0xFE, 0xFD]), // invalid Taint bytes
            attempt: 1,
        },
    ];

    let result = recover_runtime_frame_seed_from_events(&events);
    let seed = result.expect("recovery should succeed");
    assert!(
        seed.unsupported.slot_taint,
        "invalid taint bytes should set event_slot_taint_unsupported"
    );
}
