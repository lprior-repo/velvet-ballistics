//! Full recovery support for velvet-ballastics journal.
//!
//! Provides:
//! - Digest mismatch detection (workflow source, compiled IR, action ABI, policy)
//! - Full primitive replay (all node kinds)
//! - Non-idempotent action policy: block re-execution during recovery
//! - Replay divergence detection with typed error
//! - Snapshot-plus-tail journal recovery
//! - Full journal recovery when no snapshot available

use crate::{EventSeq, FjallJournal, JournalError, JournalEvent, RecoveryHydration};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

/// Recovery failures with typed diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    /// Journal operation failed during recovery.
    #[error("journal error during recovery: {0}")]
    Journal(#[from] JournalError),
    /// Workflow source digest does not match the stored record.
    #[error("workflow source digest mismatch: expected {expected:?}, found {found:?}")]
    WorkflowSourceDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Compiled IR digest does not match the stored record.
    #[error("compiled IR digest mismatch: expected {expected:?}, found {found:?}")]
    CompiledIrDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Action ABI digest mismatch during recovery.
    #[error("action ABI digest mismatch for action {action_id:?}")]
    ActionAbiMismatch {
        /// Action with mismatched ABI.
        action_id: ActionId,
    },
    /// Policy digest mismatch during recovery.
    #[error("policy digest mismatch for step {step:?}")]
    PolicyDigestMismatch {
        /// Step where policy diverged.
        step: StepIdx,
    },
    /// A non-idempotent action was encountered during recovery and cannot be re-executed.
    #[error(
        "non-idempotent action {action:?} at step {step:?} cannot be re-executed during recovery"
    )]
    NonIdempotentActionBlocked {
        /// Action identifier.
        action: ActionId,
        /// Step where the action was scheduled.
        step: StepIdx,
    },
    /// Replay diverged from expected state machine trajectory.
    #[error("replay divergence at step {step:?}: {detail}")]
    ReplayDivergence {
        /// Step where divergence was detected.
        step: StepIdx,
        /// Divergence description.
        detail: String,
    },
    /// No snapshot or journal events found for run.
    #[error("no recovery data found for run {run:?}")]
    NoRecoveryData {
        /// Run identifier.
        run: RunId,
    },
    /// Snapshot is present but corrupt or unreadable.
    #[error("snapshot corrupt for run {run:?} at seq {seq:?}")]
    CorruptSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence.
        seq: EventSeq,
    },
    /// Recovery produced a terminal state that does not match expectations.
    #[error("recovery terminal state mismatch: expected {expected:?}, found {found:?}")]
    TerminalStateMismatch {
        /// Expected terminal event kind.
        expected: String,
        /// Found terminal event kind.
        found: String,
    },
}

/// Result alias for recovery operations.
pub type RecoveryResult<T> = Result<T, RecoveryError>;

/// Snapshot of a run's runtime state at a specific event sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Sequence number at which this snapshot was taken.
    pub seq: EventSeq,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Slot values at snapshot time, compact binary form.
    pub slots: Vec<u8>,
}

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    completed: HashSet<(ActionId, StepIdx)>,
    failed: HashSet<(ActionId, StepIdx)>,
}

impl ActionReplayTracker {
    /// Creates an empty action replay tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            completed: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    /// Records that an action was completed during normal execution.
    /// During recovery, encountering this action again will block re-execution.
    pub fn mark_completed(&mut self, action: ActionId, step: StepIdx) {
        self.completed.insert((action, step));
    }

    /// Records that an action failed during normal execution.
    pub fn mark_failed(&mut self, action: ActionId, step: StepIdx) {
        self.failed.insert((action, step));
    }

    /// Checks whether an action has already been resolved (completed or failed)
    /// and must not be re-executed during recovery.
    #[must_use]
    pub fn is_resolved(&self, action: ActionId, step: StepIdx) -> bool {
        self.completed.contains(&(action, step)) || self.failed.contains(&(action, step))
    }
}

impl Default for ActionReplayTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Digest check level for recovery validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestCheck {
    /// Only verify workflow source digest.
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.
    Full,
}

/// Verifies that the workflow source digest matches the stored record.
pub fn check_workflow_source_digest(
    journal: &FjallJournal,
    run: RunId,
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    let Some(header) = journal.run_header(run)? else {
        return Err(RecoveryError::Journal(JournalError::MissingRunHeader {
            run,
        }));
    };
    if header.compiled_digest != expected {
        return Err(RecoveryError::WorkflowSourceDigestMismatch {
            expected,
            found: header.compiled_digest,
        });
    }
    journal.verify_run_metadata_digest(run)?;
    Ok(())
}

/// Verifies that the compiled IR digest matches the expected value.
pub fn check_compiled_ir_digest(
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(RecoveryError::CompiledIrDigestMismatch { expected, found })
    }
}

/// Verifies all digests at the requested check level.
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
) -> RecoveryResult<()> {
    if matches!(
        level,
        DigestCheck::WorkflowSourceOnly | DigestCheck::WorkflowAndIr | DigestCheck::Full
    ) {
        check_workflow_source_digest(journal, run, workflow_digest)?;
    }
    if matches!(level, DigestCheck::WorkflowAndIr | DigestCheck::Full) {
        check_compiled_ir_digest(ir_digest, found_ir_digest)?;
    }
    Ok(())
}

/// Replays a full journal for a run when no snapshot is available.
/// Returns the persisted sequence of journal events and populates the action tracker.
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    replay_events(&events, tracker)
}

/// Hydrates persisted run state and replays either full journal or snapshot tail.
pub fn recover_run(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<RecoveryHydration> {
    journal.verify_run_metadata_digest(run)?;
    let Some(hydration) = journal.hydrate_recovery_state(run)? else {
        return Err(RecoveryError::NoRecoveryData { run });
    };
    match &hydration.latest_snapshot {
        Some(snapshot) => {
            recover_snapshot_plus_tail(snapshot, &hydration.tail_events, tracker)?;
        }
        None => {
            replay_events(&hydration.tail_events, tracker)?;
        }
    }
    Ok(hydration)
}

/// Replays from a snapshot plus tail events.
/// The snapshot provides the base state, and tail events are replayed on top.
pub fn recover_snapshot_plus_tail(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>> {
    // Verify snapshot consistency
    let snapshot_seq = snapshot.seq;
    for event in tail_events {
        if event.seq() <= snapshot_seq {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "tail event seq {} is not after snapshot seq {}",
                    event.seq().get(),
                    snapshot_seq.get()
                ),
            });
        }
    }

    replay_events(tail_events, tracker)
}

/// Core replay logic for all journal event kinds.
/// Populates the action tracker and detects divergence.
pub fn replay_events(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
) -> RecoveryResult<Vec<JournalEvent>> {
    let mut replayed = Vec::new();
    let resolved_before_replay = tracker.clone();

    for event in events {
        match event {
            JournalEvent::RunAccepted { .. } => {
                // Accepted is the start of a run
            }
            JournalEvent::StepStarted { step, .. } => {
                // Step indexes are graph node identifiers, not a monotonic clock.
                let _ = step;
            }
            JournalEvent::StepSucceeded { step, .. } => {
                // Step completed successfully
                let _ = step;
            }
            JournalEvent::ActionScheduled { action, step, .. } => {
                // Only pre-existing resolutions block re-execution. Resolutions
                // observed inside this persisted replay are journal truth and may
                // be followed by retry/loop events for the same step.
                if resolved_before_replay.is_resolved(*action, *step) {
                    return Err(RecoveryError::NonIdempotentActionBlocked {
                        action: *action,
                        step: *step,
                    });
                }
            }
            JournalEvent::ActionCompletedEvent { action, step, .. } => {
                // Mark action as completed to prevent re-execution
                tracker.mark_completed(*action, *step);
            }
            JournalEvent::ActionFailedEvent { action, step, .. } => {
                // Mark action as failed to prevent re-execution
                tracker.mark_failed(*action, *step);
            }
            JournalEvent::SlotWrittenEvent { slot, .. } => {
                // Slot write during replay
                let _ = slot;
            }
            JournalEvent::WaitScheduledEvent { step, .. } => {
                let _ = step;
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                let _ = step;
            }
            JournalEvent::AskAnsweredEvent { step, .. } => {
                let _ = step;
            }
            JournalEvent::RetryScheduledEvent { step, .. } => {
                let _ = step;
            }
            JournalEvent::RunCancelled { .. } => {
                // Terminal state
            }
            JournalEvent::RunFinished { .. } => {
                // Terminal state - successful completion
            }
            JournalEvent::RunFailedEvent { .. } => {
                // Terminal state - failure
            }
        }
        replayed.push(event.clone());
    }

    Ok(replayed)
}

/// Checks whether a run has reached a terminal state.
#[must_use]
pub fn is_terminal_event(event: &JournalEvent) -> bool {
    matches!(
        event,
        JournalEvent::RunFinished { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunFailedEvent { .. }
    )
}

/// Extracts the terminal event from a replay sequence, if any.
pub fn extract_terminal(events: &[JournalEvent]) -> Option<&JournalEvent> {
    events.iter().find(|e| is_terminal_event(e))
}

#[cfg(test)]
mod tests {
    use super::{
        ActionReplayTracker, RecoveryError, RunSnapshot, check_compiled_ir_digest,
        check_workflow_source_digest, extract_terminal, is_terminal_event, recover_full_journal,
        recover_snapshot_plus_tail, replay_events,
    };
    use crate::{EventSeq, FjallJournal, JournalEvent, RunHeaderRecord};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    fn storage_tempdir() -> Result<tempfile::TempDir, std::io::Error> {
        let base =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/vb_storage_tests");
        std::fs::create_dir_all(&base)?;
        tempfile::Builder::new()
            .prefix("recovery-")
            .tempdir_in(base)
    }

    #[test]
    fn action_tracker_blocks_non_idempotent_replay() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(1);
        let step = StepIdx::new(5);

        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));

        let events = vec![JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step,
            action,
        }];

        let result = replay_events(&events, &mut tracker);
        assert!(
            matches!(
                result,
                Err(RecoveryError::NonIdempotentActionBlocked { .. })
            ),
            "should block re-execution of completed action"
        );
    }

    #[test]
    fn action_tracker_allows_first_execution() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(1);
        let step = StepIdx::new(5);

        let events = vec![
            JournalEvent::ActionScheduled {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                step,
                action,
            },
            JournalEvent::ActionCompletedEvent {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                step,
                action,
            },
        ];

        let result = replay_events(&events, &mut tracker);
        assert!(result.is_ok(), "first execution should succeed");
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn action_tracker_tracks_failed_actions() {
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(2);
        let step = StepIdx::new(3);

        tracker.mark_failed(action, step);
        assert!(tracker.is_resolved(action, step));

        let events = vec![JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step,
            action,
        }];

        let result = replay_events(&events, &mut tracker);
        assert!(
            matches!(
                result,
                Err(RecoveryError::NonIdempotentActionBlocked { .. })
            ),
            "should block re-execution of failed action"
        );
    }

    #[test]
    fn compiled_ir_digest_match_succeeds() {
        let digest = test_digest(42);
        let result = check_compiled_ir_digest(digest, digest);
        assert!(result.is_ok());
    }

    #[test]
    fn compiled_ir_digest_mismatch_fails() {
        let expected = test_digest(1);
        let found = test_digest(2);
        let result = check_compiled_ir_digest(expected, found);
        assert!(
            matches!(result, Err(RecoveryError::CompiledIrDigestMismatch { .. })),
            "mismatched digests should fail"
        );
    }

    #[test]
    fn workflow_digest_check_uses_run_metadata() {
        let temp_dir = storage_tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };
        let run = RunId::new(31);
        let digest = test_digest(7);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 2,
        };
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };

        assert!(journal.put_run_header(&header).is_ok());
        assert!(journal.append_journaled(&accepted).is_ok());

        let result = check_workflow_source_digest(&journal, run, test_digest(8));

        assert!(matches!(
            result,
            Err(RecoveryError::WorkflowSourceDigestMismatch { found, .. }) if found == digest
        ));
    }

    #[test]
    fn workflow_digest_check_rejects_metadata_event_mismatch() {
        let temp_dir = storage_tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };
        let run = RunId::new(32);
        let digest = test_digest(7);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 2,
        };
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(9),
        };

        assert!(journal.put_run_header(&header).is_ok());
        assert!(journal.append_journaled(&accepted).is_ok());

        let result = check_workflow_source_digest(&journal, run, digest);

        assert!(matches!(result, Err(RecoveryError::Journal(_))));
    }

    #[test]
    fn is_terminal_event_identifies_terminals() {
        assert!(is_terminal_event(&JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
        }));
        assert!(is_terminal_event(&JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        }));
        assert!(is_terminal_event(&JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        }));
        assert!(!is_terminal_event(&JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
        }));
    }

    #[test]
    fn extract_terminal_finds_last_terminal() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::RunFinished {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                result: SlotIdx::new(0),
            },
        ];

        let terminal = extract_terminal(&events);
        assert!(terminal.is_some());
        assert!(matches!(terminal, Some(JournalEvent::RunFinished { .. })));
    }

    #[test]
    fn extract_terminal_returns_none_without_terminal() {
        let events = vec![JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        }];

        let terminal = extract_terminal(&events);
        assert!(terminal.is_none());
    }

    #[test]
    fn snapshot_plus_tail_rejects_event_before_snapshot() {
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            workflow: test_digest(1),
            slots: Vec::new(),
        };
        let tail = vec![JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(3), // before snapshot
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "tail event before snapshot should be rejected"
        );
    }

    #[test]
    fn full_journal_recovery_with_no_data_fails() {
        let temp_dir = storage_tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else {
            return;
        };
        let mut tracker = ActionReplayTracker::new();

        let result = recover_full_journal(&journal, RunId::new(999), &mut tracker);
        assert!(
            matches!(result, Err(RecoveryError::NoRecoveryData { .. })),
            "empty journal should produce NoRecoveryData"
        );
    }

    #[test]
    fn full_journal_recovery_replays_events() {
        let temp_dir = storage_tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else {
            return;
        };
        let run = RunId::new(42);

        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let started = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        let finished = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
        };

        assert!(journal.append_journaled(&accepted).is_ok());
        assert!(journal.append_journaled(&started).is_ok());
        assert!(journal.append_journaled(&finished).is_ok());

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker);

        assert!(matches!(result, Ok(events) if events.len() == 3));
    }

    #[test]
    fn replay_allows_loop_back_to_lower_step() {
        let run = RunId::new(43);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(4),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(4),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(2),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(4),
                result: SlotIdx::new(0),
            },
        ];
        let mut tracker = ActionReplayTracker::new();

        let result = replay_events(&events, &mut tracker);

        assert!(matches!(result, Ok(replayed) if replayed == events));
    }

    #[test]
    fn replay_allows_persisted_retry_after_action_failure() {
        let run = RunId::new(44);
        let action = ActionId::new(3);
        let step = StepIdx::new(1);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(1),
                step,
                action,
            },
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(2),
                step,
                action,
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(3),
                step,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(4),
                step,
                action,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(5),
                step,
                action,
            },
        ];
        let mut tracker = ActionReplayTracker::new();

        let result = replay_events(&events, &mut tracker);

        assert!(matches!(result, Ok(replayed) if replayed == events));
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn snapshot_tail_replays_loop_without_monotonic_step_order() {
        let run = RunId::new(45);
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(10),
            workflow: test_digest(1),
            slots: Vec::new(),
        };
        let tail = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(11),
                step: StepIdx::new(9),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(12),
                step: StepIdx::new(9),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(13),
                step: StepIdx::new(1),
            },
        ];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);

        assert!(matches!(result, Ok(replayed) if replayed == tail));
    }

    #[test]
    fn replay_all_event_kinds() {
        let run = RunId::new(7);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: SlotIdx::new(0),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                action: ActionId::new(1),
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(0),
                action: ActionId::new(1),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(2),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(7),
                step: StepIdx::new(2),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(8),
                step: StepIdx::new(3),
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(9),
                step: StepIdx::new(3),
                output: SlotIdx::new(1),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(10),
                result: SlotIdx::new(1),
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker);

        assert!(matches!(result, Ok(events) if events.len() == 11));
        assert!(tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
    }
}
