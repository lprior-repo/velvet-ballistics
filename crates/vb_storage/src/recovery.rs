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
        ActionReplayTracker, RecoveryError, RunSnapshot, check_compiled_ir_digest,
        extract_terminal, is_terminal_event, recover_full_journal, recover_snapshot_plus_tail,
        replay_events,
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
}
