use super::action_wait::ScheduledAction;
use super::{TraceReplayInput, TraceReplayMode, invalid};
use crate::JournalEvent as E;
use crate::recovery::RecoveryResult;
use std::collections::HashSet;
use vb_core::replay::ReplayAction;
use vb_core::replay::step::{ReplayCollectStateSnapshot, ReplayCollectStates};
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, RunFrame, SlotIdx, SlotValue, StepIdx, Taint,
    ValueStore,
};

pub(super) struct Driver<'a> {
    pub(super) plan: &'a CompiledWorkflow,
    pub(super) frame: RunFrame,
    pub(super) store: ValueStore,
    pub(super) collect: ReplayCollectStates,
    pub(super) collect_snapshots: Vec<OwnedCollectReplaySnapshot>,
    pub(super) active: Option<(StepIdx, ReplayAction)>,
    pub(super) captures: HashSet<SlotIdx>,
    pub(super) ask: Option<(StepIdx, StepIdx, SlotIdx)>,
    pub(super) answer: Option<(SlotValue, Taint)>,
    pub(super) answered: bool,
    pub(super) finished: bool,
    terminal: bool,
    pub(super) action: Option<ScheduledAction>,
    pub(super) waiting: Option<(StepIdx, u16)>,
    pub(super) mode: TraceReplayMode,
}

/// Transient replay evidence aligned to a single Collect slot-write event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedCollectReplaySnapshot {
    /// Durable journal sequence number.
    pub seq: crate::EventSeq,
    /// Run owning the event.
    pub run: vb_core::RunId,
    /// Collect step that emitted the event.
    pub step: StepIdx,
    /// Collector slot written by the event.
    pub slot: SlotIdx,
    /// State after this event, absent when CollectStart emitted an empty page.
    pub state: Option<ReplayCollectStateSnapshot>,
}

pub(super) fn replay(events: &[E], plan: &CompiledWorkflow) -> RecoveryResult<RunFrame> {
    replay_with_inputs(
        events,
        plan,
        &TraceReplayInput {
            initial_inputs: &[],
            store_checkpoint: None,
            mode: TraceReplayMode::Scalar,
        },
    )
}

pub(crate) fn replay_with_inputs(
    events: &[E],
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
) -> RecoveryResult<RunFrame> {
    replay_with_inputs_and_store(events, plan, input).map(|(frame, _store)| frame)
}

pub(crate) fn replay_with_inputs_and_store(
    events: &[E],
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
) -> RecoveryResult<(RunFrame, ValueStore)> {
    replay_with_inputs_and_store_and_collect_snapshots(events, plan, input)
        .map(|(frame, store, _snapshots)| (frame, store))
}

pub(crate) fn replay_with_inputs_and_store_and_collect_snapshots(
    events: &[E],
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
) -> RecoveryResult<(RunFrame, ValueStore, Vec<OwnedCollectReplaySnapshot>)> {
    let first = events
        .first()
        .ok_or_else(|| invalid(plan.entry(), "empty bound trace"))?;
    let store = match input.store_checkpoint {
        Some(checkpoint) => {
            ValueStore::from_checkpoint_v1(checkpoint.clone()).map_err(|error| {
                invalid(plan.entry(), format!("invalid owned value store: {error}"))
            })?
        }
        None => {
            let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
            if let Some(metadata) = plan.source_metadata() {
                vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)
                    .map_err(|e| invalid(plan.entry(), e.to_string()))?;
            }
            store
        }
    };
    let mut frame = vb_core::new_run_frame(first.run_id(), plan)
        .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    for (slot, value, taint) in input.initial_inputs {
        frame
            .write_slot_with_taint(*slot, *value, *taint)
            .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    }
    let mut driver = Driver {
        plan,
        frame,
        store,
        collect: ReplayCollectStates::new(),
        collect_snapshots: Vec::new(),
        active: None,
        captures: HashSet::new(),
        ask: None,
        answer: None,
        answered: false,
        finished: false,
        terminal: false,
        action: None,
        waiting: None,
        mode: input.mode,
    };
    let mut previous = None;
    let mut accepted = false;
    let mut admitted = false;
    let mut started = false;
    let parts = plan.to_parts();
    let budget =
        vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
            .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    if input.mode == TraceReplayMode::Scalar && budget.max_parallel_in_flight != 0 {
        return Err(invalid(
            plan.entry(),
            "parallel trace is outside scalar replay",
        ));
    }
    driver
        .frame
        .set_max_parallel_in_flight(budget.max_parallel_in_flight);
    for event in events {
        if event.run_id() != first.run_id()
            || previous.is_some_and(|seq: u64| seq.checked_add(1) != Some(event.seq().get()))
        {
            return Err(invalid(
                driver.frame.pc(),
                "incomplete bound event sequence",
            ));
        }
        previous = Some(event.seq().get());
        match event {
            E::RunAccepted { workflow, .. } if !accepted && *workflow == plan.digest() => {
                accepted = true
            }
            E::RunAdmission { .. } | E::RunAdmissionBudget { .. }
                if accepted && !admitted && !started =>
            {
                admitted = true;
            }
            E::RunContinuation { .. } if accepted && admitted && !started => {}
            E::CompatibilityDecisionEvent { .. }
            | E::TenantAdmissionEvent { .. }
            | E::PolicyDecisionEvent { .. }
            | E::GateOpenedEvent { .. }
            | E::GateDecisionEvent { .. }
            | E::RetryCheckpoint { .. }
                if accepted => {}
            E::StepStarted { step, .. } if accepted && admitted => {
                started = true;
                driver.start(*step)?;
            }
            E::SlotWrittenEvent {
                run,
                seq,
                slot,
                value,
                extra,
                ..
            } => driver.capture(*run, *seq, *slot, value.as_deref(), extra.as_deref())?,
            E::StepSucceeded { step, output, .. } => driver.succeeded(*step, *output)?,
            E::StepFailed { step, .. } => driver.failed(*step)?,
            E::AskScheduledEvent { step, .. } => driver.scheduled_ask(*step)?,
            E::AskAnsweredEvent { step, .. } => driver.answered_ask(*step)?,
            E::AskTimedOutEvent { step, .. } => driver.timed_out_ask(*step)?,
            E::ActionFailedEvent { step, .. } => driver.action_failed(*step)?,
            E::RunFailedEvent { .. } if driver.active.is_none() => driver.terminal = true,
            E::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
                ..
            } => driver.scheduled_action(*ticket, *input, *output, *action_abi_digest)?,
            E::ActionCompletedEnvelope {
                ticket,
                outcome,
                output,
                value,
                taint,
                value_digest,
                encoded_len,
                action_abi_digest,
                ..
            } => driver.completed_action(super::action_complete::ActionCompletion {
                ticket: *ticket,
                outcome: *outcome,
                output: *output,
                value,
                taint: *taint,
                value_digest,
                encoded_len: *encoded_len,
                abi_digest: *action_abi_digest,
            })?,
            E::WaitScheduledEvent { step, attempt, .. } => {
                driver.wait_scheduled(*step, *attempt)?;
            }
            E::WaitResolvedEvent { step, attempt, .. } => {
                driver.wait_resolved(*step, *attempt)?;
            }
            E::RunFinished { result, .. }
                if driver.finished && !driver.terminal && driver.active.is_none() =>
            {
                let Some(K::Finish { result: expected }) =
                    plan.node(driver.frame.pc()).map(|node| &node.kind)
                else {
                    return Err(invalid(driver.frame.pc(), "terminal without Finish node"));
                };
                if expected != result {
                    return Err(invalid(
                        driver.frame.pc(),
                        "terminal result differs from Finish node",
                    ));
                }
                driver
                    .frame
                    .read_slot(*result)
                    .map_err(|e| invalid(driver.frame.pc(), e.to_string()))?;
                driver.terminal = true;
            }
            _ => {
                return Err(invalid(
                    driver.frame.pc(),
                    format!(
                        "unsupported or out-of-order bound event {:?}",
                        event.record_kind()
                    ),
                ));
            }
        }
        if driver.frame.executed() > budget.max_total_steps {
            return Err(invalid(
                driver.frame.pc(),
                "bound trace exceeds workflow step budget",
            ));
        }
    }
    if driver.active.is_some() && !driver.pending_boundary() {
        return Err(invalid(driver.frame.pc(), "incomplete step receipt"));
    }
    if input.mode == TraceReplayMode::Scalar && driver.frame.parallel_in_flight() != 0 {
        return Err(invalid(
            driver.frame.pc(),
            "unknown scalar parallel reservations",
        ));
    }
    if driver.finished && !driver.terminal {
        return Err(invalid(driver.frame.pc(), "terminal receipt missing"));
    }
    Ok((driver.frame, driver.store, driver.collect_snapshots))
}
