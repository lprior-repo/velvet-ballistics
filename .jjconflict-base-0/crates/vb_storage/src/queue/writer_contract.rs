#![forbid(unsafe_code)]

/// Pure queue transition outcome shared by JournalWriterQueue and Verus route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StorageQueueStepResult {
    /// Queue drain completed with no pending items.
    Drained,
    /// Strict batch removal was attempted before durable persistence.
    StrictPersistFailed,
    /// Queue drain ended with pending items remaining.
    DrainIncomplete,
    /// Shutdown and durable close are allowed after an empty drain.
    ClosedAfterEmpty,
}

/// Primitive, side-effect-free queue state used by the durable writer shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StorageQueueDecisionState {
    pub pending_len: usize,
    pub capacity: usize,
    pub batch_size: usize,
    pub shutdown: bool,
    pub has_strict: bool,
    pub persisted: bool,
}

/// Pure acceptance decision for the in-memory writer queue.
#[must_use]
pub(super) const fn enqueue_allowed(state: StorageQueueDecisionState) -> StorageQueueStepResult {
    if state.shutdown {
        return StorageQueueStepResult::DrainIncomplete;
    }
    if state.pending_len >= state.capacity {
        return StorageQueueStepResult::DrainIncomplete;
    }
    StorageQueueStepResult::Drained
}

/// Pure strict batch removal decision used by the writer queue shell.
#[must_use]
pub(super) const fn strict_batch_remove_decision(
    state: StorageQueueDecisionState,
) -> StorageQueueStepResult {
    if strict_removal_allowed(state.has_strict, state.persisted) {
        return StorageQueueStepResult::Drained;
    }
    StorageQueueStepResult::StrictPersistFailed
}

/// Pure finish-drain decision used by the writer queue shell.
#[must_use]
pub(super) const fn finish_drain_decision(
    state: StorageQueueDecisionState,
) -> StorageQueueStepResult {
    if state.pending_len == 0 {
        return StorageQueueStepResult::Drained;
    }
    StorageQueueStepResult::DrainIncomplete
}

/// Pure close-barrier decision used before Fjall close invocation.
#[must_use]
pub(super) const fn shutdown_and_close_decision(
    state: StorageQueueDecisionState,
) -> StorageQueueStepResult {
    if state.shutdown && state.pending_len == 0 {
        return StorageQueueStepResult::ClosedAfterEmpty;
    }
    StorageQueueStepResult::DrainIncomplete
}

/// Pure static loop-bound decision for draining a bounded queue.
#[must_use]
pub(super) const fn drain_iteration_bound(state: StorageQueueDecisionState) -> Option<usize> {
    match state.capacity.checked_div(state.batch_size) {
        Some(base) => base.checked_add(2),
        None => None,
    }
}

/// Shared pure predicate for queue emptiness after drain.
#[must_use]
pub(super) const fn queue_is_empty(pending: usize) -> bool {
    pending == 0
}

/// Shared pure predicate for strict-item removal after persistence.
#[must_use]
pub(super) const fn strict_removal_allowed(has_strict: bool, persisted: bool) -> bool {
    !has_strict || persisted
}

/// Shared pure finish-drain decision used by the writer queue shell.
#[must_use]
pub(super) const fn finish_drain_status(pending_after_drain: usize) -> StorageQueueStepResult {
    finish_drain_decision(StorageQueueDecisionState {
        pending_len: pending_after_drain,
        capacity: 0,
        batch_size: 1,
        shutdown: false,
        has_strict: false,
        persisted: false,
    })
}

/// Shared pure strict batch-removal decision used by the writer queue shell.
#[must_use]
pub(super) const fn strict_batch_remove_status(
    has_strict: bool,
    persisted: bool,
) -> StorageQueueStepResult {
    strict_batch_remove_decision(StorageQueueDecisionState {
        pending_len: 0,
        capacity: 0,
        batch_size: 1,
        shutdown: false,
        has_strict,
        persisted,
    })
}

/// Shared pure shutdown-and-close decision used before Fjall close invocation.
#[must_use]
pub(super) const fn shutdown_and_close_status(
    shutdown: bool,
    pending_after_drain: usize,
) -> StorageQueueStepResult {
    shutdown_and_close_decision(StorageQueueDecisionState {
        pending_len: pending_after_drain,
        capacity: 0,
        batch_size: 1,
        shutdown,
        has_strict: false,
        persisted: false,
    })
}
