//! BDD recovery tests — B-001 through B-020 coverage.
//!
//! Each test name follows: `fn [subject]_[outcome]_when_[condition]()`
//! Given-When-Then structure in doc comments.

#![forbid(unsafe_code)]

use chrono::Utc;
use tempfile::TempDir;
use vb_core::{
    ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, WorkflowDigest,
};
use vb_storage::recovery::{
    ActionReplayTracker, DigestVerificationRequest, MissingRunStateComponent,
    MissingRunStateComponents, RecoveredStepEntry, RecoveredStepState, RecoveryCannotResumeState,
    RecoveryError, RecoveryFrameSeed, RecoveryHydration, RecoveryRuntimeSummary,
    RecoveryTerminalState, RunSnapshot, UnsupportedRecoveryState, check_action_abi_digests,
    check_compiled_ir_digest, check_policy_digests, check_workflow_source_digest,
    hydrate_run_frame, hydrate_run_frame_from_events, recover_full_journal,
    recover_runtime_frame_seed, recover_runtime_summary, verify_digests,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent};

fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn open_journal(dir: &TempDir) -> FjallJournal {
    FjallJournal::open(dir.path(), Some(FjallConfig::default()))
        .expect("journal open should succeed")
}

fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) {
    for event in events {
        match journal.append_strict(event) {
            Ok(()) | Err(vb_storage::JournalError::DuplicateEvent { .. }) => {}
            Err(error) => panic!("strict append should succeed: {error:?}"),
        }
    }
}

fn assert_unsupported_frame_seed<T: core::fmt::Debug>(
    result: Result<T, RecoveryError>,
    expected_run: RunId,
    expected_reason: &str,
) {
    let Err(RecoveryError::UnsupportedFrameSeed { run, reason }) = result else {
        panic!("expected UnsupportedFrameSeed, got: {result:?}");
    };
    assert_eq!(run, expected_run);
    assert_eq!(reason, expected_reason);
}

fn test_admission_event(run: RunId, seq: EventSeq, digest: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        input_digest: vb_storage::RunInputBinding::LegacyUnbound,
        run,
        seq,
        artifact_digest: digest,
        policy_pin: vb_storage::RunPolicyPin::Unverified,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    }
}

include!("recovery_bdd_tests/part_a.rs");
include!("recovery_bdd_tests/part_b.rs");
include!("recovery_bdd_tests/part_c.rs");
include!("recovery_bdd_tests/part_d.rs");
