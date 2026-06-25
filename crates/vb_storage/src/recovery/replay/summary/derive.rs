#![forbid(unsafe_code)]
//! Frame-seed assembly: turn an accumulator into a [`RecoveryFrameSeed`].
//!
//! [`RecoveryFrameSeedBuilder`] is a thin compatibility adapter that
//! delegates to the direct `recover_runtime_frame_seed_from_events*`
//! functions exposed below. Also provides:
//! - `recover_run_admission_from_events`: latest admission metadata
//! - `dimension_count` and the production proof surface
//!   (`recovery_dimension_count_from_index`, `recovery_seed_dimensions_positive`,
//!   `recovery_observed_dimension_is_positive`).

use std::collections::{HashMap, HashSet};

use vb_core::{ActionId, CompiledWorkflow, RunId, SlotIdx, StepIdx, WorkflowDigest};

use crate::recovery::types::{
    RecoveredPendingAction, RecoveredRunAdmission, RecoveredStepEntry, RecoveredStepState,
    RecoveryError, RecoveryFrameSeed, RecoveryResult, UnsupportedRecoveryState,
};
use crate::JournalEvent;

use super::accumulator::FrameSeedAccumulator;
use super::hydrate::RecoveredSlots;

/// Builder that constructs a [`RecoveryFrameSeed`] from journal events.
///
/// This type is intentionally retained as a tiny compatibility adapter for
/// callers that configure recovery incrementally. It owns no recovery logic;
/// all behavior delegates to the direct public functions below.
pub struct RecoveryFrameSeedBuilder<'a> {
    workflow: Option<&'a CompiledWorkflow>,
}

impl<'a> RecoveryFrameSeedBuilder<'a> {
    /// Creates a frame seed builder without compiled workflow replay support.
    #[must_use]
    pub const fn new() -> Self {
        Self { workflow: None }
    }

    /// Adds a compiled workflow used to reconstruct deterministic slot values.
    #[must_use]
    pub const fn with_workflow(mut self, workflow: &'a CompiledWorkflow) -> Self {
        self.workflow = Some(workflow);
        self
    }

    /// Build a frame seed from a pre-collected event slice.
    pub fn build(&self, events: &[JournalEvent]) -> RecoveryResult<RecoveryFrameSeed> {
        match self.workflow {
            Some(workflow) => {
                recover_runtime_frame_seed_from_events_with_workflow(events, workflow)
            }
            None => recover_runtime_frame_seed_from_events(events),
        }
    }
}

impl Default for RecoveryFrameSeedBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovers a [`RecoveryFrameSeed`] from ordered journal events.
///
/// Reconstructs step states, dimensions, and program counter from the
/// durable event sequence.
pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    recover_runtime_frame_seed_from_events_inner(events, None)
}

/// Recovers a [`RecoveryFrameSeed`] and reconstructs deterministic slot state
/// from a compiled workflow.
pub fn recover_runtime_frame_seed_from_events_with_workflow(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
) -> RecoveryResult<RecoveryFrameSeed> {
    reject_workflow_digest_mismatch(events, workflow.digest())?;
    recover_runtime_frame_seed_from_events_inner(events, Some(workflow))
}

/// Recovers the latest admission metadata from ordered journal events.
#[must_use]
pub fn recover_run_admission_from_events(events: &[JournalEvent]) -> Option<RecoveredRunAdmission> {
    events.iter().rev().find_map(|event| match event {
        JournalEvent::RunAdmission {
            run,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => Some(RecoveredRunAdmission {
            artifact_digest: *artifact_digest,
            run_id: *run,
            granted_capabilities: granted_capabilities.clone(),
            policy: *policy,
        }),
        _ => None,
    })
}

/// Fail-closed gate for the compiled workflow digest used by recovery.
///
/// Preconditions:
/// - `events` is the ordered slice passed to the surrounding recovery routine.
/// - `expected` is the compiled workflow digest for the run being recovered.
///
/// Postconditions:
/// - Returns `Ok(())` only when at least one `JournalEvent::RunAccepted` is
///   present in `events` and its digest equals `expected`.
/// - Returns `Err(RecoveryError::CompiledIrDigestMismatch { .. })` when a
///   `RunAccepted` event is present but its digest differs from `expected`.
/// - Returns `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO,
///   detail: "RunAccepted evidence missing" })` when `events` is empty or
///   contains no `RunAccepted` event, so the gate fails closed on missing
///   evidence rather than silently passing.
pub(crate) fn reject_workflow_digest_mismatch(
    events: &[JournalEvent],
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    for event in events {
        if let JournalEvent::RunAccepted { workflow, .. } = event {
            return if *workflow == expected {
                Ok(())
            } else {
                Err(RecoveryError::CompiledIrDigestMismatch {
                    expected,
                    found: *workflow,
                })
            };
        }
    }
    Err(RecoveryError::ReplayDivergence {
        step: StepIdx::ZERO,
        detail: String::from("RunAccepted evidence missing"),
    })
}

fn recover_runtime_frame_seed_from_events_inner(
    events: &[JournalEvent],
    workflow: Option<&CompiledWorkflow>,
) -> RecoveryResult<RecoveryFrameSeed> {
    let first = events
        .first()
        .ok_or(RecoveryError::NoRecoveryData { run: RunId::new(0) })?;
    let run = first.run_id();
    let accumulator = recover_frame_seed_accumulator(events, run, first.seq())?;
    build_recovery_frame_seed(accumulator, workflow)
}

fn recover_frame_seed_accumulator(
    events: &[JournalEvent],
    run: RunId,
    first_seq: crate::EventSeq,
) -> RecoveryResult<FrameSeedAccumulator> {
    events.iter().try_fold(
        FrameSeedAccumulator::new(run, first_seq),
        |accumulator, event| accumulator.apply(event),
    )
}

fn build_recovery_frame_seed(
    accumulator: FrameSeedAccumulator,
    workflow: Option<&CompiledWorkflow>,
) -> RecoveryResult<RecoveryFrameSeed> {
    let run = accumulator.run;
    let step_count = dimension_count(accumulator.max_step_idx, run)?;
    let slot_count = dimension_count(accumulator.max_slot_idx, run)?;
    let first_step = accumulator.first_step();
    let slots = super::hydrate::recover_slots(&accumulator, workflow)?;
    let unsupported = seed_unsupported_state(&accumulator, &slots);
    let steps = recovered_steps(accumulator.step_states);
    let pending_actions = recovered_pending_actions(accumulator.pending_actions);

    Ok(RecoveryFrameSeed {
        summary: accumulator.summary,
        first_step,
        step_count,
        slot_count,
        pc: accumulator.pc,
        steps,
        slots: slots.entries,
        pending_actions,
        unsupported,
    })
}

fn seed_unsupported_state(
    accumulator: &FrameSeedAccumulator,
    slots: &RecoveredSlots,
) -> UnsupportedRecoveryState {
    let slot_evidence_seen =
        accumulator.summary.slots_written > 0 || accumulator.summary.steps_succeeded > 0;
    let slot_values_unsupported =
        accumulator.missing_slot_values || (slot_evidence_seen && !slots.fully_supported);
    [
        if slot_values_unsupported {
            UnsupportedRecoveryState::slot_values_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
        if accumulator.event_slot_taint_unsupported {
            UnsupportedRecoveryState::event_slot_taint_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
        if accumulator.pending_actions.is_empty() {
            UnsupportedRecoveryState::SUPPORTED
        } else {
            UnsupportedRecoveryState::pending_actions_unsupported()
        },
    ]
    .into_iter()
    .fold(
        UnsupportedRecoveryState::SUPPORTED,
        UnsupportedRecoveryState::union,
    )
}

trait RecoveryIndex {
    fn index(self) -> u16;
}

impl RecoveryIndex for StepIdx {
    fn index(self) -> u16 {
        self.get()
    }
}

impl RecoveryIndex for SlotIdx {
    fn index(self) -> u16 {
        self.get()
    }
}

fn dimension_count<T: RecoveryIndex>(max: Option<T>, run: RunId) -> RecoveryResult<u16> {
    max.map(|value| {
        value
            .index()
            .checked_add(1)
            .ok_or(RecoveryError::FrameDimensionOverflow { run })
    })
    .map_or(Ok(0), |result| result)
}

/// Production proof surface for turning a maximum zero-based dimension into a count.
pub fn recovery_dimension_count_from_index(
    max_index: Option<u16>,
    run: RunId,
) -> RecoveryResult<u16> {
    max_index
        .map(|value| {
            value
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .map_or(Ok(0), |result| result)
}

/// Production proof surface for successful non-empty/evidence-bearing seed dimensions.
#[must_use]
pub const fn recovery_seed_dimensions_positive(seed: &RecoveryFrameSeed) -> bool {
    seed.step_count > 0 && seed.slot_count > 0
}

/// Production proof surface for an observed dimension requiring positive count.
#[must_use]
pub const fn recovery_observed_dimension_is_positive(max_index: Option<u16>, count: u16) -> bool {
    match max_index {
        Some(_) => count > 0,
        None => count == 0,
    }
}

fn recovered_steps(step_states: HashMap<StepIdx, RecoveredStepState>) -> Vec<RecoveredStepEntry> {
    step_states
        .into_iter()
        .map(|(step, state)| RecoveredStepEntry { step, state })
        .collect()
}

fn recovered_pending_actions(
    pending_actions: HashSet<(ActionId, StepIdx)>,
) -> Vec<RecoveredPendingAction> {
    pending_actions
        .into_iter()
        .map(|(action, step)| RecoveredPendingAction { step, action })
        .collect()
}
