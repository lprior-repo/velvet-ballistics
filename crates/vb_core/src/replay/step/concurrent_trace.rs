//! Chronological concurrent replay: one recorded transition per invocation.
use super::super::{ReplayError, engine_to_replay_err};
use super::concurrent_validation::check_for_each_capacity;
use super::types::ReplayAction;
use crate::{
    CompiledNode, CompiledNodeKind as K, CompiledWorkflow, ListId, RunFrame, SlotIdx, SlotValue,
    StepIdx, StepState, Taint, ValueStore,
};

fn invalid(reason: &'static str) -> ReplayError {
    ReplayError::Internal { reason }
}

struct ForEachTrace {
    owner: StepIdx,
    input: SlotIdx,
    item: SlotIdx,
    body: StepIdx,
    join: StepIdx,
    output: SlotIdx,
    source: ListId,
    count: usize,
}

impl ForEachTrace {
    fn checked(
        plan: &CompiledWorkflow,
        run: &RunFrame,
        store: &ValueStore,
        owner: StepIdx,
    ) -> Result<Self, ReplayError> {
        let node = plan
            .node(owner)
            .ok_or_else(|| invalid("concurrent owner missing"))?;
        let K::ForEachConcurrentStart {
            input,
            item_slot,
            limit,
            at_once,
            body,
            join,
        } = node.kind
        else {
            return Err(invalid("concurrent trace owner is not for_each"));
        };
        let source = list_slot(run, input)?;
        let count = store.list(source).map_err(engine_to_replay_err)?.len();
        if count > limit.as_usize() {
            return Err(ReplayError::IterationLimitExceeded {
                resource: "for_each_limit",
            });
        }
        check_for_each_capacity(plan, owner, join, count, limit, at_once)?;
        let output = plan
            .node(join)
            .and_then(|node| node.output.slot())
            .ok_or_else(|| invalid("concurrent output missing"))?;
        Ok(Self {
            owner,
            input,
            item: item_slot,
            body,
            join,
            output,
            source,
            count,
        })
    }

    fn advance(
        &self,
        run: &mut RunFrame,
        store: &ValueStore,
        ordinal: usize,
    ) -> Result<ReplayAction, ReplayError> {
        let next = if ordinal == self.count {
            self.join
        } else {
            let ordinal =
                u32::try_from(ordinal).map_err(|_| invalid("concurrent ordinal overflow"))?;
            let (value, taint) = store
                .list_item_with_taint(self.source, ordinal)
                .map_err(engine_to_replay_err)?;
            let taint = crate::value::join_taint(
                run.read_taint(self.input).map_err(engine_to_replay_err)?,
                taint,
            );
            run.write_slot_with_taint(self.item, value, taint)
                .map_err(engine_to_replay_err)?;
            self.body
        };
        run.set_pc(next).map_err(engine_to_replay_err)?;
        Ok(ReplayAction::Continue(next))
    }

    fn start(
        &self,
        run: &mut RunFrame,
        store: &mut ValueStore,
    ) -> Result<ReplayAction, ReplayError> {
        let empty = store
            .insert_list(Vec::<SlotValue>::new())
            .map_err(engine_to_replay_err)?;
        let taint = run.read_taint(self.input).map_err(engine_to_replay_err)?;
        run.write_slot_with_taint(self.output, SlotValue::List(empty), taint)
            .map_err(engine_to_replay_err)?;
        self.advance(run, store, 0)
    }

    fn complete(
        &self,
        run: &mut RunFrame,
        store: &mut ValueStore,
        result: Option<SlotIdx>,
    ) -> Result<ReplayAction, ReplayError> {
        if run.step_state(self.owner).map_err(engine_to_replay_err)? != StepState::Succeeded {
            return Err(invalid("concurrent owner has not succeeded"));
        }
        let current = list_slot(run, self.output)?;
        let completed = store.list(current).map_err(engine_to_replay_err)?.len();
        if completed >= self.count {
            return Err(invalid("concurrent cursor exceeds input"));
        }
        let value = lane_result(run, result)?;
        append_result(run, store, self.output, current, value)?;
        let next = completed
            .checked_add(1)
            .ok_or_else(|| invalid("concurrent cursor overflow"))?;
        self.advance(run, store, next)
    }
}

fn list_slot(run: &RunFrame, slot: SlotIdx) -> Result<ListId, ReplayError> {
    match *run.read_slot(slot).map_err(engine_to_replay_err)? {
        SlotValue::List(id) => Ok(id),
        _ => Err(invalid("concurrent cursor or input is not a list")),
    }
}

fn lane_result(run: &RunFrame, result: Option<SlotIdx>) -> Result<(SlotValue, Taint), ReplayError> {
    match result {
        Some(slot) => Ok((
            *run.read_slot(slot).map_err(engine_to_replay_err)?,
            run.read_taint(slot).map_err(engine_to_replay_err)?,
        )),
        None => Ok((SlotValue::Null, Taint::Clean)),
    }
}

fn append_result(
    run: &mut RunFrame,
    store: &mut ValueStore,
    output: SlotIdx,
    current: ListId,
    value: (SlotValue, Taint),
) -> Result<(), ReplayError> {
    let (values, taints) = store
        .prepare_list_append(current, Some(value))
        .map_err(engine_to_replay_err)?;
    let list = store
        .insert_list_with_taint(values, taints)
        .map_err(engine_to_replay_err)?;
    let taint = crate::value::join_taint(
        run.read_taint(output).map_err(engine_to_replay_err)?,
        value.1,
    );
    run.write_slot_with_taint(output, SlotValue::List(list), taint)
        .map_err(engine_to_replay_err)
}

/// Reconstructs one journaled concurrent transition, preserving arena allocation order.
/// Non-concurrent nodes return `None` for the ordinary replay dispatcher.
pub fn replay_concurrent_trace_step(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
) -> Result<Option<ReplayAction>, ReplayError> {
    let action = match node.kind {
        K::ForEachConcurrentStart { .. } => {
            ForEachTrace::checked(plan, run, store, node.id)?.start(run, store)?
        }
        K::ConcurrentLaneEnd { owner, result } => {
            ForEachTrace::checked(plan, run, store, owner)?.complete(run, store, result)?
        }
        K::ConcurrentGroupJoin { .. } => {
            let next = node
                .routing
                .next
                .step()
                .ok_or_else(|| invalid("concurrent join continuation missing"))?;
            run.set_pc(next).map_err(engine_to_replay_err)?;
            ReplayAction::Continue(next)
        }
        _ => return Ok(None),
    };
    Ok(Some(action))
}

/// Rearms only a previously completed ForEach body at its next recorded start.
pub fn prepare_concurrent_trace_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    step: StepIdx,
) -> Result<(), ReplayError> {
    if run.step_state(step).map_err(engine_to_replay_err)? != StepState::Succeeded {
        return Ok(());
    }
    let owner = (0..plan.node_count())
        .filter_map(|raw| plan.node(StepIdx::new(raw)))
        .find(|node| matches!(node.kind, K::ForEachConcurrentStart { body, .. } if body == step));
    if let Some(owner) = owner {
        if run.step_state(owner.id).map_err(engine_to_replay_err)? != StepState::Succeeded {
            return Err(invalid("concurrent body lacks owner authority"));
        }
        run.mark_pending(step).map_err(engine_to_replay_err)?;
    }
    Ok(())
}

/// Mirrors the live lane-end boundary, leaving the entry succeeded until activation.
pub fn finish_concurrent_trace_step(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    step: StepIdx,
) -> Result<(), ReplayError> {
    let Some(K::ConcurrentLaneEnd { owner, .. }) = plan.node(step).map(|node| &node.kind) else {
        return Ok(());
    };
    let Some(K::ForEachConcurrentStart { body, .. }) = plan.node(*owner).map(|node| &node.kind)
    else {
        return Ok(());
    };
    if run.pc() != *body || *body > step {
        return Ok(());
    }
    (body.get()..=step.get())
        .map(StepIdx::new)
        .filter(|candidate| candidate != body)
        .try_for_each(|candidate| {
            if run.step_state(candidate).map_err(engine_to_replay_err)? == StepState::Succeeded {
                run.mark_pending(candidate).map_err(engine_to_replay_err)?;
            }
            Ok(())
        })
}
