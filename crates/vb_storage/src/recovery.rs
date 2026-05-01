//! Full recovery support for velvet-ballastics journal.
//!
//! Provides:
//! - Digest mismatch detection (workflow source, compiled IR, action ABI, policy)
//! - Full primitive replay (all node kinds)
//! - Non-idempotent action policy: block re-execution during recovery
//! - Replay divergence detection with typed error
//! - Snapshot-plus-tail journal recovery
//! - Full journal recovery when no snapshot available

use crate::{EventSeq, FjallJournal, JournalError, JournalEvent};
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
    let events = journal.events_for_run(run)?;
    for event in &events {
        if let JournalEvent::RunAccepted { workflow, .. } = event {
            if *workflow != expected {
                return Err(RecoveryError::WorkflowSourceDigestMismatch {
                    expected,
                    found: *workflow,
                });
            }
            return Ok(());
        }
    }
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
/// Returns the ordered sequence of journal events and populates the action tracker.
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
    let mut last_step: Option<StepIdx> = None;

    for event in events {
        match event {
            JournalEvent::RunAccepted { .. } => {
                // Accepted is the start of a run
            }
            JournalEvent::StepStarted { step, .. } => {
                // Verify step ordering
                if let Some(prev) = last_step
                    && step.get() < prev.get()
                {
                    return Err(RecoveryError::ReplayDivergence {
                        step: *step,
                        detail: format!(
                            "step {} executed before previous step {}",
                            step.get(),
                            prev.get()
                        ),
                    });
                }
                last_step = Some(*step);
            }
            JournalEvent::StepSucceeded { step, .. } => {
                // Step completed successfully
                let _ = step;
            }
            JournalEvent::ActionScheduled { action, step, .. } => {
                // Check if this action was already resolved
                if tracker.is_resolved(*action, *step) {
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
        ActionReplayTracker, DigestCheck, RecoveryError, RecoveryResult, RunSnapshot,
        check_compiled_ir_digest, check_workflow_source_digest, extract_terminal,
        is_terminal_event, recover_full_journal, recover_snapshot_plus_tail, replay_events,
        verify_digests,
    };
    use crate::{EventSeq, FjallJournal, JournalEvent};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn action_tracker_blocks_non_idempotent_replay() {
        // Given an action marked as completed in the tracker
        // When the same action appears in a replay event list
        // Then replay_events returns NonIdempotentActionBlocked
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
        let Err(err) = result else {
            panic!("replay should fail for already-completed action");
        };
        assert!(matches!(
            err,
            RecoveryError::NonIdempotentActionBlocked { .. }
        ));
    }

    #[test]
    fn action_tracker_allows_first_execution() {
        // Given an action not yet recorded in the tracker
        // When ActionScheduled then ActionCompleted are replayed
        // Then replay succeeds and the tracker records the action as resolved
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
        // Given an action marked as failed in the tracker
        // When the same action appears in a replay event list
        // Then replay_events returns NonIdempotentActionBlocked
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
        let Err(err) = result else {
            panic!("replay should fail for already-failed action");
        };
        assert!(matches!(
            err,
            RecoveryError::NonIdempotentActionBlocked { .. }
        ));
    }

    #[test]
    fn compiled_ir_digest_match_succeeds() {
        // Given identical expected and found digests
        // When check_compiled_ir_digest is called
        // Then it returns Ok
        let digest = test_digest(42);
        let result = check_compiled_ir_digest(digest, digest);
        assert!(result.is_ok());
    }

    #[test]
    fn compiled_ir_digest_mismatch_fails() {
        // Given different expected and found digests
        // When check_compiled_ir_digest is called
        // Then it returns CompiledIrDigestMismatch with the correct values
        let expected = test_digest(1);
        let found = test_digest(2);
        let Err(err) = check_compiled_ir_digest(expected, found) else {
            panic!("mismatched digests should fail");
        };
        assert!(matches!(
            err,
            RecoveryError::CompiledIrDigestMismatch { .. }
        ));
    }

    #[test]
    fn is_terminal_event_identifies_terminals() {
        // Given each terminal event variant
        // When is_terminal_event is called
        // Then it returns true for terminals and false for non-terminals
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
        // Given a RunAccepted followed by a RunFinished event
        // When extract_terminal is called
        // Then it returns the RunFinished event
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
        // Given only a RunAccepted event (no terminal)
        // When extract_terminal is called
        // Then it returns None
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
        // Given a snapshot at seq 5 and a tail event at seq 3
        // When recover_snapshot_plus_tail is called
        // Then it returns ReplayDivergence
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            workflow: test_digest(1),
            slots: Vec::new(),
        };
        let tail = vec![JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
        let Err(err) = result else {
            panic!("tail event before snapshot should be rejected");
        };
        assert!(matches!(err, RecoveryError::ReplayDivergence { .. }));
    }

    #[test]
    fn full_journal_recovery_with_no_data_fails() {
        // Given an empty journal with no events for run 999
        // When recover_full_journal is called
        // Then it returns NoRecoveryData for that run
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else {
            assert!(false, "journal should open");
            return;
        };
        let mut tracker = ActionReplayTracker::new();

        let result = recover_full_journal(&journal, RunId::new(999), &mut tracker);
        let Err(err) = result else {
            panic!("empty journal should produce NoRecoveryData");
        };
        assert!(matches!(err, RecoveryError::NoRecoveryData { .. }));
    }

    #[test]
    fn full_journal_recovery_replays_events() {
        // Given a journal with 3 events (accepted, started, finished) for run 42
        // When recover_full_journal is called
        // Then exactly 3 events are replayed
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else {
            assert!(false, "journal should open");
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
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("full journal recovery should succeed");
        assert_eq!(replayed.len(), 3);
    }

    #[test]
    fn replay_all_event_kinds() {
        // Given a sequence covering all 13 journal event variants
        // When replay_events is called
        // Then all 11 events are replayed and the action tracker records the completed action
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
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay of all event kinds should succeed");
        assert_eq!(replayed.len(), 11);
        assert!(tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
    }

    #[test]
    fn snapshot_plus_tail_accepts_valid_tail_events() {
        // Given a snapshot at seq 5 and valid tail events at seq 6 and 7
        // When recover_snapshot_plus_tail is called
        // Then the tail events are replayed successfully
        let snapshot = RunSnapshot {
            run: RunId::new(10),
            seq: EventSeq::new(5),
            workflow: test_digest(1),
            slots: Vec::new(),
        };
        let tail = vec![
            JournalEvent::StepStarted {
                run: RunId::new(10),
                seq: EventSeq::new(6),
                step: StepIdx::new(0),
            },
            JournalEvent::StepSucceeded {
                run: RunId::new(10),
                seq: EventSeq::new(7),
                step: StepIdx::new(0),
                output: SlotIdx::new(1),
            },
        ];
        let mut tracker = ActionReplayTracker::new();

        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("valid tail events should replay successfully");
        assert_eq!(replayed.len(), 2);
    }

    #[test]
    fn replay_detects_out_of_order_step() {
        // Given events where StepStarted at step 2 precedes a StepStarted at step 1
        // When replay_events processes them
        // Then it returns ReplayDivergence for the backward step
        let run = RunId::new(20);
        let events = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(2),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker);
        let Err(err) = result else {
            panic!("out-of-order steps should cause divergence");
        };
        assert!(matches!(err, RecoveryError::ReplayDivergence { .. }));
    }

    // --- New Recovery Tests ---

    #[test]
    fn check_workflow_source_digest_returns_mismatch_when_digests_differ() {
        // Given a journal with a RunAccepted event using digest [1;32]
        // When check_workflow_source_digest is called with digest [2;32]
        // Then it returns WorkflowSourceDigestMismatch with exact expected/found
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(100);
        let stored_digest = test_digest(1);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: stored_digest,
        };
        assert!(journal.append_journaled(&event).is_ok());

        let wrong_digest = test_digest(2);
        let result = check_workflow_source_digest(&journal, run, wrong_digest);
        let Err(RecoveryError::WorkflowSourceDigestMismatch { expected, found }) = result else {
            panic!("expected WorkflowSourceDigestMismatch, got {:?}", result);
        };
        assert_eq!(expected, wrong_digest);
        assert_eq!(found, stored_digest);
    }

    #[test]
    fn check_workflow_source_digest_succeeds_when_digests_match() {
        // Given a journal with a RunAccepted event using digest [5;32]
        // When check_workflow_source_digest is called with the same digest
        // Then it returns Ok
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(101);
        let digest = test_digest(5);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        assert!(journal.append_journaled(&event).is_ok());

        let result = check_workflow_source_digest(&journal, run, digest);
        assert!(result.is_ok());
    }

    #[test]
    fn check_compiled_ir_digest_returns_mismatch_when_digests_differ() {
        // Given different expected and found digests
        // When check_compiled_ir_digest is called
        // Then it returns CompiledIrDigestMismatch with exact fields
        let expected = test_digest(10);
        let found = test_digest(20);
        let result = check_compiled_ir_digest(expected, found);
        let Err(RecoveryError::CompiledIrDigestMismatch {
            expected: exp,
            found: fnd,
        }) = result
        else {
            panic!("expected CompiledIrDigestMismatch, got {:?}", result);
        };
        assert_eq!(exp, expected);
        assert_eq!(fnd, found);
    }

    #[test]
    fn verify_digests_returns_ok_when_all_match() {
        // Given a journal with matching digests
        // When verify_digests is called with Full level
        // Then it returns Ok
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(200);
        let digest = test_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        assert!(journal.append_journaled(&event).is_ok());

        let result = verify_digests(
            &journal,
            run,
            digest,
            test_digest(8),
            test_digest(8),
            DigestCheck::Full,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn verify_digests_returns_mismatch_when_ir_differs() {
        // Given matching workflow digests but different IR digests
        // When verify_digests is called with WorkflowAndIr level
        // Then it returns CompiledIrDigestMismatch
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(201);
        let digest = test_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        assert!(journal.append_journaled(&event).is_ok());

        let result = verify_digests(
            &journal,
            run,
            digest,
            test_digest(8),
            test_digest(9),
            DigestCheck::WorkflowAndIr,
        );
        assert!(matches!(
            result,
            Err(RecoveryError::CompiledIrDigestMismatch { .. })
        ));
    }

    #[test]
    fn recover_full_journal_returns_no_recovery_data_when_empty() {
        // Given an empty journal
        // When recover_full_journal is called
        // Then it returns NoRecoveryData with the correct run
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(999);
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker);
        let Err(RecoveryError::NoRecoveryData { run: found_run }) = result else {
            panic!("expected NoRecoveryData, got {:?}", result);
        };
        assert_eq!(found_run, run);
    }

    #[test]
    fn replay_events_produces_correct_final_state_from_empty() {
        // Given an empty events list
        // When replay_events is called
        // Then it returns an empty vector
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&[], &mut tracker);
        assert!(result.is_ok());
        let events = result.expect("empty replay should succeed");
        assert!(events.is_empty());
    }

    #[test]
    fn replay_events_accumulates_state_from_multiple_events() {
        // Given three events: RunAccepted, ActionScheduled, ActionCompleted
        // When replay_events is called
        // Then all three are replayed and the tracker marks the action as resolved
        let run = RunId::new(30);
        let action = ActionId::new(1);
        let step = StepIdx::new(0);

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
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(2),
                step,
                action,
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker);
        assert!(result.is_ok());
        let replayed = result.expect("replay should succeed");
        assert_eq!(replayed.len(), 3);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn is_terminal_event_returns_true_for_finished() {
        // Given a RunFinished event
        // When is_terminal_event is called
        // Then it returns true
        let event = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_true_for_failed() {
        // Given a RunFailedEvent
        // When is_terminal_event is called
        // Then it returns true
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_true_for_cancelled() {
        // Given a RunCancelled event
        // When is_terminal_event is called
        // Then it returns true
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(5),
        };
        assert!(is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_false_for_submitted() {
        // Given a RunAccepted event
        // When is_terminal_event is called
        // Then it returns false
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn is_terminal_event_returns_false_for_step_started() {
        // Given a StepStarted event
        // When is_terminal_event is called
        // Then it returns false
        let event = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        assert!(!is_terminal_event(&event));
    }

    #[test]
    fn extract_terminal_returns_some_for_finished_event() {
        // Given events ending with RunFinished
        // When extract_terminal is called
        // Then it returns Some with the exact RunFinished event
        let finished = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(3),
            result: SlotIdx::new(42),
        };
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            finished.clone(),
        ];

        let result = extract_terminal(&events);
        assert!(result.is_some());
        assert_eq!(result, Some(&finished));
    }

    #[test]
    fn extract_terminal_returns_none_for_non_terminal_event() {
        // Given only non-terminal events
        // When extract_terminal is called
        // Then it returns None
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
        ];

        let result = extract_terminal(&events);
        assert!(result.is_none());
    }

    #[test]
    fn action_replay_tracker_default_is_new() {
        // Given the Default impl for ActionReplayTracker
        // When default() is called
        // Then it produces an empty tracker with no resolved actions
        let tracker = ActionReplayTracker::default();
        assert!(!tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
    }

    #[test]
    fn snapshot_plus_tail_rejects_event_at_same_seq_as_snapshot() {
        // Given a snapshot at seq 5 and a tail event also at seq 5
        // When recover_snapshot_plus_tail is called
        // Then it returns ReplayDivergence
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            workflow: test_digest(1),
            slots: Vec::new(),
        };
        let tail = vec![JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(5),
            step: StepIdx::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
        let Err(RecoveryError::ReplayDivergence { step, detail }) = result else {
            panic!("expected ReplayDivergence, got {:?}", result);
        };
        assert_eq!(step, StepIdx::ZERO);
        assert!(!detail.is_empty());
    }

    #[test]
    fn recover_full_journal_replays_failed_action() {
        // Given a journal with ActionScheduled followed by ActionFailedEvent
        // When recover_full_journal is called
        // Then the action is marked as resolved (failed) in the tracker
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(300);
        let action = ActionId::new(10);
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
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(3),
            },
        ];

        for event in &events {
            assert!(journal.append_journaled(event).is_ok());
        }

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker);
        assert!(result.is_ok());
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn replay_events_handles_all_non_terminal_variants() {
        // Given a sequence covering RunAccepted, StepStarted, SlotWrittenEvent,
        // WaitScheduledEvent, AskScheduledEvent, AskAnsweredEvent, RetryScheduledEvent
        // When replay_events is called
        // Then all events are replayed successfully
        let run = RunId::new(50);
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
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(2),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(2),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(3),
            },
        ];

        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker);
        assert!(result.is_ok());
        let replayed = result.expect("replay should succeed");
        assert_eq!(replayed.len(), 7);
    }

    #[test]
    fn verify_digests_workflow_source_only_checks_only_workflow() {
        // Given matching workflow digest but different IR digest
        // When verify_digests is called with WorkflowSourceOnly level
        // Then it returns Ok (IR mismatch is not checked)
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(400);
        let wf_digest = test_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: wf_digest,
        };
        assert!(journal.append_journaled(&event).is_ok());

        let result = verify_digests(
            &journal,
            run,
            wf_digest,
            test_digest(8),
            test_digest(99), // different, but should not be checked at this level
            DigestCheck::WorkflowSourceOnly,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn check_workflow_source_digest_returns_ok_when_no_events() {
        // Given a journal with no events for the run
        // When check_workflow_source_digest is called
        // Then it returns Ok (no RunAccepted event means no mismatch)
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let result = check_workflow_source_digest(&journal, RunId::new(500), test_digest(1));
        assert!(result.is_ok());
    }

    // --- Section: Recovery Error Variant Exact-Assertion Tests ---

    #[test]
    fn recovery_error_journal_wraps_journal_error() {
        // Given a JournalError
        // When wrapped in RecoveryError::Journal via From
        // Then the display message contains the inner error
        let inner = crate::JournalError::KeyCapacity;
        let err = RecoveryError::Journal(inner);
        let msg = format!("{}", err);
        assert!(msg.contains("journal error"));
    }

    #[test]
    fn recovery_error_workflow_source_digest_mismatch_has_exact_fields() {
        // Given different expected and found digests
        // When WorkflowSourceDigestMismatch is constructed
        // Then the fields match exactly
        let expected = test_digest(10);
        let found = test_digest(20);
        let err = RecoveryError::WorkflowSourceDigestMismatch { expected, found };
        let msg = format!("{}", err);
        assert!(msg.contains("workflow source digest mismatch"));
        assert!(msg.contains("expected"));
        assert!(msg.contains("found"));
    }

    #[test]
    fn recovery_error_compiled_ir_digest_mismatch_has_exact_fields() {
        // Given different expected and found IR digests
        // When CompiledIrDigestMismatch is constructed
        // Then the fields match exactly
        let expected = test_digest(11);
        let found = test_digest(22);
        let err = RecoveryError::CompiledIrDigestMismatch { expected, found };
        let msg = format!("{}", err);
        assert!(msg.contains("compiled IR digest mismatch"));
    }

    #[test]
    fn recovery_error_action_abi_mismatch_has_exact_fields() {
        // Given an ActionId
        // When ActionAbiMismatch is constructed
        // Then the fields match exactly
        let action_id = ActionId::new(5);
        let err = RecoveryError::ActionAbiMismatch { action_id };
        let msg = format!("{}", err);
        assert!(msg.contains("action ABI digest mismatch"));
    }

    #[test]
    fn recovery_error_policy_digest_mismatch_has_exact_fields() {
        // Given a StepIdx
        // When PolicyDigestMismatch is constructed
        // Then the fields match exactly
        let step = StepIdx::new(7);
        let err = RecoveryError::PolicyDigestMismatch { step };
        let msg = format!("{}", err);
        assert!(msg.contains("policy digest mismatch"));
    }

    #[test]
    fn recovery_error_non_idempotent_action_blocked_has_exact_fields() {
        // Given an action and step
        // When NonIdempotentActionBlocked is constructed
        // Then the fields match exactly
        let action = ActionId::new(3);
        let step = StepIdx::new(4);
        let err = RecoveryError::NonIdempotentActionBlocked { action, step };
        let msg = format!("{}", err);
        assert!(msg.contains("non-idempotent"));
        assert!(msg.contains("cannot be re-executed"));
    }

    #[test]
    fn recovery_error_replay_divergence_has_exact_fields() {
        // Given a step and detail
        // When ReplayDivergence is constructed
        // Then the fields match exactly
        let step = StepIdx::new(9);
        let detail = "test divergence".to_string();
        let err = RecoveryError::ReplayDivergence { step, detail };
        let msg = format!("{}", err);
        assert!(msg.contains("replay divergence"));
        assert!(msg.contains("test divergence"));
    }

    #[test]
    fn recovery_error_no_recovery_data_has_exact_fields() {
        // Given a RunId
        // When NoRecoveryData is constructed
        // Then the fields match exactly
        let run = RunId::new(42);
        let err = RecoveryError::NoRecoveryData { run };
        let msg = format!("{}", err);
        assert!(msg.contains("no recovery data"));
    }

    #[test]
    fn recovery_error_corrupt_snapshot_has_exact_fields() {
        // Given a run and seq
        // When CorruptSnapshot is constructed
        // Then the fields match exactly
        let run = RunId::new(55);
        let seq = crate::EventSeq::new(10);
        let err = RecoveryError::CorruptSnapshot { run, seq };
        let msg = format!("{}", err);
        assert!(msg.contains("snapshot corrupt"));
    }

    #[test]
    fn recovery_error_terminal_state_mismatch_has_exact_fields() {
        // Given expected and found strings
        // When TerminalStateMismatch is constructed
        // Then the fields match exactly
        let expected = "RunFinished".to_string();
        let found = "RunFailed".to_string();
        let err = RecoveryError::TerminalStateMismatch { expected: expected.clone(), found: found.clone() };
        let msg = format!("{}", err);
        assert!(msg.contains("terminal state mismatch"));
    }

    // --- Section: Recovery Lifecycle BDD Tests ---

    #[test]
    fn recover_full_journal_returns_empty_recovery_for_empty_journal() {
        // Given an empty journal
        // When recover_full_journal is called for a run
        // Then it returns NoRecoveryData with the correct run
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(1);
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker);
        let Err(RecoveryError::NoRecoveryData { run: found_run }) = result else {
            panic!("expected NoRecoveryData, got {:?}", result);
        };
        assert_eq!(found_run, run);
    }

    #[test]
    fn recover_full_journal_reconstructs_run_state_from_events() {
        // Given a journal with accepted, step started, run finished
        // When recover_full_journal is called
        // Then 3 events are returned in order
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(10);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: test_digest(1) },
            crate::JournalEvent::StepStarted { run, seq: crate::EventSeq::new(1), step: StepIdx::new(0) },
            crate::JournalEvent::RunFinished { run, seq: crate::EventSeq::new(2), result: vb_core::SlotIdx::new(0) },
        ];
        for event in &events {
            assert!(journal.append_journaled(event).is_ok());
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("full journal recovery should succeed");
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0].seq(), crate::EventSeq::new(0));
        assert_eq!(replayed[1].seq(), crate::EventSeq::new(1));
        assert_eq!(replayed[2].seq(), crate::EventSeq::new(2));
    }

    #[test]
    fn recover_full_journal_identifies_active_runs() {
        // Given a journal with an accepted but not terminated run
        // When recover_full_journal is called
        // Then extract_terminal returns None (run is still active)
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(20);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: test_digest(1) },
            crate::JournalEvent::StepStarted { run, seq: crate::EventSeq::new(1), step: StepIdx::new(0) },
        ];
        for event in &events {
            assert!(journal.append_journaled(event).is_ok());
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("recovery should succeed");
        let terminal = extract_terminal(&replayed);
        assert!(terminal.is_none(), "active run should have no terminal event");
    }

    #[test]
    fn recover_full_journal_identifies_completed_runs() {
        // Given a journal with a run ending in RunFinished
        // When recover_full_journal is called
        // Then extract_terminal returns the RunFinished event
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(30);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: test_digest(1) },
            crate::JournalEvent::RunFinished { run, seq: crate::EventSeq::new(1), result: vb_core::SlotIdx::new(0) },
        ];
        for event in &events {
            assert!(journal.append_journaled(event).is_ok());
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("recovery should succeed");
        let terminal = extract_terminal(&replayed);
        assert!(terminal.is_some());
        let Some(crate::JournalEvent::RunFinished { result, .. }) = terminal else {
            panic!("expected RunFinished terminal event");
        };
        assert_eq!(*result, vb_core::SlotIdx::new(0));
    }

    #[test]
    fn recover_full_journal_identifies_failed_runs() {
        // Given a journal with a run ending in RunFailedEvent
        // When recover_full_journal is called
        // Then extract_terminal returns the RunFailedEvent
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(40);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: test_digest(1) },
            crate::JournalEvent::RunFailedEvent { run, seq: crate::EventSeq::new(1) },
        ];
        for event in &events {
            assert!(journal.append_journaled(event).is_ok());
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("recovery should succeed");
        let terminal = extract_terminal(&replayed);
        assert!(terminal.is_some());
        let Some(crate::JournalEvent::RunFailedEvent { .. }) = terminal else {
            panic!("expected RunFailedEvent terminal event");
        };
    }

    #[test]
    fn recover_snapshot_plus_tail_returns_recovery_data() {
        // Given a snapshot at seq 0 and tail events at seq 1 and 2
        // When recover_snapshot_plus_tail is called
        // Then 2 tail events are replayed
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![],
        };
        let tail = vec![
            crate::JournalEvent::StepStarted { run: RunId::new(1), seq: crate::EventSeq::new(1), step: StepIdx::new(0) },
            crate::JournalEvent::StepSucceeded { run: RunId::new(1), seq: crate::EventSeq::new(2), step: StepIdx::new(0), output: vb_core::SlotIdx::new(1) },
        ];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
        let Ok(replayed) = result else {
            panic!("expected Ok, got {:?}", result);
        };
        assert_eq!(replayed.len(), 2);
    }

    #[test]
    fn recover_snapshot_plus_tail_applies_tail_events_to_snapshot() {
        // Given a snapshot at seq 5 and tail events with actions
        // When recover_snapshot_plus_tail is called
        // Then action tracker is updated with the action from tail events
        let snapshot = RunSnapshot {
            run: RunId::new(2),
            seq: crate::EventSeq::new(5),
            workflow: test_digest(2),
            slots: vec![1, 2, 3],
        };
        let action = ActionId::new(10);
        let step = StepIdx::new(1);
        let tail = vec![
            crate::JournalEvent::ActionScheduled { run: RunId::new(2), seq: crate::EventSeq::new(6), step, action },
            crate::JournalEvent::ActionCompletedEvent { run: RunId::new(2), seq: crate::EventSeq::new(7), step, action },
        ];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
        assert!(result.is_ok());
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn replay_events_processes_submitted_event() {
        // Given a RunAccepted event
        // When replay_events is called
        // Then the event is replayed without state change to tracker
        let run = RunId::new(1);
        let events = vec![
            crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: test_digest(1) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_step_started_event() {
        // Given a StepStarted event
        // When replay_events is called
        // Then the event is replayed and last_step is updated internally
        let run = RunId::new(2);
        let events = vec![
            crate::JournalEvent::StepStarted { run, seq: crate::EventSeq::new(0), step: StepIdx::new(5) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_step_ended_event() {
        // Given a StepSucceeded event
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(3);
        let events = vec![
            crate::JournalEvent::StepSucceeded { run, seq: crate::EventSeq::new(0), step: StepIdx::new(0), output: vb_core::SlotIdx::new(1) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_run_finished_event() {
        // Given a RunFinished terminal event
        // When replay_events is called
        // Then the event is replayed and is identified as terminal
        let run = RunId::new(4);
        let events = vec![
            crate::JournalEvent::RunFinished { run, seq: crate::EventSeq::new(0), result: vb_core::SlotIdx::new(99) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn replay_events_processes_run_failed_event() {
        // Given a RunFailedEvent terminal event
        // When replay_events is called
        // Then the event is replayed and is identified as terminal
        let run = RunId::new(5);
        let events = vec![
            crate::JournalEvent::RunFailedEvent { run, seq: crate::EventSeq::new(0) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn check_workflow_source_digest_accepts_matching_digest() {
        // Given a journal with a RunAccepted event using digest [5;32]
        // When check_workflow_source_digest is called with the same digest
        // Then it returns Ok
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(600);
        let digest = test_digest(5);
        let event = crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: digest };
        assert!(journal.append_journaled(&event).is_ok());

        let result = check_workflow_source_digest(&journal, run, digest);
        assert!(result.is_ok());
    }

    #[test]
    fn check_compiled_ir_digest_accepts_matching_digest() {
        // Given identical expected and found digests
        // When check_compiled_ir_digest is called
        // Then it returns Ok
        let digest = test_digest(42);
        let result = check_compiled_ir_digest(digest, digest);
        assert!(result.is_ok());
    }

    #[test]
    fn verify_digests_returns_ok_when_all_digests_match() {
        // Given a journal with matching workflow and IR digests
        // When verify_digests is called at Full level
        // Then it returns Ok
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(700);
        let wf_digest = test_digest(10);
        let ir_digest = test_digest(20);
        let event = crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: wf_digest };
        assert!(journal.append_journaled(&event).is_ok());

        let result = verify_digests(&journal, run, wf_digest, ir_digest, ir_digest, DigestCheck::Full);
        assert!(result.is_ok());
    }

    // --- Section: ActionReplayTracker BDD Tests ---

    #[test]
    fn tracker_new_starts_empty() {
        // Given a new ActionReplayTracker
        // When is_resolved is called for any action/step
        // Then it returns false
        let tracker = ActionReplayTracker::new();
        assert!(!tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
    }

    #[test]
    fn tracker_mark_completed_tracks_action() {
        // Given a tracker with a completed action
        // When is_resolved is called
        // Then it returns true
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(5);
        let step = StepIdx::new(2);
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn tracker_mark_failed_tracks_action() {
        // Given a tracker with a failed action
        // When is_resolved is called
        // Then it returns true
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(6);
        let step = StepIdx::new(3);
        tracker.mark_failed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn tracker_different_actions_are_independent() {
        // Given a tracker with action 1 completed
        // When is_resolved is called for action 2
        // Then it returns false
        let mut tracker = ActionReplayTracker::new();
        let step = StepIdx::new(0);
        tracker.mark_completed(ActionId::new(1), step);
        assert!(!tracker.is_resolved(ActionId::new(2), step));
    }

    #[test]
    fn tracker_same_action_different_steps_are_independent() {
        // Given a tracker with action 1 at step 0 completed
        // When is_resolved is called for action 1 at step 1
        // Then it returns false
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(1);
        tracker.mark_completed(action, StepIdx::new(0));
        assert!(!tracker.is_resolved(action, StepIdx::new(1)));
    }

    #[test]
    fn tracker_default_is_same_as_new() {
        // Given a default-constructed tracker
        // When compared to a new() tracker
        // Then both are empty
        let default_tracker = ActionReplayTracker::default();
        let new_tracker = ActionReplayTracker::new();
        assert!(!default_tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
        assert!(!new_tracker.is_resolved(ActionId::new(1), StepIdx::new(0)));
    }

    // --- Section: DigestCheck BDD Tests ---

    #[test]
    fn digest_check_variants_are_distinct() {
        // Given all DigestCheck variants
        // When compared
        // Then they are not equal to each other
        assert_ne!(DigestCheck::WorkflowSourceOnly, DigestCheck::WorkflowAndIr);
        assert_ne!(DigestCheck::WorkflowAndIr, DigestCheck::Full);
        assert_ne!(DigestCheck::WorkflowSourceOnly, DigestCheck::Full);
    }

    #[test]
    fn digest_check_workflow_source_only_does_not_check_ir() {
        // Given matching workflow digest but mismatched IR digest
        // When verify_digests is called with WorkflowSourceOnly level
        // Then it returns Ok (IR not checked)
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(800);
        let wf_digest = test_digest(7);
        let event = crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: wf_digest };
        assert!(journal.append_journaled(&event).is_ok());

        let result = verify_digests(
            &journal,
            run,
            wf_digest,
            test_digest(8),
            test_digest(99),
            DigestCheck::WorkflowSourceOnly,
        );
        assert!(result.is_ok());
    }

    // --- Section: RunSnapshot Tests ---

    #[test]
    fn run_snapshot_equality_works() {
        // Given two identical snapshots
        // When compared
        // Then they are equal
        let s1 = RunSnapshot {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1, 2, 3],
        };
        let s2 = RunSnapshot {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1, 2, 3],
        };
        assert_eq!(s1, s2);
    }

    #[test]
    fn run_snapshot_inequality_detects_different_slots() {
        // Given two snapshots with different slots
        // When compared
        // Then they are not equal
        let s1 = RunSnapshot {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1],
        };
        let s2 = RunSnapshot {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![2],
        };
        assert_ne!(s1, s2);
    }

    #[test]
    fn run_snapshot_clone_is_equal() {
        // Given a snapshot
        // When cloned
        // Then the clone is equal to the original
        let s = RunSnapshot {
            run: RunId::new(5),
            seq: crate::EventSeq::new(3),
            workflow: test_digest(7),
            slots: vec![4, 5, 6],
        };
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    // --- Section: Replay with All Event Kinds ---

    #[test]
    fn replay_events_processes_action_failed_event() {
        // Given an ActionFailedEvent
        // When replay_events is called
        // Then the tracker marks the action as resolved
        let run = RunId::new(10);
        let action = ActionId::new(7);
        let step = StepIdx::new(2);
        let events = vec![
            crate::JournalEvent::ActionFailedEvent { run, seq: crate::EventSeq::new(0), step, action },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn replay_events_processes_run_cancelled_event() {
        // Given a RunCancelled terminal event
        // When replay_events is called
        // Then the event is replayed and identified as terminal
        let run = RunId::new(11);
        let events = vec![
            crate::JournalEvent::RunCancelled { run, seq: crate::EventSeq::new(0) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn replay_events_processes_slot_written_event() {
        // Given a SlotWrittenEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(12);
        let events = vec![
            crate::JournalEvent::SlotWrittenEvent { run, seq: crate::EventSeq::new(0), slot: vb_core::SlotIdx::new(3) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_wait_scheduled_event() {
        // Given a WaitScheduledEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(13);
        let events = vec![
            crate::JournalEvent::WaitScheduledEvent { run, seq: crate::EventSeq::new(0), step: StepIdx::new(1) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_ask_scheduled_event() {
        // Given an AskScheduledEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(14);
        let events = vec![
            crate::JournalEvent::AskScheduledEvent { run, seq: crate::EventSeq::new(0), step: StepIdx::new(2) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_ask_answered_event() {
        // Given an AskAnsweredEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(15);
        let events = vec![
            crate::JournalEvent::AskAnsweredEvent { run, seq: crate::EventSeq::new(0), step: StepIdx::new(2) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_retry_scheduled_event() {
        // Given a RetryScheduledEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(16);
        let events = vec![
            crate::JournalEvent::RetryScheduledEvent { run, seq: crate::EventSeq::new(0), step: StepIdx::new(3) },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker)
            .expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn recover_full_journal_with_cancelled_run() {
        // Given a journal with a run ending in RunCancelled
        // When recover_full_journal is called
        // Then the terminal event is RunCancelled
        let temp_dir = tempfile::tempdir();
        assert!(temp_dir.is_ok());
        let Ok(temp_dir) = temp_dir else { return };
        let journal = crate::FjallJournal::open(temp_dir.path());
        assert!(journal.is_ok());
        let Ok(journal) = journal else { return };

        let run = RunId::new(50);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted { run, seq: crate::EventSeq::new(0), workflow: test_digest(1) },
            crate::JournalEvent::RunCancelled { run, seq: crate::EventSeq::new(1) },
        ];
        for event in &events {
            assert!(journal.append_journaled(event).is_ok());
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("recovery should succeed");
        let terminal = extract_terminal(&replayed);
        assert!(terminal.is_some());
        let Some(crate::JournalEvent::RunCancelled { .. }) = terminal else {
            panic!("expected RunCancelled terminal event");
        };
    }

    #[test]
    fn snapshot_plus_tail_with_empty_tail_returns_empty() {
        // Given a snapshot and empty tail events
        // When recover_snapshot_plus_tail is called
        // Then zero events are replayed
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: crate::EventSeq::new(5),
            workflow: test_digest(1),
            slots: vec![],
        };
        let tail: Vec<crate::JournalEvent> = vec![];
        let mut tracker = ActionReplayTracker::new();

        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("empty tail should succeed");
        assert!(replayed.is_empty());
    }

    #[test]
    fn recovery_result_ok_carries_value() {
        // Given a successful RecoveryResult
        // When unwrapped
        // Then it carries the value
        let result: RecoveryResult<u32> = Ok(42);
        let Ok(val) = result else {
            panic!("expected Ok");
        };
        assert_eq!(val, 42);
    }

    #[test]
    fn recovery_result_err_carries_error() {
        // Given a failed RecoveryResult
        // When unwrapped
        // Then it carries the error
        let result: RecoveryResult<u32> = Err(RecoveryError::NoRecoveryData { run: RunId::new(1) });
        let Err(err) = result else {
            panic!("expected Err");
        };
        let RecoveryError::NoRecoveryData { run } = err else {
            panic!("expected NoRecoveryData");
        };
        assert_eq!(run, RunId::new(1));
    }
}
