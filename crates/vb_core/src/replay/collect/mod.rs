#![forbid(unsafe_code)]
//! Collect/pagination subsystem.
//!
//! Implements the CollectStart → CollectPage → CollectNext → CollectFinish
//! lifecycle for deterministically replaying list iteration with pagination
//! state tracking.

use std::collections::HashMap;

use crate::frame::RunFrame;
use crate::ids::{ListId, SlotIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;

use super::{ReplayAction, ReplayError, basic::increment_replay_executed, slot_to_replay_err};

// ---------------------------------------------------------------------------
// Pagination state types
// ---------------------------------------------------------------------------

/// Internal collector state tracked across CollectPage/CollectNext steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplayCollectState {
    pub(super) source: ListId,
    pub(super) current_page: ListId,
    pub(super) cursor: usize,
    pub(super) page_size: usize,
    pub(super) item_count: usize,
    pub(super) taint: Taint,
}

/// Arguments supplied to the CollectStart step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplayCollectStartArgs {
    pub(super) source: SlotIdx,
    pub(super) limit: u32,
    pub(super) page_size: u32,
    pub(super) body: StepIdx,
    pub(super) done: StepIdx,
}

// Re-export StepIdx here since it's used in the public-facing start args
use crate::ids::StepIdx;

/// Caller-owned pagination state map.
///
/// One entry per active collector slot. The outer `step.rs` dispatch
/// creates a single instance per replay session and threads it through
/// all collect-related calls.
#[derive(Debug, Default)]
pub struct ReplayCollectStates {
    entries: HashMap<SlotIdx, ReplayCollectState>,
}

impl ReplayCollectStates {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn upsert(&mut self, collector: SlotIdx, state: ReplayCollectState) {
        self.entries.insert(collector, state);
    }

    pub(super) fn find(
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

    pub(super) fn remove(&mut self, collector: SlotIdx) {
        self.entries.remove(&collector);
    }
}

// ---------------------------------------------------------------------------
// Collect step handlers
// ---------------------------------------------------------------------------

/// Handles CollectStart: validates source list, creates first page,
/// initializes pagination state.
pub(super) fn replay_collect_start(
    run: &mut RunFrame,
    store: &mut ValueStore,
    states: &mut ReplayCollectStates,
    node: &crate::workflow::CompiledNode,
    args: ReplayCollectStartArgs,
) -> Result<ReplayAction, ReplayError> {
    let list_id = read_list_slot(run, args.source)?;
    let source_taint = run.read_taint(args.source).map_err(slot_to_replay_err)?;
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

/// Handles CollectPage: validates that the collector slot holds a list,
/// then advances to the body step.
pub(super) fn replay_collect_page(
    run: &mut RunFrame,
    collector_slot: SlotIdx,
    body: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    validate_collect_slot_list(run, collector_slot)?;
    replay_jump(run, body)
}

/// Handles CollectNext: reads the next page of items from the source
/// list into the collector slot.
pub(super) fn replay_collect_next(
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

/// Handles CollectFinish: writes the collector slot value to the
/// node's output slot and cleans up.
pub(super) fn replay_collect_finish(
    run: &mut RunFrame,
    states: &mut ReplayCollectStates,
    node: &crate::workflow::CompiledNode,
    collector_slot: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(collector_slot).map_err(slot_to_replay_err)?;
    let taint = run.read_taint(collector_slot).map_err(slot_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "CollectFinish node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    states.remove(collector_slot);
    super::basic::advance_to_next(run, node).map(ReplayAction::Continue)
}

// ---------------------------------------------------------------------------
// Collect helpers
// ---------------------------------------------------------------------------

fn validate_collect_slot_list(run: &RunFrame, slot: SlotIdx) -> Result<(), ReplayError> {
    read_list_slot(run, slot).map(|_list| ())
}

fn read_list_slot(run: &RunFrame, slot: SlotIdx) -> Result<ListId, ReplayError> {
    match *run.read_slot(slot).map_err(slot_to_replay_err)? {
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
        .map_err(slot_to_replay_err)?;
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

fn replay_jump(run: &mut RunFrame, target: StepIdx) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(slot_to_replay_err)?;
    increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(target))
}
