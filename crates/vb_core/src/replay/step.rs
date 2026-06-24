#![forbid(unsafe_code)]
//! Replay step execution.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use std::collections::HashMap;

use crate::ids::{ConstIdx, ExprIdx, ListId, SlotIdx, StepIdx, SymbolId};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use super::{ReplayError, eval_expr_for_replay, engine_to_replay_err};

/// Typed non-deterministic suspension kind observed during replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SuspensionKind {
    /// Action boundary waiting for an external action completion.
    ActionPending,
    /// Ask node waiting for an external answer.
    AskPending,
    /// Wait-until node waiting for a deadline.
    WaitUntil,
    /// Wait-event node waiting for an event or timeout.
    WaitEvent,
}

impl SuspensionKind {
    /// Stable diagnostic name for logs and compatibility assertions.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActionPending => "Do",
            Self::AskPending => "Ask",
            Self::WaitUntil => "WaitUntil",
            Self::WaitEvent => "WaitEvent",
        }
    }
}

impl core::fmt::Display for SuspensionKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str((*self).as_str())
    }
}

/// Internal action returned by `replay_step`.
#[non_exhaustive]
pub enum ReplayAction {
    /// Continue to the next step.
    Continue(StepIdx),
    /// The run finished.
    Finished,
    /// The run is suspended on a non-deterministic node.
    Suspended { step: StepIdx, kind: SuspensionKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplayCollectState {
    source: ListId,
    current_page: ListId,
    cursor: usize,
    page_size: usize,
    item_count: usize,
    taint: Taint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplayCollectStartArgs {
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
}

#[derive(Debug, Default)]
pub struct ReplayCollectStates {
    entries: HashMap<SlotIdx, ReplayCollectState>,
}

impl ReplayCollectStates {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn upsert(&mut self, collector: SlotIdx, state: ReplayCollectState) {
        self.entries.insert(collector, state);
    }

    fn find(
        &self,
        collector: SlotIdx,
        current_page: ListId,
    ) -> Result<ReplayCollectState, ReplayError> {
        self.entries
            .get(&collector)
            .filter(|state| state.current_page == current_page)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "collect pagination state missing during replay",
            })
    }

    fn remove(&mut self, collector: SlotIdx) {
        self.entries.remove(&collector);
    }
}

/// Replays a single deterministic step.
///
/// For deterministic node kinds (SetConst, Copy, EvalExpr, BuildObject, BuildList,
/// Finish, Nop), executes the same logic as the engine's `step_once`.
/// For non-deterministic (Do/Action, Ask, WaitUntil, WaitEvent), returns a
/// suspension signal.
pub fn replay_step(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
) -> Result<ReplayAction, ReplayError> {
    replay_step_with_collect(node, run, store, plan, &mut ReplayCollectStates::new())
}

/// Replays a single deterministic step with caller-owned collect pagination state.
pub fn replay_step_with_collect(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
    collect_states: &mut ReplayCollectStates,
) -> Result<ReplayAction, ReplayError> {
    match &node.kind {
        CompiledNodeKind::Nop => replay_nop(node, run),
        CompiledNodeKind::SetConst { value } => replay_set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => replay_copy(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => replay_eval_expr(plan, run, store, node, *expr),
        CompiledNodeKind::BuildObject { fields } => replay_build_object(run, store, node, fields),
        CompiledNodeKind::BuildList { items } => replay_build_list(run, store, node, items),
        CompiledNodeKind::Finish { result } => replay_finish(run, *result),
        CompiledNodeKind::Jump { target } => replay_jump(run, *target),
        CompiledNodeKind::Do { .. } => Ok(replay_suspend(node, SuspensionKind::ActionPending)),
        CompiledNodeKind::Ask { .. } => Ok(replay_suspend(node, SuspensionKind::AskPending)),
        CompiledNodeKind::WaitUntil { .. } => Ok(replay_suspend(node, SuspensionKind::WaitUntil)),
        CompiledNodeKind::WaitEvent { .. } => Ok(replay_suspend(node, SuspensionKind::WaitEvent)),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => super::choose::replay_choose_slot(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => super::choose::replay_choose_expr(plan, run, store, branches, *otherwise),
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => replay_collect_start(
            run,
            store,
            collect_states,
            node,
            ReplayCollectStartArgs {
                source: *source,
                limit: *limit,
                page_size: *page_size,
                body: *body,
                done: *done,
            },
        ),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            ..
        } => replay_collect_page(run, *collector_slot, *body),
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => replay_collect_next(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            replay_collect_finish(run, collect_states, node, *collector_slot)
        }
        _ => Err(ReplayError::Internal {
            reason: "unsupported node kind for replay",
        }),
    }
}

fn replay_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut ReplayCollectStates,
    node: &CompiledNode,
    args: ReplayCollectStartArgs,
) -> Result<ReplayAction, ReplayError> {
    let list_id = read_list_slot(run, args.source)?;
    let source_taint = run.read_taint(args.source).map_err(engine_to_replay_err)?;
    let page_size = replay_page_size(args.page_size)?;
    let item_limit = replay_item_limit(args.limit)?;
    if page_size > item_limit {
        return Err(ReplayError::Internal {
            reason: "collect page size exceeds limit during replay",
        });
    }
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "collect source list missing during replay",
    })?;
    let item_count = items.len();
    if item_count > item_limit {
        return Err(ReplayError::Internal {
            reason: "collect item count exceeds limit during replay",
        });
    }
    let collector = node.output.map_or(args.source, |slot| slot);
    if items.is_empty() {
        write_empty_collect_page(run, store, collector, source_taint)?;
        states.remove(collector);
        return replay_jump(run, args.done);
    }
    let page = collect_page_items(items, 0, page_size)?;
    let current_page = write_collect_page(run, store, collector, &page, source_taint)?;
    states.upsert(
        collector,
        ReplayCollectState {
            source: list_id,
            current_page,
            cursor: page.len(),
            page_size,
            item_count,
            taint: source_taint,
        },
    );
    replay_jump(run, args.body)
}

fn replay_collect_page(
    run: &mut RunFrame,
    collector_slot: SlotIdx,
    body: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    validate_collect_slot_list(run, collector_slot)?;
    replay_jump(run, body)
}

fn replay_collect_next(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut ReplayCollectStates,
    collector_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let current_id = read_list_slot(run, collector_slot)?;
    let current = store.list(current_id).map_err(|_| ReplayError::Internal {
        reason: "collect current page missing during replay",
    })?;
    if current.is_empty() {
        states.remove(collector_slot);
        return replay_jump(run, done);
    }
    let state = states.find(collector_slot, current_id)?;
    let source_items = store
        .list(state.source)
        .map_err(|_| ReplayError::Internal {
            reason: "collect source list missing during replay",
        })?;
    if source_items.len() != state.item_count {
        return Err(ReplayError::Internal {
            reason: "collect source length changed during replay",
        });
    }
    if state.cursor >= state.item_count {
        write_empty_collect_page(run, store, collector_slot, state.taint)?;
        states.remove(collector_slot);
        return replay_jump(run, done);
    }
    let page = collect_page_items(source_items, state.cursor, state.page_size)?;
    let current_page = write_collect_page(run, store, collector_slot, &page, state.taint)?;
    states.upsert(
        collector_slot,
        ReplayCollectState {
            current_page,
            cursor: state
                .cursor
                .checked_add(page.len())
                .ok_or(ReplayError::Internal {
                    reason: "collect cursor overflow during replay",
                })?,
            ..state
        },
    );
    replay_jump(run, body)
}

fn validate_collect_slot_list(run: &RunFrame, slot: SlotIdx) -> Result<(), ReplayError> {
    read_list_slot(run, slot).map(|_list| ())
}

fn replay_collect_finish(
    run: &mut RunFrame,
    states: &mut ReplayCollectStates,
    node: &CompiledNode,
    collector_slot: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(collector_slot).map_err(engine_to_replay_err)?;
    let taint = run.read_taint(collector_slot).map_err(engine_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "CollectFinish node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    states.remove(collector_slot);
    advance_to_next(run, node).map(ReplayAction::Continue)
}

fn read_list_slot(run: &RunFrame, slot: SlotIdx) -> Result<ListId, ReplayError> {
    match *run.read_slot(slot).map_err(engine_to_replay_err)? {
        SlotValue::List(list) => Ok(list),
        _ => Err(ReplayError::Internal {
            reason: "collect slot was not list during replay",
        }),
    }
}

fn replay_page_size(raw: u32) -> Result<usize, ReplayError> {
    match raw {
        0 => Err(ReplayError::Internal {
            reason: "collect page size was zero during replay",
        }),
        value => usize::try_from(value).map_err(|_| ReplayError::Internal {
            reason: "collect page size overflow during replay",
        }),
    }
}

fn replay_item_limit(raw: u32) -> Result<usize, ReplayError> {
    usize::try_from(raw).map_err(|_| ReplayError::Internal {
        reason: "collect limit overflow during replay",
    })
}

fn collect_page_items(
    items: &[SlotValue],
    start: usize,
    page_size: usize,
) -> Result<Box<[SlotValue]>, ReplayError> {
    let remaining = items
        .len()
        .checked_sub(start)
        .ok_or(ReplayError::Internal {
            reason: "collect cursor beyond item count during replay",
        })?;
    Ok(items
        .iter()
        .skip(start)
        .take(page_size.min(remaining))
        .copied()
        .collect::<Vec<_>>()
        .into_boxed_slice())
}

fn write_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    items: &[SlotValue],
    taint: Taint,
) -> Result<ListId, ReplayError> {
    let page_id = store
        .insert_list(items.to_vec().into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert collect page failed during replay",
        })?;
    run.write_slot_with_taint(collector, SlotValue::List(page_id), taint)
        .map_err(engine_to_replay_err)?;
    Ok(page_id)
}

fn write_empty_collect_page(
    run: &mut RunFrame,
    store: &mut ValueStore,
    collector: SlotIdx,
    taint: Taint,
) -> Result<(), ReplayError> {
    write_collect_page(run, store, collector, &[], taint).map(|_page_id| ())
}

fn replay_nop(node: &CompiledNode, run: &mut RunFrame) -> Result<ReplayAction, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "Nop node missing next step",
    })?;
    run.set_pc(next).map_err(engine_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_finish(run: &mut RunFrame, result: SlotIdx) -> Result<ReplayAction, ReplayError> {
    let _value = *run.read_slot(result).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading finish result slot",
        },
    })?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Finished)
}

fn replay_jump(run: &mut RunFrame, target: StepIdx) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(engine_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(target))
}

fn replay_suspend(node: &CompiledNode, kind: SuspensionKind) -> ReplayAction {
    ReplayAction::Suspended {
        step: node.id,
        kind,
    }
}

fn replay_set_const(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &CompiledNode,
    value: ConstIdx,
) -> Result<ReplayAction, ReplayError> {
    let constant = plan.constant(value).copied().ok_or(ReplayError::Internal {
        reason: "constant out of bounds",
    })?;
    let slot_value = constant
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "SetConst node missing output slot",
    })?;
    run.write_slot(output, slot_value)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_copy(
    run: &mut RunFrame,
    node: &CompiledNode,
    source: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(source).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading copy source slot",
        },
    })?;
    let taint = run.read_taint(source).map_err(engine_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "Copy node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_eval_expr(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    expr: ExprIdx,
) -> Result<ReplayAction, ReplayError> {
    let (value, taint) = eval_expr_for_replay(plan, run, store, expr)
        .map_err(|_| ReplayError::ExpressionEvalFailed { step: node.id })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "EvalExpr node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_build_object(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<ReplayAction, ReplayError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields.get(index).ok_or(ReplayError::Internal {
            reason: "build_object field index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_object field slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(engine_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(ObjectField {
            key: *key,
            value,
            taint: slot_taint,
        });
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_object field index overflow",
        })?;
    }
    let handle = store
        .insert_object(entries.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert_object failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildObject node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), accumulated_taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_build_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    items: &[SlotIdx],
) -> Result<ReplayAction, ReplayError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items.get(index).ok_or(ReplayError::Internal {
            reason: "build_list item index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_list item slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(engine_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        values.push(value);
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_list item index overflow",
        })?;
    }
    let handle =
        store
            .insert_list(values.into_boxed_slice())
            .map_err(|_| ReplayError::Internal {
                reason: "insert_list failed",
            })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildList node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), accumulated_taint)
        .map_err(engine_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn advance_to_next(run: &mut RunFrame, node: &CompiledNode) -> Result<StepIdx, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "node missing next step",
    })?;
    run.set_pc(next).map_err(engine_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(next)
}

fn increment_replay_executed(run: &mut RunFrame) -> Result<(), ReplayError> {
    run.increment_executed().map_err(|_| ReplayError::Internal {
        reason: "executed counter overflow",
    })
}

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
