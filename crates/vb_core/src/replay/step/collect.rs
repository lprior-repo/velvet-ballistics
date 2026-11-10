use crate::frame::RunFrame;
use crate::ids::{ListId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::CompiledNode;

use super::super::{ReplayError, engine_to_replay_err};
use super::nodes::{advance_to_next, replay_jump};
use super::types::{ReplayAction, ReplayCollectStartArgs, ReplayCollectState, ReplayCollectStates};

pub(super) fn replay_collect_start(
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
    let collector = node.output.slot().map_or(args.source, |slot| slot);
    if items.is_empty() {
        write_empty_collect_page(run, store, collector, source_taint)?;
        states.remove(collector);
        return replay_jump(run, args.done);
    }
    let page = collect_page_items(items, 0, page_size)?;
    let current_page = write_collect_page(
        run,
        store,
        collector,
        &page,
        source_taint,
        Some((list_id, 0)),
    )?;
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

pub(super) fn replay_collect_page(
    run: &mut RunFrame,
    collector_slot: SlotIdx,
    body: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    validate_collect_slot_list(run, collector_slot)?;
    replay_jump(run, body)
}

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
        run.write_slot_with_taint(collector_slot, SlotValue::List(state.source), state.taint)
            .map_err(engine_to_replay_err)?;
        states.remove(collector_slot);
        return replay_jump(run, done);
    }
    let page = collect_page_items(source_items, state.cursor, state.page_size)?;
    let current_page = write_collect_page(
        run,
        store,
        collector_slot,
        &page,
        state.taint,
        Some((state.source, state.cursor)),
    )?;
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

pub(super) fn replay_collect_finish(
    run: &mut RunFrame,
    states: &mut ReplayCollectStates,
    node: &CompiledNode,
    collector_slot: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run
        .read_slot(collector_slot)
        .map_err(engine_to_replay_err)?;
    let taint = run
        .read_taint(collector_slot)
        .map_err(engine_to_replay_err)?;
    let output = node.output.slot().ok_or(ReplayError::Internal {
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
    source: Option<(ListId, usize)>,
) -> Result<ListId, ReplayError> {
    let taints = if let Some((source, start)) = source {
        store
            .list_items_with_taint(source)
            .map_err(|_| ReplayError::Internal {
                reason: "collect source taints missing during replay",
            })?
            .skip(start)
            .take(items.len())
            .map(|(_, taint)| taint)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    } else {
        Box::from([])
    };
    let page_id = store
        .insert_list_with_taint(items.to_vec().into_boxed_slice(), taints)
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
    write_collect_page(run, store, collector, &[], taint, None).map(|_page_id| ())
}
