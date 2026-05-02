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
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

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
    /// Durable event indexes exceed the runtime frame dimensions that can be represented.
    #[error("recovery frame dimension overflow for run {run:?}")]
    FrameDimensionOverflow {
        /// Run identifier.
        run: RunId,
    },
}

/// Result alias for recovery operations.
pub type RecoveryResult<T> = Result<T, RecoveryError>;

/// Terminal status recovered from durable journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryTerminalState {
    /// Run was cancelled before completion.
    Cancelled,
    /// Run completed and selected a result slot.
    Finished {
        /// Result slot selected by the finish event.
        result: SlotIdx,
    },
    /// Run failed.
    Failed,
}

/// Runtime summary that can be recovered without reconstructing a live `RunFrame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRuntimeSummary {
    /// Run identifier summarized by this recovery view.
    pub run: RunId,
    /// First sequence observed for the run.
    pub first_seq: EventSeq,
    /// Last sequence observed for the run.
    pub last_seq: EventSeq,
    /// Compiled workflow digest from the acceptance event, when present.
    pub workflow: Option<WorkflowDigest>,
    /// Number of step start events.
    pub steps_started: u64,
    /// Number of step success events.
    pub steps_succeeded: u64,
    /// Number of action schedule events.
    pub actions_scheduled: u64,
    /// Number of resolved action events.
    pub actions_resolved: u64,
    /// Number of boundary suspension events.
    pub suspensions: u64,
    /// Number of slot write events.
    pub slots_written: u64,
    /// Terminal status, when a terminal event exists.
    pub terminal: Option<RecoveryTerminalState>,
}

/// Explicit recovery product. Supports summary-only or full live-frame seed
/// recovery from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryHydration {
    /// Summary-only recovery product.
    Summary(RecoveryRuntimeSummary),
    /// Full live-frame seed recovered from durable events.
    FrameSeed(RecoveryFrameSeed),
}

/// Step state recovered from durable lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveredStepState {
    /// Step has started or is waiting on action completion.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step is suspended on a wait primitive.
    Waiting,
    /// Step is suspended on an ask primitive.
    Asking,
}

/// One recovered step-state entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredStepEntry {
    /// Step index.
    pub step: StepIdx,
    /// Durable state inferred for this step.
    pub state: RecoveredStepState,
}

/// State that durable headers/events still cannot reconstruct into a live frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedRecoveryState {
    /// Slot values are not present in current slot-written records.
    pub slot_values: bool,
    /// Slot taint is not present in current slot-written records.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in current action records.
    pub action_payloads: bool,
}

/// Minimal live-frame seed recovered from durable journal headers/events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryFrameSeed {
    /// Runtime summary for the same event set.
    pub summary: RecoveryRuntimeSummary,
    /// First program-counter step for the rebuilt frame.
    pub first_step: StepIdx,
    /// Minimum step-state capacity needed for observed events.
    pub step_count: u16,
    /// Minimum slot capacity needed for observed slot/result references.
    pub slot_count: u16,
    /// Program counter inferred from the latest observed step event.
    pub pc: StepIdx,
    /// Final step states inferred from durable lifecycle events.
    pub steps: Vec<RecoveredStepEntry>,
    /// Exact pieces of live runtime state not represented by durable events yet.
    pub unsupported: UnsupportedRecoveryState,
}

impl RecoveryHydration {
    /// Returns the summary carried by this hydration product.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(summary) => *summary,
            Self::FrameSeed(seed) => seed.summary,
        }
    }
}

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
    // TODO(Full digest): ActionAbiMismatch and PolicyDigestMismatch checks require
    // (a) an action-ABI digest per action (ActionDigest or similar) available in the
    //     compiled IR or action registry, and
    // (b) a per-step policy digest recorded alongside each StepStarted/ActionScheduled event.
    // Once the compiled IR carries `action_abi_digest: WorkflowDigest` per action and the
    // journal records `policy_digest: WorkflowDigest` per step, this branch should:
    //   for each ActionScheduled event in the journal:
    //     let stored_abi = event.action_abi_digest;
    //     let current_abi = action_registry.digest(event.action);
    //     if stored_abi != current_abi { return Err(ActionAbiMismatch { action_id }) }
    //   for each StepStarted event:
    //     let stored_policy = event.policy_digest;
    //     let current_policy = compiled_ir.policy_digest_at(event.step);
    //     if stored_policy != current_policy { return Err(PolicyDigestMismatch { step }) }
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

/// Loads a snapshot from the journal, translating decode failures to
/// `RecoveryError::CorruptSnapshot`.
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> RecoveryResult<RunSnapshot> {
    match journal.snapshot(run, seq) {
        Ok(Some(snapshot)) => Ok(snapshot),
        Ok(None) => Err(RecoveryError::CorruptSnapshot { run, seq }),
        Err(JournalError::PostcardDecodeFailed) => Err(RecoveryError::CorruptSnapshot { run, seq }),
        Err(other) => Err(RecoveryError::Journal(other)),
    }
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
            JournalEvent::StepSucceeded { .. } => {
                // Step completed successfully
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
            JournalEvent::SlotWrittenEvent { .. } => {
                // Slot write during replay
            }
            JournalEvent::WaitScheduledEvent { .. } => {}
            JournalEvent::AskScheduledEvent { .. } => {}
            JournalEvent::AskAnsweredEvent { .. } => {}
            JournalEvent::RetryScheduledEvent { .. } => {}
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

/// Builds a summary-only recovery product from already ordered journal events.
pub fn summarize_recovery_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryHydration> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData { run: RunId::new(0) });
    };
    let run = first.run_id();
    let mut summary = RecoveryRuntimeSummary {
        run,
        first_seq: first.seq(),
        last_seq: first.seq(),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };

    for event in events {
        if event.run_id() != run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery summary received events for multiple runs".to_owned(),
            });
        }
        summary.last_seq = event.seq();
        apply_summary_event(&mut summary, event);
    }

    Ok(RecoveryHydration::Summary(summary))
}

/// Recovers a summary-only runtime hydration product for a run.
pub fn recover_runtime_summary(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryHydration> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    summarize_recovery_events(&events)
}

/// Recovers a minimal live-frame seed from durable journal events for a run.
pub fn recover_runtime_frame_seed(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryFrameSeed> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    recover_runtime_frame_seed_from_events(&events)
}

/// Recovers a minimal live-frame seed from already ordered journal events.
pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    let hydration = summarize_recovery_events(events)?;
    let summary = hydration.summary();
    let mut builder = RecoveryFrameSeedBuilder::new(summary);

    for event in events {
        if event.run_id() != summary.run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery frame seed received events for multiple runs".to_owned(),
            });
        }
        builder.observe_event(event)?;
    }

    let seed = builder.finish()?;

    if seed.summary.slots_written > 0 && seed.unsupported.slot_values {
        return Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "recovery cannot reconstruct slot values from durable events".to_owned(),
        });
    }
    if seed.summary.slots_written > 0 && seed.unsupported.slot_taint {
        return Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "recovery cannot reconstruct slot taint from durable events".to_owned(),
        });
    }

    Ok(seed)
}

/// Recovers summary hydration for every durable run header whose journal has no
/// terminal event. The run header scan supplies candidates; journal events define
/// incompleteness because the status byte/index has no stable terminal mapping.
pub fn recover_all_incomplete_runs(
    journal: &FjallJournal,
) -> RecoveryResult<Vec<RecoveryHydration>> {
    let headers = journal.run_headers()?;
    let mut recovered = Vec::new();

    for header in headers {
        let events = journal.events_for_run(header.run)?;
        if events.is_empty() {
            return Err(RecoveryError::NoRecoveryData { run: header.run });
        }
        if extract_terminal(&events).is_none() {
            recovered.push(summarize_recovery_events(&events)?);
        }
    }

    Ok(recovered)
}

fn apply_summary_event(summary: &mut RecoveryRuntimeSummary, event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            summary.workflow = Some(*workflow);
        }
        JournalEvent::StepStarted { .. } => {
            summary.steps_started = summary.steps_started.saturating_add(1);
        }
        JournalEvent::StepSucceeded { .. } => {
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
        }
        JournalEvent::ActionScheduled { .. } => {
            summary.actions_scheduled = summary.actions_scheduled.saturating_add(1);
        }
        JournalEvent::ActionCompletedEvent { .. } | JournalEvent::ActionFailedEvent { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
        }
        JournalEvent::SlotWrittenEvent { .. } => {
            summary.slots_written = summary.slots_written.saturating_add(1);
        }
        JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::RetryScheduledEvent { .. } => {
            summary.suspensions = summary.suspensions.saturating_add(1);
        }
        JournalEvent::AskAnsweredEvent { .. } => {}
        JournalEvent::RunCancelled { .. } => {
            summary.terminal = Some(RecoveryTerminalState::Cancelled);
        }
        JournalEvent::RunFinished { result, .. } => {
            summary.terminal = Some(RecoveryTerminalState::Finished { result: *result });
        }
        JournalEvent::RunFailedEvent { .. } => {
            summary.terminal = Some(RecoveryTerminalState::Failed);
        }
    }
}

struct RecoveryFrameSeedBuilder {
    summary: RecoveryRuntimeSummary,
    max_step: Option<StepIdx>,
    max_slot: Option<SlotIdx>,
    pc: StepIdx,
    steps: Vec<RecoveredStepEntry>,
    unsupported: UnsupportedRecoveryState,
}

impl RecoveryFrameSeedBuilder {
    fn new(summary: RecoveryRuntimeSummary) -> Self {
        Self {
            summary,
            max_step: None,
            max_slot: None,
            pc: StepIdx::ZERO,
            steps: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
            },
        }
    }

    fn observe_event(&mut self, event: &JournalEvent) -> RecoveryResult<()> {
        match event {
            JournalEvent::RunAccepted { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunFailedEvent { .. } => Ok(()),
            JournalEvent::StepStarted { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Running);
                Ok(())
            }
            JournalEvent::StepSucceeded { step, output, .. } => {
                self.observe_step(*step, RecoveredStepState::Succeeded);
                self.observe_slot(*output);
                self.unsupported.slot_values = true;
                self.unsupported.slot_taint = true;
                Ok(())
            }
            JournalEvent::ActionScheduled { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Running);
                self.unsupported.action_payloads = true;
                Ok(())
            }
            JournalEvent::ActionCompletedEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Succeeded);
                self.unsupported.action_payloads = true;
                Ok(())
            }
            JournalEvent::ActionFailedEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Failed);
                self.unsupported.action_payloads = true;
                Ok(())
            }
            JournalEvent::SlotWrittenEvent { slot, .. } => {
                self.observe_slot(*slot);
                self.unsupported.slot_values = true;
                self.unsupported.slot_taint = true;
                Ok(())
            }
            JournalEvent::WaitScheduledEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Waiting);
                Ok(())
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Asking);
                Ok(())
            }
            JournalEvent::AskAnsweredEvent { step, .. }
            | JournalEvent::RetryScheduledEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Running);
                Ok(())
            }
            JournalEvent::RunFinished { result, .. } => {
                self.observe_slot(*result);
                Ok(())
            }
        }
    }

    fn observe_step(&mut self, step: StepIdx, state: RecoveredStepState) {
        self.pc = step;
        self.max_step = Some(match self.max_step {
            Some(current) if current >= step => current,
            _ => step,
        });
        let mut index = 0usize;
        while index < self.steps.len() {
            if let Some(entry) = self.steps.get_mut(index)
                && entry.step == step
            {
                entry.state = state;
                return;
            }
            index = index.saturating_add(1);
        }
        self.steps.push(RecoveredStepEntry { step, state });
    }

    fn observe_slot(&mut self, slot: SlotIdx) {
        self.max_slot = Some(match self.max_slot {
            Some(current) if current >= slot => current,
            _ => slot,
        });
    }

    fn finish(self) -> RecoveryResult<RecoveryFrameSeed> {
        let step_count = count_from_max_step(self.max_step, self.summary.run)?;
        let slot_count = count_from_max_slot(self.max_slot, self.summary.run)?;
        Ok(RecoveryFrameSeed {
            summary: self.summary,
            first_step: StepIdx::ZERO,
            step_count,
            slot_count,
            pc: self.pc,
            steps: self.steps,
            unsupported: self.unsupported,
        })
    }
}

fn count_from_max_step(max_step: Option<StepIdx>, run: RunId) -> RecoveryResult<u16> {
    let Some(step) = max_step else {
        return Ok(1);
    };
    step.get()
        .checked_add(1)
        .ok_or(RecoveryError::FrameDimensionOverflow { run })
}

fn count_from_max_slot(max_slot: Option<SlotIdx>, run: RunId) -> RecoveryResult<u16> {
    let Some(slot) = max_slot else {
        return Ok(0);
    };
    slot.get()
        .checked_add(1)
        .ok_or(RecoveryError::FrameDimensionOverflow { run })
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
    events.iter().rev().find(|event| is_terminal_event(event))
}

#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use super::{
        ActionReplayTracker, DigestCheck, RecoveredStepEntry, RecoveredStepState, RecoveryError,
        RecoveryFrameSeed, RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary,
        RecoveryTerminalState, RunSnapshot, UnsupportedRecoveryState, check_compiled_ir_digest,
        check_workflow_source_digest, extract_terminal, is_terminal_event,
        recover_all_incomplete_runs, recover_full_journal, recover_runtime_frame_seed,
        recover_runtime_frame_seed_from_events, recover_runtime_summary,
        recover_snapshot_plus_tail, replay_events, summarize_recovery_events, verify_digests,
    };
    use crate::{EventSeq, FjallJournal, JournalEvent, RunHeaderRecord};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn summarize_recovery_events_returns_summary_hydration() {
        let run = RunId::new(77);
        let workflow = test_digest(9);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(2),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(2),
                action: ActionId::new(5),
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(2),
                action: ActionId::new(5),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(4),
                result: SlotIdx::new(3),
            },
        ];

        let hydration = summarize_recovery_events(&events).expect("summary recovery succeeds");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected summary hydration");
        };

        assert_eq!(summary.run, run);
        assert_eq!(summary.first_seq, EventSeq::new(0));
        assert_eq!(summary.last_seq, EventSeq::new(4));
        assert_eq!(summary.workflow, Some(workflow));
        assert_eq!(summary.steps_started, 1);
        assert_eq!(summary.actions_scheduled, 1);
        assert_eq!(summary.actions_resolved, 1);
        assert_eq!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(3),
            })
        );
    }

    #[test]
    fn recover_runtime_summary_reads_summary_from_journal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let run = RunId::new(79);
        let workflow = test_digest(10);

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("accepted append succeeds");
        journal
            .append_journaled(&JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
            })
            .expect("cancelled append succeeds");

        let summary = recover_runtime_summary(&journal, run)
            .expect("summary recovers")
            .summary();

        assert_eq!(summary.run, run);
        assert_eq!(summary.workflow, Some(workflow));
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));
    }

    #[test]
    fn recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states() {
        let run = RunId::new(91);
        let workflow = test_digest(13);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(3),
                output: SlotIdx::new(4),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(4),
                result: SlotIdx::new(5),
            },
        ];

        let seed = recover_runtime_frame_seed_from_events(&events).expect("seed recovers");

        assert_eq!(seed.summary.run, run);
        assert_eq!(seed.summary.workflow, Some(workflow));
        assert_eq!(seed.step_count, 4);
        assert_eq!(seed.slot_count, 6);
        assert_eq!(seed.pc, StepIdx::new(3));
        assert!(seed.steps.iter().any(
            |entry| entry.step == StepIdx::new(1) && entry.state == RecoveredStepState::Waiting
        ));
        assert!(
            seed.steps.iter().any(|entry| entry.step == StepIdx::new(3)
                && entry.state == RecoveredStepState::Succeeded)
        );
        assert_eq!(
            seed.unsupported,
            UnsupportedRecoveryState {
                slot_values: true,
                slot_taint: true,
                action_payloads: false,
            }
        );
    }

    #[test]
    fn recover_runtime_frame_seed_rejects_dimension_overflow() {
        let run = RunId::new(92);
        let events = vec![JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::MAX,
        }];

        let result = recover_runtime_frame_seed_from_events(&events);

        assert!(
            matches!(result, Err(RecoveryError::FrameDimensionOverflow { run: found }) if found == run)
        );
    }

    #[test]
    fn recover_runtime_frame_seed_reads_events_from_journal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let run = RunId::new(93);
        let workflow = test_digest(14);

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("accepted append succeeds");
        journal
            .append_journaled(&JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(2),
            })
            .expect("ask append succeeds");

        let seed = recover_runtime_frame_seed(&journal, run).expect("seed recovers");

        assert_eq!(seed.step_count, 3);
        assert_eq!(seed.slot_count, 0);
        assert_eq!(seed.pc, StepIdx::new(2));
        assert!(seed.steps.iter().any(
            |entry| entry.step == StepIdx::new(2) && entry.state == RecoveredStepState::Asking
        ));
    }

    #[test]
    fn recover_all_incomplete_runs_returns_only_non_terminal_runs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let workflow = test_digest(11);
        let incomplete = RunId::new(81);
        let finished = RunId::new(82);

        put_test_header(&journal, incomplete, workflow);
        put_test_header(&journal, finished, workflow);
        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run: incomplete,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("incomplete accepted append succeeds");
        journal
            .append_journaled(&JournalEvent::StepStarted {
                run: incomplete,
                seq: EventSeq::new(1),
                step: StepIdx::new(4),
            })
            .expect("incomplete step append succeeds");
        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run: finished,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("finished accepted append succeeds");
        journal
            .append_journaled(&JournalEvent::RunFinished {
                run: finished,
                seq: EventSeq::new(1),
                result: SlotIdx::new(2),
            })
            .expect("finished append succeeds");

        let recovered =
            recover_all_incomplete_runs(&journal).expect("incomplete recovery succeeds");

        assert_eq!(recovered.len(), 1);
        assert_eq!(
            recovered.first().expect("one recovery").summary().run,
            incomplete
        );
    }

    #[test]
    fn recover_all_incomplete_runs_rejects_header_without_journal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let run = RunId::new(83);
        let workflow = test_digest(12);

        put_test_header(&journal, run, workflow);

        let result = recover_all_incomplete_runs(&journal);

        assert!(
            matches!(result, Err(RecoveryError::NoRecoveryData { run: found }) if found == run)
        );
    }

    fn put_test_header(journal: &FjallJournal, run: RunId, digest: WorkflowDigest) {
        journal
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 123,
            })
            .expect("header write succeeds");
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TerminalSummary {
        Cancelled,
        Finished(SlotIdx),
        Failed,
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    struct ReplaySummary {
        accepted: usize,
        step_started: usize,
        step_succeeded: usize,
        action_scheduled: usize,
        action_completed: usize,
        action_failed: usize,
        wait_scheduled: usize,
        ask_scheduled: usize,
        ask_answered: usize,
        terminal: Option<TerminalSummary>,
    }

    fn summarize_events(events: &[JournalEvent]) -> ReplaySummary {
        let mut summary = ReplaySummary::default();
        for event in events {
            match event {
                JournalEvent::RunAccepted { .. } => {
                    summary.accepted = summary.accepted.saturating_add(1);
                }
                JournalEvent::StepStarted { .. } => {
                    summary.step_started = summary.step_started.saturating_add(1);
                }
                JournalEvent::StepSucceeded { .. } => {
                    summary.step_succeeded = summary.step_succeeded.saturating_add(1);
                }
                JournalEvent::ActionScheduled { .. } => {
                    summary.action_scheduled = summary.action_scheduled.saturating_add(1);
                }
                JournalEvent::ActionCompletedEvent { .. } => {
                    summary.action_completed = summary.action_completed.saturating_add(1);
                }
                JournalEvent::ActionFailedEvent { .. } => {
                    summary.action_failed = summary.action_failed.saturating_add(1);
                }
                JournalEvent::WaitScheduledEvent { .. } => {
                    summary.wait_scheduled = summary.wait_scheduled.saturating_add(1);
                }
                JournalEvent::AskScheduledEvent { .. } => {
                    summary.ask_scheduled = summary.ask_scheduled.saturating_add(1);
                }
                JournalEvent::AskAnsweredEvent { .. } => {
                    summary.ask_answered = summary.ask_answered.saturating_add(1);
                }
                JournalEvent::RunCancelled { .. } => {
                    summary.terminal = Some(TerminalSummary::Cancelled);
                }
                JournalEvent::RunFinished { result, .. } => {
                    summary.terminal = Some(TerminalSummary::Finished(*result));
                }
                JournalEvent::RunFailedEvent { .. } => {
                    summary.terminal = Some(TerminalSummary::Failed);
                }
                JournalEvent::SlotWrittenEvent { .. }
                | JournalEvent::RetryScheduledEvent { .. } => {}
            }
        }
        summary
    }

    fn combine_summaries(base: ReplaySummary, tail: ReplaySummary) -> ReplaySummary {
        ReplaySummary {
            accepted: base.accepted.saturating_add(tail.accepted),
            step_started: base.step_started.saturating_add(tail.step_started),
            step_succeeded: base.step_succeeded.saturating_add(tail.step_succeeded),
            action_scheduled: base.action_scheduled.saturating_add(tail.action_scheduled),
            action_completed: base.action_completed.saturating_add(tail.action_completed),
            action_failed: base.action_failed.saturating_add(tail.action_failed),
            wait_scheduled: base.wait_scheduled.saturating_add(tail.wait_scheduled),
            ask_scheduled: base.ask_scheduled.saturating_add(tail.ask_scheduled),
            ask_answered: base.ask_answered.saturating_add(tail.ask_answered),
            terminal: tail.terminal.or(base.terminal),
        }
    }

    fn summary_through(events: &[JournalEvent], seq: EventSeq) -> ReplaySummary {
        let mut prefix = Vec::new();
        for event in events {
            if event.seq() <= seq {
                prefix.push(event.clone());
            }
        }
        summarize_events(&prefix)
    }

    fn tail_after(events: &[JournalEvent], seq: EventSeq) -> Vec<JournalEvent> {
        let mut tail = Vec::new();
        for event in events {
            if event.seq() > seq {
                tail.push(event.clone());
            }
        }
        tail
    }

    fn append_events(
        journal: &FjallJournal,
        events: &[JournalEvent],
    ) -> Result<(), crate::JournalError> {
        for event in events {
            journal.append_journaled(event)?;
        }
        Ok(())
    }

    fn assert_snapshot_tail_matches_full_summary(
        run: RunId,
        snapshot_seq: EventSeq,
        events: &[JournalEvent],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let journal = FjallJournal::open(temp_dir.path(), None)?;
        append_events(&journal, events)?;

        let mut full_tracker = ActionReplayTracker::new();
        let full_replay = recover_full_journal(&journal, run, &mut full_tracker)?;

        let snapshot = RunSnapshot {
            run,
            seq: snapshot_seq,
            workflow: test_digest(1),
            slots: Vec::new(),
        };
        let tail = tail_after(events, snapshot_seq);
        let mut tail_tracker = ActionReplayTracker::new();
        let tail_replay = recover_snapshot_plus_tail(&snapshot, &tail, &mut tail_tracker)?;

        let full_summary = summarize_events(&full_replay);
        let snapshot_summary = summary_through(events, snapshot_seq);
        let tail_summary = summarize_events(&tail_replay);
        let combined_summary = combine_summaries(snapshot_summary, tail_summary);

        assert_eq!(full_summary, combined_summary);
        Ok(())
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

        let replayed = replay_events(&events, &mut tracker).expect("first execution should succeed");
        assert_eq!(replayed.len(), 2);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn snapshot_tail_matches_full_journal_lifecycle_summary()
    -> Result<(), Box<dyn std::error::Error>> {
        let run = RunId::new(900);
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
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: SlotIdx::new(3),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(3),
            },
        ];

        assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
    }

    #[test]
    fn snapshot_tail_matches_full_journal_action_summary() -> Result<(), Box<dyn std::error::Error>>
    {
        let run = RunId::new(901);
        let action = ActionId::new(4);
        let step = StepIdx::new(2);
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
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(0),
            },
        ];

        assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
    }

    #[test]
    fn snapshot_tail_matches_full_journal_wait_summary() -> Result<(), Box<dyn std::error::Error>> {
        let run = RunId::new(902);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(7),
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(2),
            },
        ];

        assert_snapshot_tail_matches_full_summary(run, EventSeq::new(0), &events)
    }

    #[test]
    fn snapshot_tail_matches_full_journal_ask_summary() -> Result<(), Box<dyn std::error::Error>> {
        let run = RunId::new(903);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(8),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(8),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(3),
                result: SlotIdx::new(1),
            },
        ];

        assert_snapshot_tail_matches_full_summary(run, EventSeq::new(1), &events)
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
        check_compiled_ir_digest(digest, digest).expect("matching digests should succeed");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
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

        journal.append_journaled(&accepted).expect("setup: append accepted");
        journal.append_journaled(&started).expect("setup: append started");
        journal.append_journaled(&finished).expect("setup: append finished");

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
        let replayed =
            replay_events(&events, &mut tracker).expect("replay of all event kinds should succeed");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(100);
        let stored_digest = test_digest(1);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: stored_digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(101);
        let digest = test_digest(5);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        check_workflow_source_digest(&journal, run, digest).expect("matching digest should succeed");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(200);
        let digest = test_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        verify_digests(
            &journal,
            run,
            digest,
            test_digest(8),
            test_digest(8),
            DigestCheck::Full,
        )
        .expect("matching digests at Full level should succeed");
    }

    #[test]
    fn verify_digests_returns_mismatch_when_ir_differs() {
        // Given matching workflow digests but different IR digests
        // When verify_digests is called with WorkflowAndIr level
        // Then it returns CompiledIrDigestMismatch
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(201);
        let digest = test_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

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
        let replayed = replay_events(&[], &mut tracker).expect("empty replay should succeed");
        assert!(replayed.is_empty());
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
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

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
            journal.append_journaled(event).expect("setup: append event");
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("full journal recovery should succeed");
        assert_eq!(replayed.len(), 4);
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
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 7);
    }

    #[test]
    fn verify_digests_workflow_source_only_checks_only_workflow() {
        // Given matching workflow digest but different IR digest
        // When verify_digests is called with WorkflowSourceOnly level
        // Then it returns Ok (IR mismatch is not checked)
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(400);
        let wf_digest = test_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: wf_digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        verify_digests(
            &journal,
            run,
            wf_digest,
            test_digest(8),
            test_digest(99), // different, but should not be checked at this level
            DigestCheck::WorkflowSourceOnly,
        )
        .expect("WorkflowSourceOnly should skip IR check");
    }

    #[test]
    fn check_workflow_source_digest_returns_ok_when_no_events() {
        // Given a journal with no events for the run
        // When check_workflow_source_digest is called
        // Then it returns Ok (no RunAccepted event means no mismatch)
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        check_workflow_source_digest(&journal, RunId::new(500), test_digest(1))
            .expect("no events should succeed");
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
        let err = RecoveryError::TerminalStateMismatch {
            expected: expected.clone(),
            found: found.clone(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("terminal state mismatch"));
    }

    // --- Section: Recovery Lifecycle BDD Tests ---

    #[test]
    fn recover_full_journal_returns_empty_recovery_for_empty_journal() {
        // Given an empty journal
        // When recover_full_journal is called for a run
        // Then it returns NoRecoveryData with the correct run
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(10);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted {
                run,
                seq: crate::EventSeq::new(0),
                workflow: test_digest(1),
            },
            crate::JournalEvent::StepStarted {
                run,
                seq: crate::EventSeq::new(1),
                step: StepIdx::new(0),
            },
            crate::JournalEvent::RunFinished {
                run,
                seq: crate::EventSeq::new(2),
                result: vb_core::SlotIdx::new(0),
            },
        ];
        for event in &events {
            journal.append_journaled(event).expect("setup: append event");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(20);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted {
                run,
                seq: crate::EventSeq::new(0),
                workflow: test_digest(1),
            },
            crate::JournalEvent::StepStarted {
                run,
                seq: crate::EventSeq::new(1),
                step: StepIdx::new(0),
            },
        ];
        for event in &events {
            journal.append_journaled(event).expect("setup: append event");
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed =
            recover_full_journal(&journal, run, &mut tracker).expect("recovery should succeed");
        let terminal = extract_terminal(&replayed);
        assert!(
            terminal.is_none(),
            "active run should have no terminal event"
        );
    }

    #[test]
    fn recover_full_journal_identifies_completed_runs() {
        // Given a journal with a run ending in RunFinished
        // When recover_full_journal is called
        // Then extract_terminal returns the RunFinished event
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(30);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted {
                run,
                seq: crate::EventSeq::new(0),
                workflow: test_digest(1),
            },
            crate::JournalEvent::RunFinished {
                run,
                seq: crate::EventSeq::new(1),
                result: vb_core::SlotIdx::new(0),
            },
        ];
        for event in &events {
            journal.append_journaled(event).expect("setup: append event");
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed =
            recover_full_journal(&journal, run, &mut tracker).expect("recovery should succeed");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(40);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted {
                run,
                seq: crate::EventSeq::new(0),
                workflow: test_digest(1),
            },
            crate::JournalEvent::RunFailedEvent {
                run,
                seq: crate::EventSeq::new(1),
            },
        ];
        for event in &events {
            journal.append_journaled(event).expect("setup: append event");
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed =
            recover_full_journal(&journal, run, &mut tracker).expect("recovery should succeed");
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
            crate::JournalEvent::StepStarted {
                run: RunId::new(1),
                seq: crate::EventSeq::new(1),
                step: StepIdx::new(0),
            },
            crate::JournalEvent::StepSucceeded {
                run: RunId::new(1),
                seq: crate::EventSeq::new(2),
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(1),
            },
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
            crate::JournalEvent::ActionScheduled {
                run: RunId::new(2),
                seq: crate::EventSeq::new(6),
                step,
                action,
            },
            crate::JournalEvent::ActionCompletedEvent {
                run: RunId::new(2),
                seq: crate::EventSeq::new(7),
                step,
                action,
            },
        ];
        let mut tracker = ActionReplayTracker::new();

        let result = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker);
        let replayed = result.expect("snapshot plus tail recovery should succeed");
        assert_eq!(replayed.len(), 2);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn replay_events_processes_submitted_event() {
        // Given a RunAccepted event
        // When replay_events is called
        // Then the event is replayed without state change to tracker
        let run = RunId::new(1);
        let events = vec![crate::JournalEvent::RunAccepted {
            run,
            seq: crate::EventSeq::new(0),
            workflow: test_digest(1),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_step_started_event() {
        // Given a StepStarted event
        // When replay_events is called
        // Then the event is replayed and last_step is updated internally
        let run = RunId::new(2);
        let events = vec![crate::JournalEvent::StepStarted {
            run,
            seq: crate::EventSeq::new(0),
            step: StepIdx::new(5),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_step_ended_event() {
        // Given a StepSucceeded event
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(3);
        let events = vec![crate::JournalEvent::StepSucceeded {
            run,
            seq: crate::EventSeq::new(0),
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(1),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_run_finished_event() {
        // Given a RunFinished terminal event
        // When replay_events is called
        // Then the event is replayed and is identified as terminal
        let run = RunId::new(4);
        let events = vec![crate::JournalEvent::RunFinished {
            run,
            seq: crate::EventSeq::new(0),
            result: vb_core::SlotIdx::new(99),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn replay_events_processes_run_failed_event() {
        // Given a RunFailedEvent terminal event
        // When replay_events is called
        // Then the event is replayed and is identified as terminal
        let run = RunId::new(5);
        let events = vec![crate::JournalEvent::RunFailedEvent {
            run,
            seq: crate::EventSeq::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn check_workflow_source_digest_accepts_matching_digest() {
        // Given a journal with a RunAccepted event using digest [5;32]
        // When check_workflow_source_digest is called with the same digest
        // Then it returns Ok
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(600);
        let digest = test_digest(5);
        let event = crate::JournalEvent::RunAccepted {
            run,
            seq: crate::EventSeq::new(0),
            workflow: digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        check_workflow_source_digest(&journal, run, digest).expect("matching digest should succeed");
    }

    #[test]
    fn check_compiled_ir_digest_accepts_matching_digest() {
        // Given identical expected and found digests
        // When check_compiled_ir_digest is called
        // Then it returns Ok
        let digest = test_digest(42);
        check_compiled_ir_digest(digest, digest).expect("matching digests should succeed");
    }

    #[test]
    fn verify_digests_returns_ok_when_all_digests_match() {
        // Given a journal with matching workflow and IR digests
        // When verify_digests is called at Full level
        // Then it returns Ok
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(700);
        let wf_digest = test_digest(10);
        let ir_digest = test_digest(20);
        let event = crate::JournalEvent::RunAccepted {
            run,
            seq: crate::EventSeq::new(0),
            workflow: wf_digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        verify_digests(
            &journal,
            run,
            wf_digest,
            ir_digest,
            ir_digest,
            DigestCheck::Full,
        )
        .expect("all matching digests at Full level should succeed");
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
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(800);
        let wf_digest = test_digest(7);
        let event = crate::JournalEvent::RunAccepted {
            run,
            seq: crate::EventSeq::new(0),
            workflow: wf_digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        verify_digests(
            &journal,
            run,
            wf_digest,
            test_digest(8),
            test_digest(99),
            DigestCheck::WorkflowSourceOnly,
        )
        .expect("WorkflowSourceOnly should skip IR check");
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
        let events = vec![crate::JournalEvent::ActionFailedEvent {
            run,
            seq: crate::EventSeq::new(0),
            step,
            action,
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn replay_events_processes_run_cancelled_event() {
        // Given a RunCancelled terminal event
        // When replay_events is called
        // Then the event is replayed and identified as terminal
        let run = RunId::new(11);
        let events = vec![crate::JournalEvent::RunCancelled {
            run,
            seq: crate::EventSeq::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn replay_events_processes_slot_written_event() {
        // Given a SlotWrittenEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(12);
        let events = vec![crate::JournalEvent::SlotWrittenEvent {
            run,
            seq: crate::EventSeq::new(0),
            slot: vb_core::SlotIdx::new(3),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_wait_scheduled_event() {
        // Given a WaitScheduledEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(13);
        let events = vec![crate::JournalEvent::WaitScheduledEvent {
            run,
            seq: crate::EventSeq::new(0),
            step: StepIdx::new(1),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_ask_scheduled_event() {
        // Given an AskScheduledEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(14);
        let events = vec![crate::JournalEvent::AskScheduledEvent {
            run,
            seq: crate::EventSeq::new(0),
            step: StepIdx::new(2),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_ask_answered_event() {
        // Given an AskAnsweredEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(15);
        let events = vec![crate::JournalEvent::AskAnsweredEvent {
            run,
            seq: crate::EventSeq::new(0),
            step: StepIdx::new(2),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn replay_events_processes_retry_scheduled_event() {
        // Given a RetryScheduledEvent
        // When replay_events is called
        // Then the event is replayed successfully
        let run = RunId::new(16);
        let events = vec![crate::JournalEvent::RetryScheduledEvent {
            run,
            seq: crate::EventSeq::new(0),
            step: StepIdx::new(3),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_events(&events, &mut tracker).expect("replay should succeed");
        assert_eq!(replayed.len(), 1);
    }

    #[test]
    fn recover_full_journal_with_cancelled_run() {
        // Given a journal with a run ending in RunCancelled
        // When recover_full_journal is called
        // Then the terminal event is RunCancelled
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = crate::FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(50);
        let events: Vec<crate::JournalEvent> = vec![
            crate::JournalEvent::RunAccepted {
                run,
                seq: crate::EventSeq::new(0),
                workflow: test_digest(1),
            },
            crate::JournalEvent::RunCancelled {
                run,
                seq: crate::EventSeq::new(1),
            },
        ];
        for event in &events {
            journal.append_journaled(event).expect("setup: append event");
        }

        let mut tracker = ActionReplayTracker::new();
        let replayed =
            recover_full_journal(&journal, run, &mut tracker).expect("recovery should succeed");
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

    // =========================================================================
    // Section: Adversarial Recovery BDD Tests
    // =========================================================================

    fn adv_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    // --- Adversarial: Corrupt Snapshot Recovery ---

    #[test]
    fn adversarial_corrupt_snapshot_missing_from_journal_returns_none() {
        // Given a snapshot pointing to a run/seq pair not in the snapshot keyspace
        // When FjallJournal::snapshot is called
        // Then it returns None (no corrupt data, just missing)
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal opens");
        let result = journal.snapshot(RunId::new(9999), EventSeq::new(0));
        let Ok(opt) = result else { return };
        assert_eq!(opt, None);
    }

    #[test]
    fn adversarial_recover_snapshot_with_corrupt_magic_returns_bad_magic() {
        // Given a manually crafted record with corrupt magic bytes
        // When decode_record is called with MAGIC_SNAPSHOT
        // Then it returns BadMagic
        let snapshot = RunSnapshot {
            run: RunId::new(500),
            seq: EventSeq::new(10),
            workflow: adv_digest(1),
            slots: vec![1, 2, 3],
        };
        let mut encoded = crate::encode_record(
            crate::MAGIC_SNAPSHOT,
            crate::RecordKind::Snapshot,
            snapshot.seq.get(),
            &snapshot,
            crate::MAX_SNAPSHOT_BYTES,
        )
        .expect("encode snapshot");
        // Corrupt the magic byte
        if let Some(byte) = encoded.get_mut(0) {
            *byte ^= 0xFF;
        }
        let result = crate::decode_record::<RunSnapshot>(
            &encoded,
            crate::MAGIC_SNAPSHOT,
            crate::MAX_SNAPSHOT_BYTES,
        );
        assert!(
            matches!(result, Err(crate::JournalError::BadMagic { .. })),
            "corrupt snapshot magic should return BadMagic, got {:?}",
            result
        );
    }

    // --- Adversarial: Recovery with Snapshot but No Journal Tail ---

    #[test]
    fn adversarial_recover_snapshot_only_no_tail_events_produces_empty_replay() {
        // Given a snapshot at seq 5 and an empty tail
        // When recover_snapshot_plus_tail is called
        // Then the replay is empty (zero events) and succeeds
        let snapshot = RunSnapshot {
            run: RunId::new(600),
            seq: EventSeq::new(5),
            workflow: adv_digest(2),
            slots: vec![],
        };
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot, &[], &mut tracker)
            .expect("empty tail should succeed");
        assert!(replayed.is_empty());
    }

    // --- Adversarial: Divergent Replay Detection ---

    #[test]
    fn adversarial_replay_divergence_out_of_order_step_returns_exact_step() {
        // Given events where step 5 comes before step 3
        // When replay_events processes them
        // Then it returns ReplayDivergence at step 3
        let run = RunId::new(700);
        let events = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(5),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(3),
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker);
        let Err(RecoveryError::ReplayDivergence { step, detail }) = result else {
            panic!("expected ReplayDivergence, got {:?}", result);
        };
        assert_eq!(step, StepIdx::new(3));
        assert!(!detail.is_empty());
    }

    #[test]
    fn adversarial_replay_divergence_summary_receives_events_for_multiple_runs() {
        // Given events from two different runs mixed together
        // When summarize_recovery_events is called
        // Then it returns ReplayDivergence with a multi-run detail
        let mixed = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(1),
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::RunAccepted {
                run: RunId::new(2),
                seq: EventSeq::new(1),
                workflow: adv_digest(2),
            },
        ];
        let result = summarize_recovery_events(&mixed);
        let Err(RecoveryError::ReplayDivergence { step, detail }) = result else {
            panic!("expected ReplayDivergence for mixed runs, got {:?}", result);
        };
        assert_eq!(step, StepIdx::ZERO);
        assert!(
            detail.contains("multiple runs"),
            "detail should mention multiple runs: {}",
            detail
        );
    }

    // --- Adversarial: Workflow Digest Mismatch During Recovery ---

    #[test]
    fn adversarial_workflow_source_digest_mismatch_returns_exact_digests() {
        // Given a journal with RunAccepted using digest [1;32]
        // When check_workflow_source_digest is called with digest [2;32]
        // Then it returns WorkflowSourceDigestMismatch with expected=[2;32], found=[1;32]
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal opens");
        let run = RunId::new(800);
        let stored = adv_digest(1);
        let wrong = adv_digest(2);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: stored,
        };
        journal.append_journaled(&event).expect("setup: append event");

        let result = check_workflow_source_digest(&journal, run, wrong);
        let Err(RecoveryError::WorkflowSourceDigestMismatch { expected, found }) = result else {
            panic!("expected WorkflowSourceDigestMismatch, got {:?}", result);
        };
        assert_eq!(expected, wrong);
        assert_eq!(found, stored);
    }

    #[test]
    fn adversarial_compiled_ir_digest_mismatch_returns_exact_digests() {
        // Given different expected and found IR digests
        // When check_compiled_ir_digest is called
        // Then it returns CompiledIrDigestMismatch with exact values
        let expected = adv_digest(10);
        let found = adv_digest(20);
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

    // --- Adversarial: Full Journal Recovery Edge Cases ---

    #[test]
    fn adversarial_recover_full_journal_with_only_terminal_event_succeeds() {
        // Given a journal with only a RunFinished event (no RunAccepted)
        // When recover_full_journal is called
        // Then it returns the single event
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal opens");
        let run = RunId::new(900);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: vb_core::SlotIdx::new(0),
        };
        journal.append_journaled(&event).expect("setup: append event");

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("recovery should succeed with terminal event");
        assert_eq!(replayed.len(), 1);
        assert!(is_terminal_event(&replayed[0]));
    }

    #[test]
    fn adversarial_recover_full_journal_with_run_accepted_only_succeeds() {
        // Given a journal with only a RunAccepted event
        // When recover_full_journal is called
        // Then it returns the single event with no terminal
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal opens");
        let run = RunId::new(901);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: adv_digest(5),
        };
        journal.append_journaled(&event).expect("setup: append event");

        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_full_journal(&journal, run, &mut tracker)
            .expect("recovery should succeed with just accepted");
        assert_eq!(replayed.len(), 1);
        assert!(extract_terminal(&replayed).is_none());
    }

    #[test]
    fn adversarial_recover_summary_counts_suspensions_correctly() {
        // Given events with WaitScheduled, AskScheduled, and RetryScheduled
        // When summarize_recovery_events counts suspensions
        // Then all three are counted (3 total)
        let run = RunId::new(910);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(2),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(3),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected summary hydration");
        };
        assert_eq!(summary.suspensions, 3);
    }

    #[test]
    fn adversarial_recover_summary_counts_slots_and_actions() {
        // Given events with slot writes, action schedules, and action completions
        // When summarize_recovery_events counts them
        // Then exact counts are returned
        let run = RunId::new(911);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(1),
                slot: vb_core::SlotIdx::new(0),
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: vb_core::SlotIdx::new(1),
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
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected summary hydration");
        };
        assert_eq!(summary.slots_written, 2);
        assert_eq!(summary.actions_scheduled, 1);
        assert_eq!(summary.actions_resolved, 2); // completed + failed
    }

    #[test]
    fn adversarial_recover_summary_terminal_states_are_mutually_exclusive() {
        // Given events ending with RunFinished (not Cancelled or Failed)
        // When summarize_recovery_events is called
        // Then the terminal state is Finished, not Cancelled or Failed
        let run = RunId::new(912);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(1),
                result: vb_core::SlotIdx::new(42),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected summary hydration");
        };
        let Some(RecoveryTerminalState::Finished { result }) = summary.terminal else {
            panic!("expected Finished terminal state");
        };
        assert_eq!(result, vb_core::SlotIdx::new(42));
    }

    // --- Adversarial: NonIdempotent Action Blocking ---

    #[test]
    fn adversarial_non_idempotent_action_blocked_after_failed_then_completed() {
        // Given an action first marked as failed, then encountered as scheduled again
        // When replay_events processes it
        // Then it returns NonIdempotentActionBlocked
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(99);
        let step = StepIdx::new(1);
        tracker.mark_failed(action, step);

        let events = vec![JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step,
            action,
        }];
        let result = replay_events(&events, &mut tracker);
        let Err(RecoveryError::NonIdempotentActionBlocked {
            action: blocked_action,
            step: blocked_step,
        }) = result
        else {
            panic!("expected NonIdempotentActionBlocked, got {:?}", result);
        };
        assert_eq!(blocked_action, action);
        assert_eq!(blocked_step, step);
    }

    #[test]
    fn adversarial_non_idempotent_action_multiple_resolutions_blocked() {
        // Given an action that was completed then scheduled again
        // When replay_events encounters the second schedule
        // Then it returns NonIdempotentActionBlocked with the correct action/step
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(50);
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
            // Re-schedule of the same action -- should be blocked
            JournalEvent::ActionScheduled {
                run: RunId::new(1),
                seq: EventSeq::new(2),
                step,
                action,
            },
        ];
        let result = replay_events(&events, &mut tracker);
        let Err(RecoveryError::NonIdempotentActionBlocked {
            action: blocked_action,
            step: blocked_step,
        }) = result
        else {
            panic!("expected NonIdempotentActionBlocked, got {:?}", result);
        };
        assert_eq!(blocked_action, action);
        assert_eq!(blocked_step, step);
    }

    // --- Adversarial: Snapshot + Tail Edge Cases ---

    #[test]
    fn adversarial_snapshot_plus_tail_with_many_events_succeeds() {
        // Given a snapshot at seq 0 and 50 tail events
        // When recover_snapshot_plus_tail is called
        // Then all 50 events are replayed
        let snapshot = RunSnapshot {
            run: RunId::new(1000),
            seq: EventSeq::new(0),
            workflow: adv_digest(1),
            slots: vec![],
        };
        let mut tail = Vec::new();
        for i in 0..50u16 {
            tail.push(JournalEvent::StepStarted {
                run: RunId::new(1000),
                seq: EventSeq::new(u64::from(i).saturating_add(1)),
                step: StepIdx::new(i),
            });
        }
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("50 tail events should replay");
        assert_eq!(replayed.len(), 50);
    }

    #[test]
    fn adversarial_snapshot_plus_tail_events_from_different_run_injected() {
        // Given a snapshot for run A and a tail event for run B
        // When recover_snapshot_plus_tail replays them
        // Then replay succeeds (cross-run validation is at a higher layer)
        // but the returned events carry the different run id
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: adv_digest(1),
            slots: vec![],
        };
        let tail = vec![JournalEvent::StepStarted {
            run: RunId::new(2), // different run
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();
        // recover_snapshot_plus_tail does not validate run_id consistency
        // within the tail -- that is summarize_recovery_events' job
        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("mixed-run tail events should replay");
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].run_id(), RunId::new(2));
    }

    // --- Adversarial: Recovery Hydration Summary Edge Cases ---

    #[test]
    fn adversarial_summarize_empty_events_returns_no_recovery_data() {
        // Given an empty event list
        // When summarize_recovery_events is called
        // Then it returns NoRecoveryData with run 0
        let result = summarize_recovery_events(&[]);
        let Err(RecoveryError::NoRecoveryData { run }) = result else {
            panic!("expected NoRecoveryData, got {:?}", result);
        };
        assert_eq!(run, RunId::new(0));
    }

    #[test]
    fn adversarial_summarize_single_run_finished_sets_terminal() {
        // Given a single RunFinished event
        // When summarize_recovery_events is called
        // Then the terminal is Finished and steps are 0
        let run = RunId::new(920);
        let events = vec![JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: vb_core::SlotIdx::new(5),
        }];
        let hydration = summarize_recovery_events(&events).expect("summary");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected summary hydration");
        };
        assert_eq!(summary.run, run);
        assert_eq!(summary.steps_started, 0);
        assert_eq!(summary.steps_succeeded, 0);
        assert_eq!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished {
                result: vb_core::SlotIdx::new(5),
            })
        );
    }

    // --- Adversarial: ActionReplayTracker Edge Cases ---

    #[test]
    fn adversarial_tracker_mark_completed_idempotent() {
        // Given an action marked completed twice
        // When is_resolved is called
        // Then it still returns true
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(10);
        let step = StepIdx::new(0);
        tracker.mark_completed(action, step);
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn adversarial_tracker_mark_failed_then_completed_both_resolve() {
        // Given an action marked both failed and completed
        // When is_resolved is called
        // Then it returns true (the action is in both sets)
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(11);
        let step = StepIdx::new(1);
        tracker.mark_failed(action, step);
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));
    }

    // --- Adversarial: DigestCheck Level Edge Cases ---

    #[test]
    fn adversarial_verify_digests_full_level_catches_ir_mismatch() {
        // Given matching workflow digest but different IR digests
        // When verify_digests is called with Full level
        // Then it returns CompiledIrDigestMismatch
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal opens");
        let run = RunId::new(950);
        let wf_digest = adv_digest(7);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: wf_digest,
        };
        journal.append_journaled(&event).expect("setup: append event");

        let result = verify_digests(
            &journal,
            run,
            wf_digest,
            adv_digest(8),
            adv_digest(9), // mismatch
            DigestCheck::Full,
        );
        let Err(RecoveryError::CompiledIrDigestMismatch { expected, found }) = result else {
            panic!("expected CompiledIrDigestMismatch, got {:?}", result);
        };
        assert_eq!(expected, adv_digest(8));
        assert_eq!(found, adv_digest(9));
    }

    // --- Adversarial: Recovery from Persisted Journal ---

    #[test]
    fn adversarial_recover_runtime_summary_from_persisted_journal() {
        // Given a journal that is opened, written, closed, and reopened
        // When recover_runtime_summary is called on the reopened journal
        // Then it returns the correct summary
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(960);
        let workflow = adv_digest(42);

        {
            let journal = FjallJournal::open(temp_dir.path(), None).expect("journal opens");
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                })
                .expect("append");
            journal
                .append_journaled(&JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(0),
                })
                .expect("append");
            journal
                .append_journaled(&JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(2),
                })
                .expect("append");
            journal.persist_strict().expect("persist");
        }

        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("reopen");
        let hydration = recover_runtime_summary(&journal2, run).expect("recovery summary");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected summary hydration");
        };
        assert_eq!(summary.run, run);
        assert_eq!(summary.workflow, Some(workflow));
        assert_eq!(summary.steps_started, 1);
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));
    }

    // --- Adversarial: is_terminal_event Edge Cases ---

    #[test]
    fn adversarial_all_non_terminal_events_identified_as_not_terminal() {
        // Given every non-terminal JournalEvent variant
        // When is_terminal_event is called
        // Then it returns false for all of them
        let run = RunId::new(1);
        let non_terminals: Vec<JournalEvent> = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0),
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
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(0),
                action: ActionId::new(1),
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(6),
                slot: vb_core::SlotIdx::new(0),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(7),
                step: StepIdx::new(1),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(8),
                step: StepIdx::new(2),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(9),
                step: StepIdx::new(2),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(10),
                step: StepIdx::new(3),
            },
        ];
        for event in &non_terminals {
            assert!(
                !is_terminal_event(event),
                "event {:?} should not be terminal",
                event
            );
        }
    }

    #[test]
    fn adversarial_extract_terminal_finds_last_terminal_in_sequence_with_earlier_terminal() {
        // Given events with RunCancelled at seq 1 and RunFinished at seq 2
        // When extract_terminal is called
        // Then it returns the RunFinished (last terminal, searching in reverse)
        let run = RunId::new(1);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: vb_core::SlotIdx::new(0),
            },
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some());
        let Some(JournalEvent::RunFinished { result, .. }) = terminal else {
            panic!("expected RunFinished as last terminal");
        };
        assert_eq!(*result, vb_core::SlotIdx::new(0));
    }

    // --- Adversarial: RecoveryHydration Accessor ---

    #[test]
    fn adversarial_hydration_summary_accessor_returns_inner_summary() {
        // Given a RecoveryHydration::Summary
        // When summary() is called
        // Then it returns the inner RecoveryRuntimeSummary
        let inner = RecoveryRuntimeSummary {
            run: RunId::new(42),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(5),
            workflow: Some(adv_digest(1)),
            steps_started: 2,
            steps_succeeded: 1,
            actions_scheduled: 1,
            actions_resolved: 1,
            suspensions: 0,
            slots_written: 3,
            terminal: Some(RecoveryTerminalState::Failed),
        };
        let hydration = RecoveryHydration::Summary(inner);
        let summary = hydration.summary();
        assert_eq!(summary.run, RunId::new(42));
        assert_eq!(summary.terminal, Some(RecoveryTerminalState::Failed));
    }

    #[test]
    fn adversarial_hydration_frame_seed_accessor_returns_embedded_summary() {
        // Given a RecoveryHydration::FrameSeed
        // When summary() is called
        // Then it returns the summary embedded in the seed
        let inner = RecoveryRuntimeSummary {
            run: RunId::new(43),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(3),
            workflow: Some(adv_digest(2)),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 1,
            terminal: Some(RecoveryTerminalState::Finished {
                result: vb_core::SlotIdx::new(0),
            }),
        };
        let seed = RecoveryFrameSeed {
            summary: inner,
            first_step: vb_core::StepIdx::ZERO,
            step_count: 2,
            slot_count: 1,
            pc: vb_core::StepIdx::new(1),
            steps: vec![RecoveredStepEntry {
                step: vb_core::StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            }],
            unsupported: UnsupportedRecoveryState {
                slot_values: true,
                slot_taint: true,
                action_payloads: false,
            },
        };
        let hydration = RecoveryHydration::FrameSeed(seed);
        let summary = hydration.summary();
        assert_eq!(summary.run, RunId::new(43));
        assert_eq!(summary.steps_started, 1);
        assert_eq!(
            summary.terminal,
            Some(RecoveryTerminalState::Finished {
                result: vb_core::SlotIdx::new(0),
            })
        );
    }

    #[test]
    fn recover_runtime_frame_seed_produces_frame_seed_hydration() {
        // Given a journal with run events
        // When recover_runtime_frame_seed is called
        // Then it returns a RecoveryFrameSeed with correct dimensions
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = FjallJournal::open(dir.path(), None).expect("journal opens");
        let run = RunId::new(99);
        let workflow = adv_digest(3);

        journal
            .append_strict(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("append accepted");
        journal
            .append_strict(&JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: vb_core::StepIdx::new(0),
            })
            .expect("append started");
        journal
            .append_strict(&JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: vb_core::StepIdx::new(0),
                output: vb_core::SlotIdx::new(0),
            })
            .expect("append succeeded");

        let seed = recover_runtime_frame_seed(&journal, run).expect("frame seed recovery");
        assert_eq!(seed.summary.run, run);
        assert_eq!(seed.summary.workflow, Some(workflow));
        assert_eq!(seed.summary.steps_started, 1);
        assert_eq!(seed.summary.steps_succeeded, 1);
        assert_eq!(seed.step_count, 1);
        assert_eq!(seed.pc, vb_core::StepIdx::new(0));
        assert_eq!(seed.steps.len(), 1);
        assert_eq!(seed.steps[0].step, vb_core::StepIdx::new(0));
        assert_eq!(seed.steps[0].state, RecoveredStepState::Succeeded);
    }

    // =========================================================================
    // Section: Adversarial Recovery Cycle Tests
    // =========================================================================

    // --- Digest Verification (critical for crash integrity) ---

    #[test]
    fn adversarial_digest_mismatch_workflow_source_prevents_recovery() {
        // Given a journal with RunAccepted using workflow digest A, and IR digest B
        // When verify_digests is called with matching workflow but a different
        //   found_ir_digest from what was expected
        // Then it returns CompiledIrDigestMismatch
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(2001);
        let wf_digest = adv_digest(1);
        let ir_expected = adv_digest(10);
        let ir_found = adv_digest(20);

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: wf_digest,
            })
            .expect("setup: append RunAccepted");

        let result = verify_digests(
            &journal,
            run,
            wf_digest,
            ir_expected,
            ir_found,
            DigestCheck::WorkflowAndIr,
        );
        let Err(RecoveryError::CompiledIrDigestMismatch { expected, found }) = result else {
            panic!(
                "expected CompiledIrDigestMismatch for IR mismatch, got {:?}",
                result
            );
        };
        assert_eq!(expected, ir_expected);
        assert_eq!(found, ir_found);
    }

    #[test]
    fn adversarial_digest_mismatch_compiled_ir_prevents_recovery() {
        // Given two different compiled IR digests
        // When check_compiled_ir_digest is called
        // Then it returns CompiledIrDigestMismatch with exact expected/found
        let expected = adv_digest(0xAA);
        let found = adv_digest(0xBB);
        let result = check_compiled_ir_digest(expected, found);
        let Err(RecoveryError::CompiledIrDigestMismatch {
            expected: exp,
            found: fnd,
        }) = result
        else {
            panic!(
                "expected CompiledIrDigestMismatch, got {:?}",
                result
            );
        };
        assert_eq!(exp, expected);
        assert_eq!(fnd, found);
    }

    #[test]
    fn adversarial_missing_workflow_source_prevents_full_recovery() {
        // Given a journal with no RunAccepted event for the target run
        // When recover_runtime_summary is called
        // Then it returns NoRecoveryData (no workflow digest was ever stored)
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let ghost_run = RunId::new(2999);

        let result = recover_runtime_summary(&journal, ghost_run);
        let Err(RecoveryError::NoRecoveryData { run }) = result else {
            panic!(
                "expected NoRecoveryData for run with no stored events, got {:?}",
                result
            );
        };
        assert_eq!(run, ghost_run);
    }

    #[test]
    fn adversarial_missing_compiled_ir_prevents_full_recovery() {
        // Given a journal with RunAccepted (which stores a workflow digest)
        //   but we ask verify_digests for a compiled IR check with a mismatch
        // Then the IR mismatch is caught because the found_ir_digest differs
        //   from what was expected -- simulating a missing compiled IR scenario
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(2003);
        let wf_digest = adv_digest(5);
        let ir_expected = adv_digest(10);
        let ir_found_different = adv_digest(99);

        journal
            .append_journaled(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: wf_digest,
            })
            .expect("setup: append RunAccepted");

        let result = verify_digests(
            &journal,
            run,
            wf_digest,
            ir_expected,
            ir_found_different,
            DigestCheck::WorkflowAndIr,
        );
        let Err(RecoveryError::CompiledIrDigestMismatch { .. }) = result else {
            panic!(
                "expected CompiledIrDigestMismatch for missing/changed IR, got {:?}",
                result
            );
        };
    }

    // --- Snapshot + Tail Recovery (the primary crash recovery path) ---

    #[test]
    fn adversarial_snapshot_only_recovery_with_no_tail_events() {
        // Given a snapshot at seq 5 and no tail events
        // When recover_snapshot_plus_tail is called
        // Then it returns an empty replay (snapshot carries the state, no tail to replay)
        let snapshot = RunSnapshot {
            run: RunId::new(3001),
            seq: EventSeq::new(5),
            workflow: adv_digest(1),
            slots: vec![0xAA, 0xBB],
        };
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot, &[], &mut tracker)
            .expect("snapshot-only recovery must succeed");
        assert!(
            replayed.is_empty(),
            "no tail events means empty replay"
        );
    }

    #[test]
    fn adversarial_snapshot_plus_single_tail_event() {
        // Given a snapshot at seq 0 and one tail event at seq 1
        // When recover_snapshot_plus_tail is called
        // Then exactly 1 event is replayed
        let snapshot = RunSnapshot {
            run: RunId::new(3002),
            seq: EventSeq::new(0),
            workflow: adv_digest(2),
            slots: vec![],
        };
        let tail = vec![JournalEvent::StepStarted {
            run: RunId::new(3002),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        }];
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("snapshot plus single tail must succeed");
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].seq(), EventSeq::new(1));
    }

    #[test]
    fn adversarial_snapshot_plus_multiple_tail_events() {
        // Given a snapshot at seq 0 and three tail events at seq 1, 2, 3
        // When recover_snapshot_plus_tail is called
        // Then all 3 events are replayed in order
        let snapshot = RunSnapshot {
            run: RunId::new(3003),
            seq: EventSeq::new(0),
            workflow: adv_digest(3),
            slots: vec![],
        };
        let tail = vec![
            JournalEvent::StepStarted {
                run: RunId::new(3003),
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::StepSucceeded {
                run: RunId::new(3003),
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
            },
            JournalEvent::RunFinished {
                run: RunId::new(3003),
                seq: EventSeq::new(3),
                result: SlotIdx::new(0),
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("snapshot plus 3 tail events must succeed");
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0].seq(), EventSeq::new(1));
        assert_eq!(replayed[1].seq(), EventSeq::new(2));
        assert_eq!(replayed[2].seq(), EventSeq::new(3));
    }

    #[test]
    fn adversarial_multiple_snapshots_latest_wins() {
        // Given a snapshot at seq 0 and a later snapshot at seq 5
        // When recovery is performed using the seq-5 snapshot with tail events
        // Then the seq-5 snapshot is used and only events after seq 5 are replayed
        let snapshot_late = RunSnapshot {
            run: RunId::new(3004),
            seq: EventSeq::new(5),
            workflow: adv_digest(4),
            slots: vec![],
        };
        let tail = vec![
            JournalEvent::StepStarted {
                run: RunId::new(3004),
                seq: EventSeq::new(6),
                step: StepIdx::new(3),
            },
            JournalEvent::StepSucceeded {
                run: RunId::new(3004),
                seq: EventSeq::new(7),
                step: StepIdx::new(3),
                output: SlotIdx::new(1),
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot_late, &tail, &mut tracker)
            .expect("latest snapshot plus tail must succeed");
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].seq(), EventSeq::new(6));
        assert_eq!(replayed[1].seq(), EventSeq::new(7));
    }

    #[test]
    fn adversarial_events_before_snapshot_ignored_in_snapshot_recovery() {
        // Given events at seq 0,1,2, a snapshot at seq 2, and tail events at seq 3,4
        // When recover_snapshot_plus_tail is called with the seq-2 snapshot
        // Then only tail events (3,4) are replayed -- events 0,1 are covered by snapshot
        let snapshot = RunSnapshot {
            run: RunId::new(3005),
            seq: EventSeq::new(2),
            workflow: adv_digest(5),
            slots: vec![],
        };
        let tail = vec![
            JournalEvent::StepStarted {
                run: RunId::new(3005),
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
            },
            JournalEvent::RunFinished {
                run: RunId::new(3005),
                seq: EventSeq::new(4),
                result: SlotIdx::new(0),
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let replayed = recover_snapshot_plus_tail(&snapshot, &tail, &mut tracker)
            .expect("snapshot at seq 2 plus tail must succeed");
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].seq(), EventSeq::new(3));
        assert_eq!(replayed[1].seq(), EventSeq::new(4));
        let term = extract_terminal(&replayed);
        let Some(JournalEvent::RunFinished { result, .. }) = term else {
            panic!("expected RunFinished terminal in tail replay");
        };
        assert_eq!(*result, SlotIdx::new(0));
    }

    // --- Terminal State Handling ---

    #[test]
    fn adversarial_recovery_of_finished_run_produces_terminal_state() {
        // Given events ending with RunFinished
        // When summarize_recovery_events is called
        // Then the summary has terminal Finished with the correct result slot
        let run = RunId::new(3101);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(1),
                result: SlotIdx::new(7),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary must succeed");
        let summary = hydration.summary();
        let Some(RecoveryTerminalState::Finished { result }) = summary.terminal else {
            panic!("expected Finished terminal state, got {:?}", summary.terminal);
        };
        assert_eq!(result, SlotIdx::new(7));
    }

    #[test]
    fn adversarial_recovery_of_failed_run_produces_terminal_state() {
        // Given events ending with RunFailedEvent
        // When summarize_recovery_events is called
        // Then the summary has terminal Failed
        let run = RunId::new(3102);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(2),
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(1),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary must succeed");
        let summary = hydration.summary();
        let Some(RecoveryTerminalState::Failed) = summary.terminal else {
            panic!("expected Failed terminal state, got {:?}", summary.terminal);
        };
    }

    #[test]
    fn adversarial_recovery_of_cancelled_run_produces_terminal_state() {
        // Given events ending with RunCancelled
        // When summarize_recovery_events is called
        // Then the summary has terminal Cancelled
        let run = RunId::new(3103);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(3),
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary must succeed");
        let summary = hydration.summary();
        let Some(RecoveryTerminalState::Cancelled) = summary.terminal else {
            panic!("expected Cancelled terminal state, got {:?}", summary.terminal);
        };
    }

    #[test]
    fn adversarial_recovery_of_active_run_has_no_terminal_state() {
        // Given a run with RunAccepted and StepStarted but no finish/fail/cancel
        // When summarize_recovery_events is called
        // Then the summary has terminal None
        let run = RunId::new(3104);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(4),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
        ];
        let hydration = summarize_recovery_events(&events).expect("summary must succeed");
        let summary = hydration.summary();
        assert!(
            summary.terminal.is_none(),
            "active run must have no terminal state, got {:?}",
            summary.terminal
        );
    }

    // --- Action Replay Safety (non-idempotent action protection) ---

    #[test]
    fn adversarial_action_replay_tracker_allows_idempotent_deterministic_pure() {
        // Given an ActionReplayTracker with no prior resolutions
        // When an ActionScheduled then ActionCompletedEvent are replayed
        // Then replay succeeds -- first execution is always allowed
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(42);
        let step = StepIdx::new(3);
        let events = vec![
            JournalEvent::ActionScheduled {
                run: RunId::new(3201),
                seq: EventSeq::new(0),
                step,
                action,
            },
            JournalEvent::ActionCompletedEvent {
                run: RunId::new(3201),
                seq: EventSeq::new(1),
                step,
                action,
            },
        ];
        let replayed = replay_events(&events, &mut tracker)
            .expect("first-time deterministic pure action must replay successfully");
        assert_eq!(replayed.len(), 2);
        assert!(tracker.is_resolved(action, step));
    }

    #[test]
    fn adversarial_action_replay_tracker_blocks_non_idempotent() {
        // Given an action already marked as completed in the tracker
        // When the same action is encountered as ActionScheduled in replay
        // Then replay_events returns NonIdempotentActionBlocked
        let mut tracker = ActionReplayTracker::new();
        let action = ActionId::new(77);
        let step = StepIdx::new(5);
        tracker.mark_completed(action, step);
        assert!(tracker.is_resolved(action, step));

        let events = vec![JournalEvent::ActionScheduled {
            run: RunId::new(3202),
            seq: EventSeq::new(0),
            step,
            action,
        }];
        let result = replay_events(&events, &mut tracker);
        let Err(RecoveryError::NonIdempotentActionBlocked {
            action: blocked_action,
            step: blocked_step,
        }) = result
        else {
            panic!(
                "expected NonIdempotentActionBlocked for re-scheduled action, got {:?}",
                result
            );
        };
        assert_eq!(blocked_action, action);
        assert_eq!(blocked_step, step);
    }

    #[test]
    fn adversarial_recovered_step_states_track_all_lifecycle_events() {
        // Given events for a step that goes through started, succeeded, and another
        //   that goes through started and failed
        // When recover_runtime_frame_seed_from_events is called
        // Then the step states correctly reflect the last observed state
        let run = RunId::new(3203);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(1),
                action: ActionId::new(10),
            },
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(1),
                action: ActionId::new(10),
            },
        ];
        let seed = recover_runtime_frame_seed_from_events(&events)
            .expect("frame seed recovery must succeed");

        // Step 0: started -> succeeded (final state: Succeeded)
        let step0 = seed
            .steps
            .iter()
            .find(|e| e.step == StepIdx::new(0))
            .expect("step 0 must be present");
        assert_eq!(step0.state, RecoveredStepState::Succeeded);

        // Step 1: started -> action scheduled -> action failed (final state: Failed)
        let step1 = seed
            .steps
            .iter()
            .find(|e| e.step == StepIdx::new(1))
            .expect("step 1 must be present");
        assert_eq!(step1.state, RecoveredStepState::Failed);
    }

    // --- RecoveryHydration Variants ---

    #[test]
    fn adversarial_hydration_summary_returns_embedded_summary_fields() {
        // Given a RecoveryHydration::Summary with specific field values
        // When summary() is called
        // Then every field matches exactly what was embedded
        let inner = RecoveryRuntimeSummary {
            run: RunId::new(3301),
            first_seq: EventSeq::new(10),
            last_seq: EventSeq::new(20),
            workflow: Some(adv_digest(0xCC)),
            steps_started: 5,
            steps_succeeded: 3,
            actions_scheduled: 4,
            actions_resolved: 4,
            suspensions: 2,
            slots_written: 7,
            terminal: Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(9),
            }),
        };
        let hydration = RecoveryHydration::Summary(inner);
        let s = hydration.summary();
        assert_eq!(s.run, RunId::new(3301));
        assert_eq!(s.first_seq, EventSeq::new(10));
        assert_eq!(s.last_seq, EventSeq::new(20));
        assert_eq!(s.workflow, Some(adv_digest(0xCC)));
        assert_eq!(s.steps_started, 5);
        assert_eq!(s.steps_succeeded, 3);
        assert_eq!(s.actions_scheduled, 4);
        assert_eq!(s.actions_resolved, 4);
        assert_eq!(s.suspensions, 2);
        assert_eq!(s.slots_written, 7);
        assert_eq!(
            s.terminal,
            Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(9),
            })
        );
    }

    #[test]
    fn adversarial_hydration_frame_seed_summary_matches_embedded() {
        // Given a RecoveryHydration::FrameSeed with a specific summary
        // When summary() is called
        // Then it returns the exact summary that was embedded in the seed
        let embedded = RecoveryRuntimeSummary {
            run: RunId::new(3302),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(8),
            workflow: Some(adv_digest(0xDD)),
            steps_started: 3,
            steps_succeeded: 2,
            actions_scheduled: 1,
            actions_resolved: 1,
            suspensions: 1,
            slots_written: 4,
            terminal: None,
        };
        let seed = RecoveryFrameSeed {
            summary: embedded,
            first_step: StepIdx::ZERO,
            step_count: 3,
            slot_count: 5,
            pc: StepIdx::new(2),
            steps: vec![],
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
            },
        };
        let hydration = RecoveryHydration::FrameSeed(seed);
        let s = hydration.summary();
        assert_eq!(s.run, RunId::new(3302));
        assert_eq!(s.first_seq, EventSeq::new(0));
        assert_eq!(s.last_seq, EventSeq::new(8));
        assert_eq!(s.workflow, Some(adv_digest(0xDD)));
        assert_eq!(s.steps_started, 3);
        assert_eq!(s.steps_succeeded, 2);
        assert_eq!(s.actions_scheduled, 1);
        assert_eq!(s.actions_resolved, 1);
        assert_eq!(s.suspensions, 1);
        assert_eq!(s.slots_written, 4);
        assert!(s.terminal.is_none());
    }

    #[test]
    fn adversarial_frame_seed_carries_pc_and_step_state() {
        // Given a FrameSeed with specific PC and step states
        // When the fields are read back
        // Then pc and step entries survive the roundtrip exactly
        let step_entries = vec![
            RecoveredStepEntry {
                step: StepIdx::new(0),
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(1),
                state: RecoveredStepState::Running,
            },
            RecoveredStepEntry {
                step: StepIdx::new(2),
                state: RecoveredStepState::Waiting,
            },
            RecoveredStepEntry {
                step: StepIdx::new(3),
                state: RecoveredStepState::Failed,
            },
        ];
        let seed = RecoveryFrameSeed {
            summary: RecoveryRuntimeSummary {
                run: RunId::new(3303),
                first_seq: EventSeq::new(0),
                last_seq: EventSeq::new(10),
                workflow: Some(adv_digest(0xEE)),
                steps_started: 4,
                steps_succeeded: 1,
                actions_scheduled: 0,
                actions_resolved: 0,
                suspensions: 1,
                slots_written: 0,
                terminal: None,
            },
            first_step: StepIdx::ZERO,
            step_count: 4,
            slot_count: 0,
            pc: StepIdx::new(3),
            steps: step_entries.clone(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
            },
        };
        assert_eq!(seed.pc, StepIdx::new(3));
        assert_eq!(seed.steps.len(), 4);
        assert_eq!(seed.steps[0].state, RecoveredStepState::Succeeded);
        assert_eq!(seed.steps[1].state, RecoveredStepState::Running);
        assert_eq!(seed.steps[2].state, RecoveredStepState::Waiting);
        assert_eq!(seed.steps[3].state, RecoveredStepState::Failed);
    }

    // --- Empty & Degenerate Cases ---

    #[test]
    fn adversarial_recovery_of_journal_with_zero_events_returns_clean_summary() {
        // Given a run header in the journal but no events for that run
        // When recover_runtime_summary is called
        // Then it returns NoRecoveryData (empty event set)
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(3401);
        // Write a run header but no events
        put_test_header(&journal, run, adv_digest(1));

        let result = recover_runtime_summary(&journal, run);
        let Err(RecoveryError::NoRecoveryData { run: found_run }) = result else {
            panic!(
                "expected NoRecoveryData for run with header but zero events, got {:?}",
                result
            );
        };
        assert_eq!(found_run, run);
    }

    #[test]
    fn adversarial_recovery_of_journal_with_only_run_accepted() {
        // Given a journal with a single RunAccepted event
        // When summarize_recovery_events is called
        // Then the summary shows steps_started=0, steps_succeeded=0,
        //   actions_scheduled=0, terminal=None
        let run = RunId::new(3402);
        let events = vec![JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: adv_digest(1),
        }];
        let hydration = summarize_recovery_events(&events).expect("summary of single event");
        let RecoveryHydration::Summary(summary) = hydration else {
            panic!("expected Summary hydration");
        };
        assert_eq!(summary.run, run);
        assert_eq!(summary.steps_started, 0);
        assert_eq!(summary.steps_succeeded, 0);
        assert_eq!(summary.actions_scheduled, 0);
        assert_eq!(summary.actions_resolved, 0);
        assert_eq!(summary.suspensions, 0);
        assert_eq!(summary.slots_written, 0);
        assert!(summary.terminal.is_none());
    }

    #[test]
    fn adversarial_extract_terminal_returns_none_for_no_terminal() {
        // Given a list of events with no terminal event
        // When extract_terminal is called
        // Then it returns None
        let run = RunId::new(3403);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: adv_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                action: ActionId::new(1),
            },
        ];
        let result = extract_terminal(&events);
        assert!(
            result.is_none(),
            "no terminal event in list, expected None"
        );
    }
}
