use vb_core::ids::StepIdx;
use vb_core::{CompiledNodeKind, CompiledWorkflow};
use vb_storage::recovery::{RecoveredStepState, RecoveryFrameSeed};

use super::boundary::{RecoveredOpenAsk, RecoveredTimerSuspension};
use super::event_wait::RecoveredOpenWaitEvent;
use super::full_ask_schedule::pending_ask_schedule_for_step;
use super::full_wait_schedule::pending_wait_schedule_for_step;

pub(super) fn recoverable_open_ask_boundary(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredOpenAsk> {
    let mut current = None;
    for entry in seed
        .steps
        .iter()
        .filter(|entry| entry.state == RecoveredStepState::Asking)
    {
        let ask = recoverable_open_ask(seed, workflow, entry.step, events)?;
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
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredOpenAsk> {
    let (boundary_seq, _) = recoverable_ask_schedule(seed, workflow, step, events, false)?;
    Some(RecoveredOpenAsk::new(step, boundary_seq))
}

pub(super) fn recoverable_timed_ask_boundary(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredTimerSuspension> {
    let mut current = None;
    for entry in seed
        .steps
        .iter()
        .filter(|entry| entry.state == RecoveredStepState::Asking)
    {
        let ask = recoverable_timed_ask(seed, workflow, entry.step, events)?;
        if let Some(existing) = current
            && existing != ask
        {
            return None;
        }
        current = Some(ask);
    }
    current
}

fn recoverable_timed_ask(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    step: StepIdx,
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredTimerSuspension> {
    let (boundary_seq, _) = recoverable_ask_schedule(seed, workflow, step, events, true)?;
    Some(RecoveredTimerSuspension::new(step, boundary_seq))
}

/// Validates a suspended ask against the compiled workflow and the durable
/// ask schedule. `timed` selects asks with (versus without) a timeout slot;
/// a timed ask additionally implies timer authority, which the shard restores
/// from the recovered timeout slot.
fn recoverable_ask_schedule(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    step: StepIdx,
    events: &[vb_storage::JournalEvent],
    timed: bool,
) -> Option<(vb_storage::EventSeq, u16)> {
    if seed.pc != step {
        return None;
    }
    let node = workflow.node(step)?;
    let CompiledNodeKind::Ask { timeout_slot, .. } = node.kind else {
        return None;
    };
    if timed != timeout_slot.slot().is_some() {
        return None;
    }
    let resume_step = node.routing.next.step()?;
    match workflow.node(resume_step).map(|resume| &resume.kind) {
        Some(CompiledNodeKind::AskResume { .. }) => pending_ask_schedule_for_step(events, step),
        _ => None,
    }
}

pub(super) fn recoverable_open_wait_event_boundary(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredOpenWaitEvent> {
    let step = seed.pc;
    let entry = seed.steps.iter().find(|entry| entry.step == step)?;
    if !matches!(
        entry.state,
        RecoveredStepState::Running | RecoveredStepState::Waiting
    ) || !untimed_wait_node(workflow, step)
    {
        return None;
    }
    if pending_wait_schedule_for_step(events, step).is_some() {
        return None;
    }
    durable_wait_start(events, step).then_some(RecoveredOpenWaitEvent::new(step))
}

fn untimed_wait_node(workflow: &CompiledWorkflow, step: StepIdx) -> bool {
    workflow.node(step).is_some_and(|node| {
        matches!(
            node.kind,
            CompiledNodeKind::WaitEvent { timeout_slot, .. }
                if timeout_slot.slot().is_none()
        )
    })
}

fn durable_wait_start(events: &[vb_storage::JournalEvent], step: StepIdx) -> bool {
    events.iter().any(|event| {
        matches!(event, vb_storage::JournalEvent::StepStarted { step: started, .. } if *started == step)
    })
}

pub(super) fn recoverable_waiting_timer_boundary(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredTimerSuspension> {
    let mut current = None;
    for entry in seed
        .steps
        .iter()
        .filter(|entry| entry.state == RecoveredStepState::Waiting)
    {
        let timer = recoverable_waiting_timer(seed, workflow, entry.step, events)?;
        if let Some(existing) = current
            && existing != timer
        {
            return None;
        }
        current = Some(timer);
    }
    current
}

/// Validates a suspended wait against the compiled workflow and the durable
/// wait schedule. Untimed event waits use the separate open-boundary path.
fn recoverable_waiting_timer(
    seed: &RecoveryFrameSeed,
    workflow: &CompiledWorkflow,
    step: StepIdx,
    events: &[vb_storage::JournalEvent],
) -> Option<RecoveredTimerSuspension> {
    if seed.pc != step {
        return None;
    }
    let node = workflow.node(step)?;
    match node.kind {
        CompiledNodeKind::WaitUntil { .. } => {}
        CompiledNodeKind::WaitEvent { timeout_slot, .. } if timeout_slot.slot().is_some() => {}
        _ => return None,
    }
    node.routing.next.step()?;
    let (boundary_seq, _) = pending_wait_schedule_for_step(events, step)?;
    Some(RecoveredTimerSuspension::new(step, boundary_seq))
}
