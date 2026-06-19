#![forbid(unsafe_code)]
//! Frame seed accumulator, envelope view, and frame seed recovery.
//!
//! Provides:
//! - `RecoveryFrameSeedBuilder` — compatibility adapter for incremental recovery
//! - `recover_runtime_frame_seed_from_events` — public frame seed recovery
//! - `reject_workflow_digest_mismatch` — digest validation
//!
//! Internal:
//! - `accumulator` — FrameSeedAccumulator, ActionEnvelopeView, state machine

mod accumulator;
mod action_records;

use std::collections::HashMap;

pub(crate) use accumulator::FrameSeedAccumulator;

use crate::recovery::{
    RecoveryError, RecoveryFrameSeed, RecoveryResult, RecoveryRuntimeSummary,
    UnsupportedRecoveryState,
};
use crate::{EventSeq, JournalEvent};
use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

// ── RecoveryFrameSeedBuilder ────────────────────────────────────────────────

/// Builder that constructs a [`RecoveryFrameSeed`] from journal events.
///
/// This type is intentionally retained as a tiny compatibility adapter for
/// callers that configure recovery incrementally. It owns no recovery logic;
/// all behavior delegates to the direct public functions below.
pub struct RecoveryFrameSeedBuilder<'a> {
    workflow: Option<&'a vb_core::CompiledWorkflow>,
}

impl<'a> RecoveryFrameSeedBuilder<'a> {
    /// Creates a frame seed builder without compiled workflow replay support.
    #[must_use]
    pub const fn new() -> Self {
        Self { workflow: None }
    }

    /// Adds a compiled workflow used to reconstruct deterministic slot values.
    #[must_use]
    pub const fn with_workflow(mut self, workflow: &'a vb_core::CompiledWorkflow) -> Self {
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

// ── Public frame seed recovery ──────────────────────────────────────────────

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
    workflow: &vb_core::CompiledWorkflow,
) -> RecoveryResult<RecoveryFrameSeed> {
    reject_workflow_digest_mismatch(events, workflow.digest())?;
    recover_runtime_frame_seed_from_events_inner(events, Some(workflow))
}

/// Validates that the workflow digest in the first accepted run event matches
/// the expected compiled workflow digest.
pub fn reject_workflow_digest_mismatch(
    events: &[JournalEvent],
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAccepted { workflow, .. } if *workflow != expected => {
                Some(Err(RecoveryError::CompiledIrDigestMismatch {
                    expected,
                    found: *workflow,
                }))
            }
            JournalEvent::RunAccepted { .. } => Some(Ok(())),
            _ => None,
        })
        .map_or(Ok(()), |result| result)
}

// ── Frame seed construction helpers ─────────────────────────────────────────

fn recover_runtime_frame_seed_from_events_inner(
    events: &[JournalEvent],
    workflow: Option<&vb_core::CompiledWorkflow>,
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
    workflow: Option<&vb_core::CompiledWorkflow>,
) -> RecoveryResult<RecoveryFrameSeed> {
    let run = accumulator.run;
    let step_count = dimension_count(accumulator.max_step_idx, run)?;
    let slot_count = dimension_count(accumulator.max_slot_idx, run)?;
    let first_step = accumulator.first_step();
    let slots = crate::recovery::replay::summary::slots::recover_slots(&accumulator, workflow)?;
    let unsupported = seed_unsupported_state(&accumulator, &slots);
    let steps = recovered_steps(accumulator.step_states);

    Ok(RecoveryFrameSeed {
        summary: accumulator.summary,
        first_step,
        step_count,
        slot_count,
        pc: accumulator.pc,
        steps,
        slots: slots.entries,
        unsupported,
    })
}

fn seed_unsupported_state(
    accumulator: &FrameSeedAccumulator,
    slots: &crate::recovery::replay::summary::slots::RecoveredSlots,
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
        if accumulator.envelope_event_seen {
            // Ticket envelopes carry action payload bodies that the current runtime
            // rehydration boundary cannot re-attach to a live frame, so the seed
            // must explicitly mark these as unsupported.
            UnsupportedRecoveryState::action_payloads_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        },
    ]
    .into_iter()
    .fold(
        UnsupportedRecoveryState::SUPPORTED,
        UnsupportedRecoveryState::union,
    )
}

// ── Dimension helpers ───────────────────────────────────────────────────────

fn max_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.max(candidate)))
}

fn min_step(current: Option<StepIdx>, candidate: StepIdx) -> Option<StepIdx> {
    current.map_or(Some(candidate), |step| Some(step.min(candidate)))
}

fn max_slot(current: Option<SlotIdx>, candidate: SlotIdx) -> Option<SlotIdx> {
    current.map_or(Some(candidate), |slot| Some(slot.max(candidate)))
}

pub(super) trait RecoveryIndex {
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

pub(super) fn dimension_count<T: RecoveryIndex>(max: Option<T>, run: RunId) -> RecoveryResult<u16> {
    max.map(|value| {
        value
            .index()
            .checked_add(1)
            .ok_or(RecoveryError::FrameDimensionOverflow { run })
    })
    .map_or(Ok(0), |result| result)
}

// ── Public proof surfaces ───────────────────────────────────────────────────

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

// ── Steps conversion ────────────────────────────────────────────────────────

fn recovered_steps(
    step_states: HashMap<StepIdx, crate::recovery::RecoveredStepState>,
) -> Vec<crate::recovery::RecoveredStepEntry> {
    step_states
        .into_iter()
        .map(|(step, state)| crate::recovery::RecoveredStepEntry { step, state })
        .collect()
}
