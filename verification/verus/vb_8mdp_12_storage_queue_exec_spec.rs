use vstd::prelude::*;

#[path = "../../crates/vb_storage/src/queue/writer_contract.rs"]
mod production_writer_contract;

const fn const_bool_len(value: bool) -> usize {
    if value {
        return 1;
    }
    0
}

const fn source_queue_empty(pending: usize) -> bool {
    production_writer_contract::queue_is_empty(pending)
}

const fn source_strict_removal_allowed(has_strict: bool, persisted: bool) -> bool {
    production_writer_contract::strict_removal_allowed(has_strict, persisted)
}

const fn source_finish_drain_drained(pending_after_drain: usize) -> bool {
    matches!(
        production_writer_contract::finish_drain_status(pending_after_drain),
        production_writer_contract::StorageQueueStepResult::Drained
    )
}

const fn source_finish_drain_incomplete(pending_after_drain: usize) -> bool {
    matches!(
        production_writer_contract::finish_drain_status(pending_after_drain),
        production_writer_contract::StorageQueueStepResult::DrainIncomplete
    )
}

const fn source_strict_batch_status_matches(has_strict: bool, persisted: bool) -> bool {
    match production_writer_contract::strict_batch_remove_status(has_strict, persisted) {
        production_writer_contract::StorageQueueStepResult::Drained => !has_strict || persisted,
        production_writer_contract::StorageQueueStepResult::StrictPersistFailed => {
            has_strict && !persisted
        }
        production_writer_contract::StorageQueueStepResult::DrainIncomplete
        | production_writer_contract::StorageQueueStepResult::ClosedAfterEmpty => false,
    }
}

const fn source_shutdown_close_matches(shutdown: bool, pending_after_drain: usize) -> bool {
    match production_writer_contract::shutdown_and_close_status(shutdown, pending_after_drain) {
        production_writer_contract::StorageQueueStepResult::ClosedAfterEmpty => {
            shutdown && production_writer_contract::queue_is_empty(pending_after_drain)
        }
        production_writer_contract::StorageQueueStepResult::DrainIncomplete => {
            !(shutdown && production_writer_contract::queue_is_empty(pending_after_drain))
        }
        production_writer_contract::StorageQueueStepResult::Drained
        | production_writer_contract::StorageQueueStepResult::StrictPersistFailed => false,
    }
}

const fn source_kernel_acceptance_matches(
    pending_len: usize,
    capacity: usize,
    batch_size: usize,
    shutdown: bool,
) -> bool {
    let state = production_writer_contract::StorageQueueDecisionState {
        pending_len,
        capacity,
        batch_size,
        shutdown,
        has_strict: false,
        persisted: false,
    };
    match production_writer_contract::enqueue_allowed(state) {
        production_writer_contract::StorageQueueStepResult::Drained => {
            !shutdown && pending_len < capacity
        }
        production_writer_contract::StorageQueueStepResult::DrainIncomplete => {
            shutdown || pending_len >= capacity
        }
        production_writer_contract::StorageQueueStepResult::StrictPersistFailed
        | production_writer_contract::StorageQueueStepResult::ClosedAfterEmpty => false,
    }
}

const fn source_kernel_bound_matches(capacity: usize, batch_size: usize) -> bool {
    let state = production_writer_contract::StorageQueueDecisionState {
        pending_len: 0,
        capacity,
        batch_size,
        shutdown: true,
        has_strict: false,
        persisted: false,
    };
    match production_writer_contract::drain_iteration_bound(state) {
        Some(bound) => batch_size != 0 && bound >= 2,
        None => batch_size == 0,
    }
}

const fn source_kernel_matches_bounded_domain(max: usize) -> bool {
    let mut pending = 0usize;
    while pending <= max {
        if !source_finish_drain_drained(pending) && pending == 0 {
            return false;
        }
        if !source_finish_drain_incomplete(pending) && pending > 0 {
            return false;
        }
        if !source_shutdown_close_matches(true, pending) {
            return false;
        }
        if !source_shutdown_close_matches(false, pending) {
            return false;
        }

        let mut capacity = 0usize;
        while capacity <= max {
            if !source_kernel_acceptance_matches(pending, capacity, 1, false) {
                return false;
            }
            if !source_kernel_acceptance_matches(pending, capacity, 1, true) {
                return false;
            }

            let mut batch_size = 1usize;
            while batch_size <= max {
                if !source_kernel_bound_matches(capacity, batch_size) {
                    return false;
                }
                batch_size += 1;
            }
            capacity += 1;
        }
        pending += 1;
    }

    source_kernel_bound_matches(max, 0)
        && source_strict_batch_status_matches(false, false)
        && source_strict_batch_status_matches(false, true)
        && source_strict_batch_status_matches(true, false)
        && source_strict_batch_status_matches(true, true)
}

const _: [(); 1] = [(); const_bool_len(source_queue_empty(0))];
const _: [(); 1] = [(); const_bool_len(!source_queue_empty(1))];
const _: [(); 1] = [(); const_bool_len(source_finish_drain_drained(0))];
const _: [(); 1] = [(); const_bool_len(source_finish_drain_incomplete(1))];
const _: [(); 1] = [(); const_bool_len(source_strict_removal_allowed(false, false))];
const _: [(); 1] = [(); const_bool_len(source_strict_removal_allowed(true, true))];
const _: [(); 1] = [(); const_bool_len(!source_strict_removal_allowed(true, false))];
const _: [(); 1] = [(); const_bool_len(source_strict_batch_status_matches(false, false))];
const _: [(); 1] = [(); const_bool_len(source_strict_batch_status_matches(true, true))];
const _: [(); 1] = [(); const_bool_len(source_strict_batch_status_matches(true, false))];
const _: [(); 1] = [(); const_bool_len(source_shutdown_close_matches(true, 0))];
const _: [(); 1] = [(); const_bool_len(source_shutdown_close_matches(true, 1))];
const _: [(); 1] = [(); const_bool_len(source_shutdown_close_matches(false, 0))];
const _: [(); 1] = [(); const_bool_len(source_kernel_acceptance_matches(0, 1, 1, false))];
const _: [(); 1] = [(); const_bool_len(source_kernel_acceptance_matches(1, 1, 1, false))];
const _: [(); 1] = [(); const_bool_len(source_kernel_acceptance_matches(0, 1, 1, true))];
const _: [(); 1] = [(); const_bool_len(source_kernel_bound_matches(4, 2))];
const _: [(); 1] = [(); const_bool_len(source_kernel_bound_matches(0, 0))];
const _: [(); 1] = [(); const_bool_len(source_kernel_matches_bounded_domain(4))];

verus! {

// Obligations: PO-vb-8mdp.12-VERUS-003A-PROD-JOURNAL-WRITER-QUEUE,
// PO-vb-8mdp.12-VERUS-003A-STORAGE-QUEUE-EXEC-SPEC,
// PO-vb-8mdp.12-VERUS-ARCH-003, PO-vb-8mdp.12-VERUS-ARCH-005.
//
// Production binding route: crates/vb_storage/src/queue/writer.rs delegates its
// queue-empty, finish-drain, strict-removal, and shutdown-close decisions to the
// shared pure module crates/vb_storage/src/queue/writer_contract.rs. This Verus
// artifact includes that exact production module above and uses const assertions
// to exhaustively check the imported production kernel for the approved finite
// model domain pending/capacity <= 4, batch_size 1..=4 plus divide-by-zero None.
// The Verus specs below prove the same first-party transition contract. This
// does not prove Mutex, VecDeque, Fjall, or filesystem physical durability.

pub enum StorageStepResult {
    Drained,
    StrictPersistFailed,
    DrainIncomplete,
    ClosedAfterEmpty,
}

pub struct QueueDecisionState {
    pub pending_len: usize,
    pub capacity: usize,
    pub batch_size: usize,
    pub shutdown: bool,
    pub has_strict: bool,
    pub persisted: bool,
}

pub open spec fn storage_queue_is_empty_spec(pending: int) -> bool {
    pending == 0
}

pub open spec fn storage_strict_removal_allowed_spec(has_strict: bool, persisted: bool) -> bool {
    !has_strict || persisted
}

pub open spec fn finish_drain_spec(pending_after_drain: int) -> StorageStepResult {
    if storage_queue_is_empty_spec(pending_after_drain) {
        StorageStepResult::Drained
    } else {
        StorageStepResult::DrainIncomplete
    }
}

pub open spec fn strict_batch_remove_spec(has_strict: bool, persisted: bool) -> StorageStepResult {
    if storage_strict_removal_allowed_spec(has_strict, persisted) {
        StorageStepResult::Drained
    } else {
        StorageStepResult::StrictPersistFailed
    }
}

pub open spec fn shutdown_and_close_spec(shutdown: bool, pending_after_drain: int) -> StorageStepResult {
    if shutdown && storage_queue_is_empty_spec(pending_after_drain) {
        StorageStepResult::ClosedAfterEmpty
    } else {
        StorageStepResult::DrainIncomplete
    }
}

pub open spec fn enqueue_allowed_spec(pending_len: int, capacity: int, shutdown: bool) -> StorageStepResult {
    if !shutdown && pending_len < capacity {
        StorageStepResult::Drained
    } else {
        StorageStepResult::DrainIncomplete
    }
}

pub open spec fn drain_iteration_bound_spec(capacity: int, batch_size: int) -> int {
    capacity / batch_size + 2
}

pub fn enqueue_allowed_exec(state: QueueDecisionState) -> (result: StorageStepResult)
    requires
        state.pending_len <= 4,
        state.capacity <= 4,
    ensures
        result == enqueue_allowed_spec(state.pending_len as int, state.capacity as int, state.shutdown),
        result == StorageStepResult::Drained ==> !state.shutdown && state.pending_len < state.capacity,
        result == StorageStepResult::DrainIncomplete ==> state.shutdown || state.pending_len >= state.capacity,
{
    if state.shutdown {
        return StorageStepResult::DrainIncomplete;
    }
    if state.pending_len >= state.capacity {
        return StorageStepResult::DrainIncomplete;
    }
    StorageStepResult::Drained
}

pub fn strict_batch_remove_exec(state: QueueDecisionState) -> (result: StorageStepResult)
    ensures
        result == strict_batch_remove_spec(state.has_strict, state.persisted),
        result == StorageStepResult::StrictPersistFailed ==> state.has_strict && !state.persisted,
        result == StorageStepResult::Drained ==> !state.has_strict || state.persisted,
{
    if !state.has_strict || state.persisted {
        return StorageStepResult::Drained;
    }
    StorageStepResult::StrictPersistFailed
}

pub fn finish_drain_exec(state: QueueDecisionState) -> (result: StorageStepResult)
    requires
        state.pending_len <= 4,
    ensures
        result == finish_drain_spec(state.pending_len as int),
        result == StorageStepResult::Drained ==> state.pending_len == 0,
        result == StorageStepResult::DrainIncomplete ==> state.pending_len > 0,
{
    if state.pending_len == 0 {
        return StorageStepResult::Drained;
    }
    StorageStepResult::DrainIncomplete
}

pub fn shutdown_and_close_exec(state: QueueDecisionState) -> (result: StorageStepResult)
    requires
        state.pending_len <= 4,
    ensures
        result == shutdown_and_close_spec(state.shutdown, state.pending_len as int),
        result == StorageStepResult::ClosedAfterEmpty ==> state.shutdown && state.pending_len == 0,
        result == StorageStepResult::DrainIncomplete ==> !state.shutdown || state.pending_len > 0,
{
    if state.shutdown && state.pending_len == 0 {
        return StorageStepResult::ClosedAfterEmpty;
    }
    StorageStepResult::DrainIncomplete
}

pub proof fn proof_queue_empty_hook_matches_pending_len(pending: int)
    requires
        0 <= pending,
        pending <= 4,
    ensures
        storage_queue_is_empty_spec(pending) <==> pending == 0,
        !storage_queue_is_empty_spec(pending) <==> pending > 0,
{
}

pub proof fn proof_finish_drain_fails_closed_on_pending(pending_after_drain: int)
    requires
        0 <= pending_after_drain,
        pending_after_drain <= 4,
    ensures
        pending_after_drain == 0 ==> finish_drain_spec(pending_after_drain) == StorageStepResult::Drained,
        pending_after_drain > 0 ==> finish_drain_spec(pending_after_drain) == StorageStepResult::DrainIncomplete,
{
}

pub proof fn proof_strict_removal_requires_persist(has_strict: bool, persisted: bool)
    ensures
        has_strict && !persisted ==> strict_batch_remove_spec(has_strict, persisted) == StorageStepResult::StrictPersistFailed,
        has_strict && persisted ==> strict_batch_remove_spec(has_strict, persisted) == StorageStepResult::Drained,
        !has_strict ==> strict_batch_remove_spec(has_strict, persisted) == StorageStepResult::Drained,
{
}

pub proof fn proof_close_requires_shutdown_and_empty(pending_after_drain: int)
    requires
        0 <= pending_after_drain,
        pending_after_drain <= 4,
    ensures
        shutdown_and_close_spec(true, pending_after_drain) == StorageStepResult::ClosedAfterEmpty ==> pending_after_drain == 0,
        pending_after_drain == 0 ==> shutdown_and_close_spec(true, pending_after_drain) == StorageStepResult::ClosedAfterEmpty,
        pending_after_drain > 0 ==> shutdown_and_close_spec(true, pending_after_drain) == StorageStepResult::DrainIncomplete,
        shutdown_and_close_spec(false, 0) == StorageStepResult::DrainIncomplete,
{
}

pub proof fn proof_storage_queue_exec_spec_bundle(pending_after_drain: int, has_strict: bool, persisted: bool)
    requires
        0 <= pending_after_drain,
        pending_after_drain <= 4,
    ensures
        storage_queue_is_empty_spec(pending_after_drain) <==> pending_after_drain == 0,
        has_strict && !persisted ==> strict_batch_remove_spec(has_strict, persisted) == StorageStepResult::StrictPersistFailed,
        pending_after_drain > 0 ==> finish_drain_spec(pending_after_drain) == StorageStepResult::DrainIncomplete,
        shutdown_and_close_spec(true, pending_after_drain) == StorageStepResult::ClosedAfterEmpty ==> pending_after_drain == 0,
{
    proof_queue_empty_hook_matches_pending_len(pending_after_drain);
    proof_strict_removal_requires_persist(has_strict, persisted);
    proof_finish_drain_fails_closed_on_pending(pending_after_drain);
    proof_close_requires_shutdown_and_empty(pending_after_drain);
}

}
