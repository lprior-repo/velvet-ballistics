//! Collect pagination state management.

use std::sync::{Mutex, MutexGuard};

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, RunId, SlotIdx};

const MAX_COLLECT_PAGINATION_STATES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CollectPaginationState {
    pub(super) frame_key: usize,
    pub(super) run_id: RunId,
    pub(super) collector_slot: SlotIdx,
    pub(super) source: ListId,
    pub(super) current_page: ListId,
    pub(super) cursor: usize,
    pub(super) page_size: usize,
    pub(super) item_count: usize,
    pub(super) limit: usize,
}

static COLLECT_PAGINATION_STATES: Mutex<Vec<CollectPaginationState>> = Mutex::new(Vec::new());

pub(super) fn collect_frame_key(run: &RunFrame) -> usize {
    std::ptr::from_ref(run).addr()
}

pub(super) fn lock_collect_states(
) -> Result<MutexGuard<'static, Vec<CollectPaginationState>>, EngineError> {
    COLLECT_PAGINATION_STATES
        .lock()
        .map_err(|_| EngineError::InternalInvariantViolation {
            reason: "collect pagination state lock poisoned",
        })
}

pub(super) fn upsert_collect_state(state: CollectPaginationState) -> Result<(), EngineError> {
    let mut states = lock_collect_states()?;
    let mut index = 0usize;
    while index < states.len() {
        let existing = states
            .get_mut(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect state index checked by loop bound",
            })?;
        if existing.frame_key == state.frame_key
            && existing.run_id == state.run_id
            && existing.collector_slot == state.collector_slot
        {
            *existing = state;
            return Ok(());
        }
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect state index overflow",
            })?;
    }
    if states.len() >= MAX_COLLECT_PAGINATION_STATES {
        return Err(EngineError::ResourceLimitExceeded {
            resource: "collect_pagination_states",
        });
    }
    states.push(state);
    Ok(())
}

pub(super) fn find_collect_state(
    run: &RunFrame,
    collector_slot: SlotIdx,
    current_page: ListId,
) -> Result<CollectPaginationState, EngineError> {
    let frame_key = collect_frame_key(run);
    let run_id = run.run_id();
    let states = lock_collect_states()?;
    let mut index = 0usize;
    while index < states.len() {
        let st = states
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect state index checked by loop bound",
            })?;
        if st.frame_key == frame_key
            && st.run_id == run_id
            && st.collector_slot == collector_slot
            && st.current_page == current_page
        {
            return Ok(*st);
        }
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "collect state index overflow",
            })?;
    }
    Err(EngineError::InvalidCompiledWorkflow {
        reason: "collect pagination state missing",
    })
}

pub(super) fn remove_collect_state(run: &RunFrame, collector_slot: SlotIdx) {
    let frame_key = collect_frame_key(run);
    let run_id = run.run_id();
    let Ok(mut states) = COLLECT_PAGINATION_STATES.lock() else {
        return;
    };
    let mut read = 0usize;
    let mut write = 0usize;
    while read < states.len() {
        let Some(st) = states.get(read).copied() else {
            return;
        };
        if st.frame_key != frame_key || st.run_id != run_id || st.collector_slot != collector_slot {
            if write != read {
                if let Some(target) = states.get_mut(write) {
                    *target = st;
                } else {
                    return;
                }
            }
            let Some(next_write) = write.checked_add(1) else {
                return;
            };
            write = next_write;
        }
        let Some(next_read) = read.checked_add(1) else {
            return;
        };
        read = next_read;
    }
    states.truncate(write);
}
