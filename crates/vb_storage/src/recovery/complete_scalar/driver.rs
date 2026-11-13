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
    pub(super) run_id: vb_core::RunId,
    pub(super) previous_seq: Option<u64>,
    pub(super) accepted: bool,
    pub(super) admitted: bool,
    pub(super) started: bool,
    pub(super) max_total_steps: u64,
    pub(super) budgeted_admission: bool,
    pub(super) origin_seen: bool,
    pub(super) admission_prefix_closed: bool,
    pub(super) compatibility_seen: bool,
    pub(super) tenant_seen: bool,
    pub(super) next_admission_seq: u64,
}

#[path = "driver/events.rs"]
mod events;

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

struct TraceBudget {
    max_total_steps: u64,
    max_parallel_in_flight: u16,
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
    let run_id = first_run_id(events, plan)?;
    let mut driver = replay_driver(plan, input, run_id)?;
    events
        .iter()
        .try_for_each(|event| driver.apply_bound_event(event))?;
    driver.finish(input.mode)
}

fn first_run_id(events: &[E], plan: &CompiledWorkflow) -> RecoveryResult<vb_core::RunId> {
    events
        .first()
        .map(E::run_id)
        .ok_or_else(|| invalid(plan.entry(), "empty bound trace"))
}

fn replay_driver<'a>(
    plan: &'a CompiledWorkflow,
    input: &TraceReplayInput<'_>,
    run_id: vb_core::RunId,
) -> RecoveryResult<Driver<'a>> {
    let store = initial_store(plan, input)?;
    let mut frame = initial_frame(plan, input, run_id)?;
    let budget = trace_budget(plan, input.mode)?;
    frame.set_max_parallel_in_flight(budget.max_parallel_in_flight);
    Ok(Driver {
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
        run_id,
        previous_seq: None,
        accepted: false,
        admitted: false,
        started: false,
        max_total_steps: budget.max_total_steps,
        budgeted_admission: false,
        origin_seen: false,
        admission_prefix_closed: false,
        compatibility_seen: false,
        tenant_seen: false,
        next_admission_seq: 2,
    })
}

fn initial_store(
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
) -> RecoveryResult<ValueStore> {
    match input.store_checkpoint {
        Some(checkpoint) => ValueStore::from_checkpoint_v1(checkpoint.clone())
            .map_err(|error| invalid(plan.entry(), format!("invalid owned value store: {error}"))),
        None => source_seeded_store(plan),
    }
}

fn source_seeded_store(plan: &CompiledWorkflow) -> RecoveryResult<ValueStore> {
    let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
    if let Some(metadata) = plan.source_metadata() {
        vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)
            .map_err(|error| invalid(plan.entry(), error.to_string()))?;
    }
    Ok(store)
}

fn initial_frame(
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
    run_id: vb_core::RunId,
) -> RecoveryResult<RunFrame> {
    let mut frame = vb_core::new_run_frame(run_id, plan)
        .map_err(|error| invalid(plan.entry(), error.to_string()))?;
    input
        .initial_inputs
        .iter()
        .try_for_each(|(slot, value, taint)| {
            frame
                .write_slot_with_taint(*slot, *value, *taint)
                .map_err(|error| invalid(plan.entry(), error.to_string()))
        })?;
    Ok(frame)
}

fn trace_budget(plan: &CompiledWorkflow, mode: TraceReplayMode) -> RecoveryResult<TraceBudget> {
    let parts = plan.to_parts();
    let budget =
        vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
            .map_err(|error| invalid(plan.entry(), error.to_string()))?;
    if mode == TraceReplayMode::Scalar && budget.max_parallel_in_flight != 0 {
        return Err(invalid(
            plan.entry(),
            "parallel trace is outside scalar replay",
        ));
    }
    Ok(TraceBudget {
        max_total_steps: budget.max_total_steps,
        max_parallel_in_flight: budget.max_parallel_in_flight,
    })
}

impl Driver<'_> {
    fn finish(
        self,
        mode: TraceReplayMode,
    ) -> RecoveryResult<(RunFrame, ValueStore, Vec<OwnedCollectReplaySnapshot>)> {
        self.validate_final_state(mode)?;
        Ok((self.frame, self.store, self.collect_snapshots))
    }

    fn validate_final_state(&self, mode: TraceReplayMode) -> RecoveryResult<()> {
        if self.budgeted_admission && !self.origin_seen {
            return Err(invalid(self.frame.pc(), "budget origin missing"));
        }
        if self.active.is_some() && !self.pending_boundary() {
            return Err(invalid(self.frame.pc(), "incomplete step receipt"));
        }
        validate_parallel_state(&self.frame, mode)?;
        validate_terminal_receipt(&self.frame, self.finished, self.terminal)
    }
}

fn validate_parallel_state(frame: &RunFrame, mode: TraceReplayMode) -> RecoveryResult<()> {
    if mode == TraceReplayMode::Scalar && frame.parallel_in_flight() != 0 {
        return Err(invalid(frame.pc(), "unknown scalar parallel reservations"));
    }
    Ok(())
}

fn validate_terminal_receipt(
    frame: &RunFrame,
    finished: bool,
    terminal: bool,
) -> RecoveryResult<()> {
    if finished && !terminal {
        return Err(invalid(frame.pc(), "terminal receipt missing"));
    }
    Ok(())
}
