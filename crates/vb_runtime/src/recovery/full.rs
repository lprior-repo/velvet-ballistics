#![forbid(unsafe_code)]

mod admission;
mod pending;
mod product;

use vb_core::action::ActionTicket;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::SlotValue;
use vb_core::{CompiledNodeKind, CompiledWorkflow};
use vb_storage::recovery::{RecoveredStepState, RecoveryFrameSeed};

pub(crate) use admission::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, action_abi_digests_from_contracts,
    validate_recovered_admission_evidence,
};
pub(crate) use pending::pending_action_ticket_from_events;
pub(crate) use product::ResumableRecoveryParts;
pub use product::{
    NonResumableRecoveryProduct, RecoveredOpenAsk, RecoveredRunBoundary, RecoveredRunBoundaryKind,
    ResumableRecoveryProduct, RuntimeRecoveryProduct, SummaryRecoveryProduct,
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

    #[must_use]
    pub(crate) const fn input(self) -> SlotIdx {
        self.input
    }

    #[must_use]
    pub(crate) const fn output(self) -> SlotIdx {
        self.output
    }

    #[must_use]
    pub(crate) const fn action_abi_digest(self) -> WorkflowDigest {
        self.action_abi_digest
    }
}

/// Hydrates a frame for the full runtime recovery path.
///
/// Unlike `RuntimeRecoveryBoundary::hydrate_run_frame`, this path is allowed
/// to proceed when a pending action has durable ticket authority and the
/// caller will hydrate the surrounding `RunState` components from artifacts.
pub(crate) fn hydrate_frame_for_full_recovery(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
) -> RuntimeResult<RunFrame> {
    super::frame::validate_recovery_seed_shape(seed)?;
    reject_unrecoverable_full_recovery_state(seed, boundary)?;
    super::frame::hydrate_shape_checked_run_frame(seed)
}

pub(crate) fn recovered_run_boundary_from_seed(
    seed: &RecoveryFrameSeed,
    pending_action: Option<RecoveredPendingActionTicket>,
    workflow: &CompiledWorkflow,
) -> RecoveredRunBoundary {
    if let Some(ticket) = pending_action {
        return RecoveredRunBoundary::from_pending_action(ticket);
    }
    match recoverable_open_ask_boundary(seed, workflow) {
        Some(ask) => RecoveredRunBoundary::from_open_ask(ask),
        None => RecoveredRunBoundary::none(),
    }
}

pub(crate) fn classify_full_recovery_resume(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
) -> vb_storage::recovery::RecoveryCannotResumeState {
    let mut state = initial_full_recovery_resume_state(seed, boundary);
    mark_pending_action_range(seed, boundary, &mut state);
    mark_step_recovery_boundaries(seed, boundary, &mut state);
    state
}

fn initial_full_recovery_resume_state(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
) -> vb_storage::recovery::RecoveryCannotResumeState {
    let mut state = vb_storage::recovery::RecoveryCannotResumeState {
        slot_values: seed.unsupported.slot_values,
        slot_taint: seed.unsupported.slot_taint,
        action_payloads: seed.unsupported.action_payloads,
        pending_actions: !seed.pending_actions.is_empty()
            && boundary.kind() != RecoveredRunBoundaryKind::PendingAction,
        ..vb_storage::recovery::RecoveryCannotResumeState::RESUMABLE
    };
    state.store_missing = state.store_missing || recovered_slots_require_value_store(seed);
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
        RecoveredStepState::Waiting => {
            state.pending_timers = true;
        }
        RecoveredStepState::Asking if !boundary.is_open_ask_step(step) => {
            state.pending_asks = true;
        }
        RecoveredStepState::Asking
        | RecoveredStepState::Running
        | RecoveredStepState::Succeeded
        | RecoveredStepState::Failed => {}
        _ => {
            state.pending_asks = true;
        }
    }
}

fn recovered_slots_require_value_store(seed: &RecoveryFrameSeed) -> bool {
    seed.slots
        .iter()
        .any(|entry| slot_value_requires_value_store(entry.value))
}

const fn slot_value_requires_value_store(value: SlotValue) -> bool {
    matches!(
        value,
        SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
    )
}

fn reject_unrecoverable_full_recovery_state(
    seed: &RecoveryFrameSeed,
    boundary: RecoveredRunBoundary,
) -> RuntimeResult<()> {
    let state = classify_full_recovery_resume(seed, boundary);
    if state.is_resumable() {
        Ok(())
    } else {
        Err(cannot_resume_error(state.unsupported_reason()))
    }
}

fn recoverable_open_ask_boundary(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
) -> Option<RecoveredOpenAsk> {
    let mut current = None;
    for entry in seed
        .steps
        .iter()
        .filter(|entry| entry.state == RecoveredStepState::Asking)
    {
        let ask = recoverable_open_ask(seed, workflow, entry.step)?;
        if let Some(existing) = current
            && existing != ask
        {
            return None;
        }
        current = Some(ask);
    }
    current
}

fn recoverable_open_ask(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    step: StepIdx,
) -> Option<RecoveredOpenAsk> {
    if seed.pc != step {
        return None;
    }
    let node = workflow.node(step)?;
    let CompiledNodeKind::Ask {
        timeout_slot: None, ..
    } = node.kind
    else {
        return None;
    };
    let resume_step = node.next?;
    match workflow.node(resume_step).map(|resume| &resume.kind) {
        Some(CompiledNodeKind::AskResume { .. }) => Some(RecoveredOpenAsk::new(step)),
        _ => None,
    }
}

fn cannot_resume_error(reason: &'static str) -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from(reason),
    }
}
