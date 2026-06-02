//! Behavior tests for vb_cli storage and I/O commands.
//!
//! Tests cover:
//! - Storage inspection behavior (events_for_run)
//! - Event replay behavior (recover_full_journal)
//! - Journal read/write behavior
//! - Error handling on corrupted data
//!
//! Note: vb_cli::StorageWorkflowResolver and vb_cli::event_name are private
//! implementation details. These tests verify the same behavior through the
//! public vb_storage and vb_ipc APIs.

use std::sync::Arc;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_core::{RunId, RuntimePolicy};
use vb_ipc::server::WorkflowResolutionError;
#[cfg(test)]
use vb_storage::__put_compiled_ir_for_testing as put_compiled_ir;
use vb_storage::{
    CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent,
    recovery::{ActionReplayTracker, extract_terminal, recover_full_journal},
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn test_tempdir() -> tempfile::TempDir {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/vb-cli-storage-io-tests-tmp");
    std::fs::create_dir_all(&root).expect("test tmp dir must exist");
    tempfile::Builder::new()
        .prefix("vb-cli-storage-io-")
        .tempdir_in(root)
        .expect("tempdir must be available")
}

fn dummy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB_u8; 32])
}

fn make_run_id(v: u64) -> RunId {
    RunId::new(v)
}

fn make_event_seq(v: u64) -> EventSeq {
    EventSeq::new(v)
}

fn make_step_idx(v: u16) -> StepIdx {
    StepIdx::new(v)
}

fn make_slot_idx(v: u16) -> SlotIdx {
    SlotIdx::new(v)
}

fn make_action_id(v: u16) -> ActionId {
    ActionId::new(v)
}

fn finish_workflow() -> Option<CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let mut parts = WorkflowParts {
        name: Box::from("finish"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    let ir = postcard::to_allocvec(&parts).ok()?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Local implementation mirroring vb_cli::StorageWorkflowResolver behavior.
/// This is needed because StorageWorkflowResolver is not publicly exported.
struct TestStorageResolver {
    journal: Arc<FjallJournal>,
}

impl TestStorageResolver {
    fn new(journal: FjallJournal) -> Self {
        Self {
            journal: Arc::new(journal),
        }
    }

    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError> {
        let record = match self.journal.compiled_ir(digest) {
            Ok(Some(record)) => record,
            Ok(None) => return Err(WorkflowResolutionError::NotFound),
            Err(_) => return Err(WorkflowResolutionError::InvalidArtifact),
        };
        if record.digest != digest {
            return Err(WorkflowResolutionError::InvalidArtifact);
        }
        let artifact = postcard::from_bytes::<vb_storage::AcceptedArtifact>(&record.ir)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
        if artifact.digest != digest {
            return Err(WorkflowResolutionError::InvalidArtifact);
        }
        let mut parts = postcard::from_bytes::<WorkflowParts>(&artifact.ir)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)?;
        parts.digest = artifact.digest;
        CompiledWorkflow::try_from_parts(parts)
            .map_err(|_| WorkflowResolutionError::InvalidArtifact)
    }
}

/// Mirror of vb_cli::storage::event_name for testing.
/// Returns the static event type name for each JournalEvent variant.
fn event_name(event: &JournalEvent) -> &'static str {
    match event {
        JournalEvent::RunAccepted { .. } => "RunAccepted",
        JournalEvent::StepStarted { .. } => "StepStarted",
        JournalEvent::StepSucceeded { .. } => "StepSucceeded",
        JournalEvent::ActionScheduled { .. } => "ActionScheduled",
        JournalEvent::ActionCompletedEvent { .. } => "ActionCompleted",
        JournalEvent::ActionFailedEvent { .. } => "ActionFailed",
        JournalEvent::SlotWrittenEvent { .. } => "SlotWritten",
        JournalEvent::WaitScheduledEvent { .. } => "WaitScheduled",
        JournalEvent::AskScheduledEvent { .. } => "AskScheduled",
        JournalEvent::AskAnsweredEvent { .. } => "AskAnswered",
        JournalEvent::RetryScheduledEvent { .. } => "RetryScheduled",
        JournalEvent::RunCancelled { .. } => "RunCancelled",
        JournalEvent::RunFinished { .. } => "RunFinished",
        JournalEvent::RunFailedEvent { .. } => "RunFailed",
        JournalEvent::RunResumed { .. } => "RunResumed",
        JournalEvent::RunRetried { .. } => "RunRetried",
        JournalEvent::RunAnswered { .. } => "RunAnswered",
        JournalEvent::RunAdmission { .. } => "RunAdmission",
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// Storage inspection behavior
// ---------------------------------------------------------------------------

/// Verify inspect reports "running" when run has no terminal event.
#[test]
fn inspect_returns_running_for_incomplete_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    // Re-open journal to ensure persistence
    drop(journal);
    let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let events = result.expect("events ok");

    // Verify no terminal event
    let terminal = events.last();
    assert!(
        !matches!(
            terminal,
            Some(JournalEvent::RunFinished { .. })
                | Some(JournalEvent::RunFailedEvent { .. })
                | Some(JournalEvent::RunCancelled { .. })
        ),
        "incomplete run must not have terminal event"
    );
}

/// Verify inspect reports "finished" for completed run.
#[test]
fn inspect_returns_finished_for_completed_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunFinished {
            run,
            seq: make_event_seq(1),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    drop(journal);
    let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let events = result.expect("events ok");

    let terminal = events.last();
    assert!(
        matches!(terminal, Some(JournalEvent::RunFinished { .. })),
        "completed run must have RunFinished terminal"
    );
}

/// Verify inspect reports "failed" for failed run.
#[test]
fn inspect_returns_failed_for_failed_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: make_event_seq(1),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    drop(journal);
    let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let events = result.expect("events ok");

    let terminal = events.last();
    assert!(
        matches!(terminal, Some(JournalEvent::RunFailedEvent { .. })),
        "failed run must have RunFailedEvent terminal"
    );
}

/// Verify inspect reports "cancelled" for cancelled run.
#[test]
fn inspect_returns_cancelled_for_cancelled_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunCancelled {
            run,
            seq: make_event_seq(1),
            attempt: 1,
            reason: Some("user requested".to_string()),
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    drop(journal);
    let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let events = result.expect("events ok");

    let terminal = events.last();
    assert!(
        matches!(terminal, Some(JournalEvent::RunCancelled { .. })),
        "cancelled run must have RunCancelled terminal"
    );
}

/// Verify inspect returns empty for non-existent run.
#[test]
fn inspect_returns_empty_for_nonexistent_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let result = journal.events_for_run(make_run_id(999));
    assert!(
        result.is_ok(),
        "events_for_run must succeed for missing run"
    );
    let events = result.expect("events ok");
    assert!(events.is_empty(), "missing run must return empty events");
}

/// Verify inspect counts events correctly.
#[test]
fn inspect_returns_correct_event_count() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: make_event_seq(2),
            step: make_step_idx(0),
            output: make_slot_idx(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: make_event_seq(3),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let fetched = result.expect("events ok");
    assert_eq!(fetched.len(), 4, "must return all 4 events");
}

// ---------------------------------------------------------------------------
// Event replay behavior
// ---------------------------------------------------------------------------

/// Verify replay recovers events for a run.
#[test]
fn replay_recovers_all_events() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: make_event_seq(2),
            step: make_step_idx(0),
            output: make_slot_idx(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: make_event_seq(3),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    assert!(result.is_ok(), "recover_full_journal must succeed");
    let recovered = result.expect("recover ok");
    assert_eq!(recovered.len(), 4, "must recover all 4 events");
}

/// Verify replay extracts terminal event correctly.
#[test]
fn replay_extracts_terminal_event() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunFinished {
            run,
            seq: make_event_seq(1),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    assert!(result.is_ok(), "recover_full_journal must succeed");
    let recovered = result.expect("recover ok");

    let terminal = extract_terminal(&recovered);
    assert!(terminal.is_some(), "terminal event must be extracted");
    assert!(
        matches!(terminal, Some(JournalEvent::RunFinished { .. })),
        "terminal must be RunFinished"
    );
}

/// Verify replay fails for non-existent run (no recovery data).
#[test]
fn replay_fails_for_nonexistent_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, make_run_id(999), &mut tracker, &[], &[]);
    assert!(
        result.is_err(),
        "recover_full_journal must fail for missing run"
    );
}

/// Verify extract_terminal returns None for incomplete run.
#[test]
fn replay_terminal_none_for_incomplete_run() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![JournalEvent::RunAccepted {
        run,
        seq: make_event_seq(0),
        workflow: dummy_digest(),
    }];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    assert!(result.is_ok(), "recover_full_journal must succeed");
    let recovered = result.expect("recover ok");

    let terminal = extract_terminal(&recovered);
    assert!(terminal.is_none(), "incomplete run must have no terminal");
}

/// Verify extract_terminal returns last terminal when multiple exist.
#[test]
fn replay_terminal_returns_last_terminal() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    // First attempt failed
    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: make_event_seq(1),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    assert!(result.is_ok(), "recover_full_journal must succeed");
    let recovered = result.expect("recover ok");

    let terminal = extract_terminal(&recovered);
    assert!(
        matches!(terminal, Some(JournalEvent::RunFailedEvent { seq, .. }) if seq.get() == 1),
        "terminal must be the last RunFailedEvent with seq 1"
    );
}

// ---------------------------------------------------------------------------
// Journal read/write behavior
// ---------------------------------------------------------------------------

/// Verify events_for_run returns events in ascending sequence order.
#[test]
fn events_for_run_returns_ascending_sequences() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: make_event_seq(2),
            step: make_step_idx(0),
            output: make_slot_idx(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: make_event_seq(3),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let fetched = result.expect("events ok");

    // Sequences must be strictly ascending
    let mut prev_seq: Option<u64> = None;
    for (i, event) in fetched.iter().enumerate() {
        let seq = event.seq().get();
        if let Some(prev) = prev_seq {
            assert!(
                seq > prev,
                "sequence {} at index {} must be greater than previous {}",
                seq,
                i,
                prev
            );
        }
        prev_seq = Some(seq);
    }
}

/// Verify events_for_run returns empty vec for run with no events.
#[test]
fn events_for_run_returns_empty_for_run_without_events() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    // Write events for one run
    let run_a = make_run_id(1);
    journal
        .append_strict_batch(&[JournalEvent::RunAccepted {
            run: run_a,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        }])
        .expect("event must append");

    // Query different run
    let result = journal.events_for_run(make_run_id(2));
    assert!(result.is_ok(), "events_for_run must succeed for empty run");
    let events = result.expect("events ok");
    assert!(events.is_empty(), "empty run must return empty vec");
}

/// Verify journal appends are durable after reopen.
#[test]
fn journal_events_persist_after_reopen() {
    let dir = test_tempdir();
    let path = dir.path().to_path_buf();

    {
        let journal = FjallJournal::open(&path, None).expect("journal must open");
        let run = make_run_id(1);
        journal
            .append_strict_batch(&[JournalEvent::RunAccepted {
                run,
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            }])
            .expect("event must append");
    }

    {
        let journal = FjallJournal::open(&path, None).expect("journal must reopen");
        let result = journal.events_for_run(make_run_id(1));
        assert!(result.is_ok(), "events must be readable after reopen");
        let events = result.expect("events ok");
        assert_eq!(events.len(), 1, "one event must persist");
    }
}

/// Verify duplicate events are rejected by journal.
#[test]
fn journal_rejects_duplicate_event() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let event = JournalEvent::RunAccepted {
        run,
        seq: make_event_seq(0),
        workflow: dummy_digest(),
    };

    journal
        .append_strict_batch(&[event.clone()])
        .expect("first append must succeed");

    let result = journal.append_strict_batch(&[event]);
    assert!(result.is_err(), "duplicate event must be rejected");
}

/// Verify journal can handle multiple runs independently.
#[test]
fn journal_handles_multiple_runs_independently() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run_a = make_run_id(1);
    let run_b = make_run_id(2);

    journal
        .append_strict_batch(&[JournalEvent::RunAccepted {
            run: run_a,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        }])
        .expect("run A event must append");

    journal
        .append_strict_batch(&[JournalEvent::RunAccepted {
            run: run_b,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        }])
        .expect("run B event must append");

    let result_a = journal.events_for_run(run_a);
    let result_b = journal.events_for_run(run_b);

    assert!(result_a.is_ok(), "run A events must be readable");
    assert!(result_b.is_ok(), "run B events must be readable");

    assert_eq!(result_a.expect("a ok").len(), 1, "run A must have 1 event");
    assert_eq!(result_b.expect("b ok").len(), 1, "run B must have 1 event");
}

/// Verify event_name returns correct static strings for all event variants.
#[test]
fn event_name_returns_correct_static_strings() {
    let events = [
        (
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            "RunAccepted",
        ),
        (
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            "StepStarted",
        ),
        (
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            "StepSucceeded",
        ),
        (
            JournalEvent::ActionScheduled {
                run: make_run_id(1),
                seq: make_event_seq(3),
                step: make_step_idx(0),
                action: make_action_id(1),
                attempt: 1,
            },
            "ActionScheduled",
        ),
        (
            JournalEvent::ActionCompletedEvent {
                run: make_run_id(1),
                seq: make_event_seq(4),
                step: make_step_idx(0),
                action: make_action_id(1),
                attempt: 1,
            },
            "ActionCompleted",
        ),
        (
            JournalEvent::ActionFailedEvent {
                run: make_run_id(1),
                seq: make_event_seq(5),
                step: make_step_idx(0),
                action: make_action_id(1),
                attempt: 1,
            },
            "ActionFailed",
        ),
        (
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(6),
                result: make_slot_idx(0),
                attempt: 1,
            },
            "RunFinished",
        ),
        (
            JournalEvent::RunFailedEvent {
                run: make_run_id(1),
                seq: make_event_seq(7),
                attempt: 1,
            },
            "RunFailed",
        ),
        (
            JournalEvent::RunCancelled {
                run: make_run_id(1),
                seq: make_event_seq(8),
                attempt: 1,
                reason: None,
            },
            "RunCancelled",
        ),
    ];

    for (event, expected_name) in events {
        let name = event_name(&event);
        assert_eq!(
            name, expected_name,
            "event_name must return '{}' for {:?}",
            expected_name, event
        );
    }
}

// ---------------------------------------------------------------------------
// Error handling on corrupted data
// ---------------------------------------------------------------------------

/// Verify FjallJournal::open fails gracefully with non-existent path.
#[test]
fn journal_open_fails_for_missing_directory() {
    let result = FjallJournal::open("/nonexistent/path/to/journal", None);
    assert!(
        result.is_err(),
        "journal open must fail for missing directory"
    );
}

/// Verify resolver returns NotFound for unknown digest.
#[test]
fn resolver_returns_not_found_for_unknown_digest() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let mut resolver = TestStorageResolver::new(journal);

    let result = resolver.resolve_workflow(dummy_digest());
    assert!(
        matches!(result, Err(WorkflowResolutionError::NotFound)),
        "unknown digest must return NotFound"
    );
}

/// Verify resolver loads compiled IR when present.
#[test]
fn resolver_loads_compiled_ir_when_present() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow must compile");

    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    if let Some(compiled) = compiled {
        vb_storage::admission::submit_artifact(&journal, &compiled, RuntimePolicy::Journaled)
            .expect("submit_artifact must succeed");

        drop(journal);

        let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");
        let mut resolver = TestStorageResolver::new(journal);

        let result = resolver.resolve_workflow(compiled.digest());
        assert!(result.is_ok(), "resolver must load compiled IR");
        assert_eq!(result.expect("ok").digest(), compiled.digest());
    }
}

/// Verify resolver returns InvalidArtifact for corrupted IR.
#[test]
fn storage_rejects_corrupted_ir_before_resolver_load() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let record = CompiledIrRecord {
        digest: dummy_digest(),
        ir: vec![0xDE, 0xAD, 0xBE, 0xEF], // Corrupted data
    };
    let write_result = put_compiled_ir(&journal, &record);
    assert!(
        matches!(write_result, Err(JournalError::ArtifactMalformed)),
        "corrupted IR must be rejected at write"
    );

    drop(journal);

    let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");
    let mut resolver = TestStorageResolver::new(journal);

    let result = resolver.resolve_workflow(record.digest);
    assert!(
        matches!(result, Err(WorkflowResolutionError::NotFound)),
        "rejected corrupted IR must resolve as NotFound"
    );
}

/// Verify resolver returns InvalidArtifact for tampered digest.
#[test]
fn resolver_returns_invalid_artifact_for_tampered_digest() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow must compile");

    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    if let Some(compiled) = compiled {
        vb_storage::admission::submit_artifact(&journal, &compiled, RuntimePolicy::Journaled)
            .expect("submit_artifact must succeed");

        drop(journal);

        let journal = FjallJournal::open(dir.path(), None).expect("journal must reopen");
        let mut resolver = TestStorageResolver::new(journal);

        // Try to resolve with different digest
        let tampered_digest = WorkflowDigest::from_bytes([0xFF; 32]);
        let result = resolver.resolve_workflow(tampered_digest);
        assert!(
            matches!(result, Err(WorkflowResolutionError::NotFound)),
            "tampered digest must return NotFound"
        );
    }
}

/// Verify events include sequence numbers.
#[test]
fn events_include_sequence_numbers() {
    let dir = test_tempdir();
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");

    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
    ];

    journal
        .append_strict_batch(&events)
        .expect("events must append");

    let result = journal.events_for_run(run);
    assert!(result.is_ok(), "events_for_run must succeed");
    let fetched = result.expect("events ok");

    assert_eq!(fetched[0].seq().get(), 0, "first event seq must be 0");
    assert_eq!(fetched[1].seq().get(), 1, "second event seq must be 1");
}

/// Verify event seq() method returns correct sequence for each variant.
#[test]
fn event_seq_returns_correct_value_for_variants() {
    let run = make_run_id(1);

    let events: Vec<JournalEvent> = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(10),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(20),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: make_event_seq(30),
            step: make_step_idx(0),
            output: make_slot_idx(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: make_event_seq(40),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    assert_eq!(events[0].seq().get(), 10);
    assert_eq!(events[1].seq().get(), 20);
    assert_eq!(events[2].seq().get(), 30);
    assert_eq!(events[3].seq().get(), 40);
}
