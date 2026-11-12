#![forbid(unsafe_code)]

mod admission;
mod arena;
mod boundary;
mod full_ask_schedule;
mod full_wait_schedule;
mod pending;
mod product;
mod suspended;

use arena::recovered_arena_handles_resolve;
use suspended::{
    recoverable_open_ask_boundary, recoverable_timed_ask_boundary,
    recoverable_waiting_timer_boundary,
};

use vb_core::CompiledWorkflow;
use vb_core::action::ActionTicket;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{RecoveredStepState, RecoveryFrameSeed};

pub(crate) use admission::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, action_abi_digests_from_contracts,
    action_contract_abi_digest, validate_recovered_admission_evidence,
};
pub use boundary::{
    RecoveredOpenAsk, RecoveredRunBoundary, RecoveredRunBoundaryKind, RecoveredTimerSuspension,
};
pub(crate) use pending::pending_action_ticket_from_events;
pub(crate) use product::ResumableRecoveryParts;
pub use product::{
    NonResumableRecoveryProduct, ResumableRecoveryProduct, RuntimeRecoveryProduct,
    SummaryRecoveryProduct,
};

use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveredPendingActionTicket {
    event_seq: vb_storage::EventSeq,
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
    action_abi_digest: WorkflowDigest,
}

impl RecoveredPendingActionTicket {
    #[must_use]
    pub(crate) const fn new(
        event_seq: vb_storage::EventSeq,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    ) -> Self {
        Self {
            event_seq,
            ticket,
            input,
            output,
            action_abi_digest,
        }
    }

    #[must_use]
    pub(crate) const fn ticket(self) -> ActionTicket {
        self.ticket
    }

    #[must_use]
    pub(crate) const fn event_seq(self) -> vb_storage::EventSeq {
        self.event_seq
    }
}

/// Hydrates a frame for the full runtime recovery path.
///
/// Unlike `RuntimeRecoveryBoundary::hydrate_run_frame`, this path is allowed
/// to proceed when a pending action has durable ticket authority and the
/// caller will hydrate the surrounding `RunState` components from artifacts.
/// Hydrates a frame for the full runtime recovery path with initial input seeds.
///
/// Retained inputs are seeded into the frame BEFORE applying journal slot writes
/// from the seed. Slots written by journal events after the initial seeding
/// retain their values (later writes win). This ensures journal replay
/// determinism while accepting retained typed inputs at cold startup.
pub(crate) fn hydrate_frame_for_full_recovery_with_inputs(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
    initial_inputs: Vec<(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)>,
    store: Option<&vb_core::ValueStore>,
) -> RuntimeResult<RunFrame> {
    super::frame::validate_recovery_seed_shape(seed)?;
    reject_unrecoverable_full_recovery_state(seed, boundary, &initial_inputs, store)?;
    super::frame::hydrate_shape_checked_run_frame_with_inputs(seed, initial_inputs)
}

pub(crate) fn recovered_run_boundary_from_seed(
    seed: &RecoveryFrameSeed,
    pending_action: Option<RecoveredPendingActionTicket>,
    workflow: &CompiledWorkflow,
    events: &[vb_storage::JournalEvent],
) -> RecoveredRunBoundary {
    if let Some(ticket) = pending_action {
        return RecoveredRunBoundary::from_pending_action(ticket);
    }
    if let Some(ask) = recoverable_open_ask_boundary(seed, workflow, events) {
        return RecoveredRunBoundary::from_open_ask(ask);
    }
    if let Some(timer) = recoverable_waiting_timer_boundary(seed, workflow, events) {
        return RecoveredRunBoundary::from_waiting_timer(timer);
    }
    match recoverable_timed_ask_boundary(seed, workflow, events) {
        Some(ask) => RecoveredRunBoundary::from_timed_ask(ask),
        None => RecoveredRunBoundary::none(),
    }
}

pub(crate) fn classify_full_recovery_resume(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
    initial_inputs: &[(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)],
    store: Option<&vb_core::ValueStore>,
) -> vb_storage::recovery::RecoveryCannotResumeState {
    let mut state = initial_full_recovery_resume_state(seed, boundary, initial_inputs, store);
    mark_pending_action_range(seed, boundary, &mut state);
    mark_step_recovery_boundaries(seed, boundary, &mut state);
    state
}

/// Determines whether the run's latest terminal evidence belongs to its current attempt.
///
/// The `RetryCheckpoint` event's `attempt` field names the *authorized next* attempt (not a
/// started attempt; see `transitions.rs::capture_retry_checkpoint`), so it is excluded when
/// computing the current attempt. This prevents a failed run (`RunFailedEvent { attempt: N }`
/// followed by `RetryCheckpoint { attempt: N + 1 }`, with nothing further) from being misread
/// as "active" by the checkpoint's future attempt number: the failed run must remain terminal
/// until an explicit `retry`, while a run that was explicitly retried and is now genuinely
/// active (e.g. `StepStarted { attempt: N + 1 }`) is correctly treated as resumable.
#[must_use]
pub(crate) fn run_is_currently_terminal(events: &[vb_storage::JournalEvent]) -> bool {
    let mut current_attempt = 1u16;
    for event in events {
        if matches!(event, vb_storage::JournalEvent::RetryCheckpoint { .. }) {
            continue;
        }
        if let Some(attempt) = event.attempt()
            && attempt > current_attempt
        {
            current_attempt = attempt;
        }
    }
    events.iter().rev().any(|event| {
        vb_storage::recovery::is_terminal_event(event)
            && event.attempt().unwrap_or(1) == current_attempt
    })
}

fn initial_full_recovery_resume_state(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
    initial_inputs: &[(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)],
    store: Option<&vb_core::ValueStore>,
) -> vb_storage::recovery::RecoveryCannotResumeState {
    let mut state = vb_storage::recovery::RecoveryCannotResumeState {
        slot_values: seed.unsupported.slot_values,
        slot_taint: seed.unsupported.slot_taint,
        action_payloads: seed.unsupported.action_payloads,
        pending_actions: !seed.pending_actions.is_empty()
            && boundary.kind() != RecoveredRunBoundaryKind::PendingAction,
        ..vb_storage::recovery::RecoveryCannotResumeState::RESUMABLE
    };
    state.store_missing =
        state.store_missing || !recovered_arena_handles_resolve(seed, initial_inputs, store);
    state
}

fn mark_pending_action_range(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
    state: &mut vb_storage::recovery::RecoveryCannotResumeState,
) {
    if let Some(evidence) = boundary.pending_action_ticket()
        && (evidence.event_seq() < seed.summary.first_seq
            || evidence.event_seq() > seed.summary.last_seq)
    {
        state.pending_actions = true;
    }
}

fn mark_step_recovery_boundaries(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
    state: &mut vb_storage::recovery::RecoveryCannotResumeState,
) {
    for entry in &seed.steps {
        mark_step_recovery_boundary(entry.state, entry.step, boundary, state);
    }
}

fn mark_step_recovery_boundary(
    recovered: RecoveredStepState,
    step: StepIdx,
    boundary: RecoveredRunBoundary,
    state: &mut vb_storage::recovery::RecoveryCannotResumeState,
) {
    match recovered {
        RecoveredStepState::Waiting if !boundary.is_waiting_timer_step(step) => {
            state.pending_timers = true;
        }
        RecoveredStepState::Asking
            if !boundary.is_open_ask_step(step) && !boundary.is_timed_ask_step(step) =>
        {
            state.pending_asks = true;
        }
        RecoveredStepState::Waiting
        | RecoveredStepState::Asking
        | RecoveredStepState::Running
        | RecoveredStepState::Succeeded
        | RecoveredStepState::Failed => {}
        _ => {
            state.pending_asks = true;
        }
    }
}

fn reject_unrecoverable_full_recovery_state(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
    initial_inputs: &[(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)],
    store: Option<&vb_core::ValueStore>,
) -> RuntimeResult<()> {
    let state = classify_full_recovery_resume(seed, boundary, initial_inputs, store);
    if state.is_resumable() {
        Ok(())
    } else {
        Err(cannot_resume_error(state.unsupported_reason()))
    }
}

fn cannot_resume_error(reason: &'static str) -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from(reason),
    }
}
