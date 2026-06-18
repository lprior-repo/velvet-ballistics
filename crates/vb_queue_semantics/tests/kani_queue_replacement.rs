#![cfg(kani)]
#![allow(unused_crate_dependencies)]

//! Production-bound Kani replacement proofs for `vb-dzibx` queue semantics.
//!
//! Obligations: PO-VB-DZIBX-QS-KANI-001 through PO-VB-DZIBX-QS-KANI-010 from
//! `.beads/vb-dzibx/proof-obligations.planned.jsonl`.
//!
//! The harnesses below call `vb_queue_semantics` production functions and
//! production types directly. They intentionally do not define mirror decision
//! structs/enums, copied queue-state structs, or local Verus-style model types.

use std::collections::VecDeque;
use vb_queue_semantics::{
    CapacityRejection, EnqueueDecision, PopDecision, PopTransition, QueueState,
    RuntimeQueueSurface, ShardTickTransition, action_dequeue_transition,
    action_enqueue_transition, command_enqueue_transition, command_pop_transition,
    command_pop_transition_decision, enqueue_decision, helper_command_pop_is_pop_front,
    helper_enqueue_accepts, helper_queue_is_full, helper_runtime_queue_full_maps,
    helper_shard_tick_is_pop_front, helper_valid_capacity, queue_is_full,
    remaining_capacity, runtime_queue_full_error_transition, shard_tick_transition,
    shard_tick_transition_decision, validate_capacity, warning_payload, warning_threshold,
    SHARED_QUEUE_CAPACITY_MAX,
};

const MAX_SYMBOLIC_QUEUE_CAPACITY: usize = 4;

fn any_runtime_queue_surface() -> RuntimeQueueSurface {
    let selector: u8 = kani::any();
    match selector % 4 {
        0 => RuntimeQueueSurface::Submit,
        1 => RuntimeQueueSurface::Cancel,
        2 => RuntimeQueueSurface::Resume,
        _ => RuntimeQueueSurface::Inspect,
    }
}

fn bounded_queue_state_u8() -> Option<QueueState<u8>> {
    let capacity: usize = kani::any();
    let len: usize = kani::any();

    kani::assume(capacity > 0);
    kani::assume(capacity <= MAX_SYMBOLIC_QUEUE_CAPACITY);
    kani::assume(len <= capacity);

    let item0: u8 = kani::any();
    let item1: u8 = kani::any();
    let item2: u8 = kani::any();
    let item3: u8 = kani::any();

    let mut items = VecDeque::new();
    if len >= 1 {
        items.push_back(item0);
    }
    if len >= 2 {
        items.push_back(item1);
    }
    if len >= 3 {
        items.push_back(item2);
    }
    if len >= 4 {
        items.push_back(item3);
    }

    QueueState::from_vec_deque(capacity, MAX_SYMBOLIC_QUEUE_CAPACITY, items).ok()
}

fn check_imported_state_observations(state: &QueueState<u8>, expected_len: usize) {
    let capacity = state.capacity();
    kani::assert(capacity > 0, "bounded imported state has positive capacity");
    kani::assert(
        capacity <= MAX_SYMBOLIC_QUEUE_CAPACITY,
        "bounded imported state capacity stays within harness bound",
    );
    kani::assert(state.len() == expected_len, "bounded imported state preserves length");
    kani::assert(state.len() <= capacity, "bounded imported state length is within capacity");
    kani::assert(
        state.is_empty() == (expected_len == 0),
        "QueueState::is_empty matches zero length",
    );
    kani::assert(
        state.is_full() == queue_is_full(capacity, expected_len),
        "QueueState::is_full delegates to production full helper",
    );
}

/// PO-VB-DZIBX-QS-KANI-001.
#[kani::proof]
fn po_vb_dzibx_qs_kani_001_capacity_validation_and_shared_helper() {
    let capacity: usize = kani::any();
    let maximum: usize = kani::any();
    let helper_capacity: usize = kani::any();

    kani::cover!(capacity == 0, "PO-001 domain reaches zero-capacity rejection");
    kani::cover!(
        capacity > 0 && capacity <= maximum,
        "PO-001 domain reaches accepted caller capacity"
    );
    kani::cover!(
        capacity > maximum,
        "PO-001 domain reaches above-maximum rejection"
    );
    kani::cover!(
        helper_capacity == SHARED_QUEUE_CAPACITY_MAX,
        "PO-001 helper domain reaches shared maximum"
    );
    kani::cover!(
        helper_capacity == SHARED_QUEUE_CAPACITY_MAX + 1,
        "PO-001 helper domain reaches one above shared maximum"
    );

    match validate_capacity(capacity, maximum) {
        Ok(()) => {
            kani::assert(capacity > 0, "validate_capacity Ok has nonzero capacity");
            kani::assert(
                capacity <= maximum,
                "validate_capacity Ok is within caller maximum",
            );
        }
        Err(CapacityRejection::Zero) => {
            kani::assert(capacity == 0, "zero rejection is exact");
        }
        Err(CapacityRejection::AboveMaximum { maximum: observed }) => {
            kani::assert(capacity > 0, "above-maximum rejection is not zero branch");
            kani::assert(capacity > maximum, "above-maximum rejection requires cap > max");
            kani::assert(observed == maximum, "above-maximum rejection preserves maximum");
        }
    }

    kani::assert(
        helper_valid_capacity(helper_capacity)
            == (helper_capacity > 0 && helper_capacity <= SHARED_QUEUE_CAPACITY_MAX),
        "helper_valid_capacity matches shared bounded domain",
    );
}

/// PO-VB-DZIBX-QS-KANI-002.
#[kani::proof]
fn po_vb_dzibx_qs_kani_002_remaining_capacity_saturates() {
    let capacity: usize = kani::any();
    let len: usize = kani::any();

    kani::cover!(len == 0, "PO-002 domain reaches empty queue observation");
    kani::cover!(len == capacity, "PO-002 domain reaches exact full boundary");
    kani::cover!(len > capacity, "PO-002 domain reaches saturating branch");
    kani::cover!(
        capacity == usize::MAX && len == usize::MAX,
        "PO-002 domain reaches usize::MAX equality boundary"
    );

    let remaining = remaining_capacity(capacity, len);
    if len <= capacity {
        kani::assert(
            remaining == capacity - len,
            "remaining_capacity subtracts when len <= capacity",
        );
    } else {
        kani::assert(remaining == 0, "remaining_capacity saturates to zero");
    }
    kani::assert(remaining <= capacity, "remaining capacity never exceeds capacity");
}

/// PO-VB-DZIBX-QS-KANI-003.
#[kani::proof]
fn po_vb_dzibx_qs_kani_003_full_accept_and_enqueue_decision() {
    let capacity: usize = kani::any();
    let len: usize = kani::any();

    kani::cover!(len < capacity, "PO-003 domain reaches enqueue accepted branch");
    kani::cover!(len == capacity, "PO-003 domain reaches exact full branch");
    kani::cover!(len > capacity, "PO-003 domain reaches over-capacity full branch");
    kani::cover!(
        capacity == 0 && len == 0,
        "PO-003 domain reaches zero-capacity full branch"
    );

    let expected_full = len >= capacity;
    kani::assert(
        queue_is_full(capacity, len) == expected_full,
        "queue_is_full matches len >= capacity",
    );
    kani::assert(
        helper_queue_is_full(capacity, len) == expected_full,
        "helper_queue_is_full matches len >= capacity",
    );
    kani::assert(
        helper_enqueue_accepts(capacity, len) == !expected_full,
        "helper_enqueue_accepts is exact negation of full",
    );

    match enqueue_decision(capacity, len) {
        EnqueueDecision::Accepted => {
            kani::assert(len < capacity, "accepted enqueue decision requires len < capacity");
        }
        EnqueueDecision::QueueFull { capacity: observed } => {
            kani::assert(len >= capacity, "QueueFull decision requires len >= capacity");
            kani::assert(observed == capacity, "QueueFull decision preserves capacity");
        }
    }
}

/// PO-VB-DZIBX-QS-KANI-004.
#[kani::proof]
fn po_vb_dzibx_qs_kani_004_pop_decision_helpers() {
    let capacity: usize = kani::any();
    let len: usize = kani::any();

    kani::cover!(
        capacity > 0 && len == 0,
        "PO-004 domain reaches nonzero-capacity empty branch"
    );
    kani::cover!(
        capacity > 0 && len > 0,
        "PO-004 domain reaches PopFront branch"
    );
    kani::cover!(
        capacity == 0 && len > 0,
        "PO-004 domain reaches zero-capacity nonempty observation"
    );

    let expected_pop = len > 0 && capacity > 0;
    kani::assert(
        helper_command_pop_is_pop_front(capacity, len) == expected_pop,
        "command pop helper matches capacity/len predicate",
    );
    kani::assert(
        helper_shard_tick_is_pop_front(capacity, len) == expected_pop,
        "shard tick helper matches command pop helper predicate",
    );
    kani::assert(
        command_pop_transition_decision(capacity, len)
            == if expected_pop {
                PopDecision::PopFront
            } else {
                PopDecision::Empty
            },
        "command pop decision matches production helper predicate",
    );
    kani::assert(
        shard_tick_transition_decision(capacity, len)
            == if expected_pop {
                PopDecision::PopFront
            } else {
                PopDecision::Empty
            },
        "shard tick decision matches production helper predicate",
    );
}

/// PO-VB-DZIBX-QS-KANI-005.
#[kani::proof]
fn po_vb_dzibx_qs_kani_005_warning_threshold_overflow_branch() {
    let capacity: usize = kani::any();

    kani::cover!(capacity == 0, "PO-005 domain reaches minimum clamp from zero");
    kani::cover!(capacity == 1, "PO-005 domain reaches minimum clamp from one");
    kani::cover!(capacity == 10, "PO-005 domain reaches exact 80 percent example");
    kani::cover!(
        capacity > usize::MAX / 8,
        "PO-005 domain reaches checked_mul overflow branch"
    );

    let threshold = warning_threshold(capacity);
    match capacity.checked_mul(8) {
        Some(scaled) => {
            let rounded_down = scaled / 10;
            let expected = if rounded_down == 0 { 1 } else { rounded_down };
            kani::assert(
                threshold == expected,
                "warning_threshold follows checked multiply success branch",
            );
            kani::assert(threshold >= 1, "non-overflow threshold is clamped to at least one");
            if capacity > 0 {
                kani::assert(
                    threshold <= capacity,
                    "non-overflow positive threshold does not exceed capacity",
                );
            }
        }
        None => {
            kani::assert(
                threshold == capacity,
                "warning_threshold returns capacity on checked_mul overflow",
            );
        }
    }
}

/// PO-VB-DZIBX-QS-KANI-006.
#[kani::proof]
fn po_vb_dzibx_qs_kani_006_warning_payload_boundaries() {
    let capacity: usize = kani::any();
    let depth: usize = kani::any();

    let threshold = warning_threshold(capacity);

    kani::cover!(depth < threshold, "PO-006 domain reaches below-threshold None branch");
    kani::cover!(
        depth >= threshold && depth <= capacity,
        "PO-006 domain reaches payload-present branch"
    );
    kani::cover!(depth > capacity, "PO-006 domain reaches over-capacity None branch");
    kani::cover!(
        capacity > usize::MAX / 8 && depth == capacity,
        "PO-006 domain reaches overflow-threshold payload boundary"
    );

    let expected_payload = depth >= threshold && depth <= capacity;
    match warning_payload(capacity, depth) {
        Some(payload) => {
            kani::assert(expected_payload, "warning_payload Some only inside threshold/capacity window");
            kani::assert(payload.depth == depth, "warning_payload preserves depth field");
            kani::assert(
                payload.capacity == capacity,
                "warning_payload preserves capacity field",
            );
        }
        None => {
            kani::assert(!expected_payload, "warning_payload None outside threshold/capacity window");
        }
    }
}

/// PO-VB-DZIBX-QS-KANI-007.
#[kani::proof]
fn po_vb_dzibx_qs_kani_007_runtime_queue_full_transition() {
    let depth: usize = kani::any();
    let capacity: usize = kani::any();
    let surface = any_runtime_queue_surface();

    kani::cover!(
        surface == RuntimeQueueSurface::Submit,
        "PO-007 domain reaches Submit surface"
    );
    kani::cover!(
        surface == RuntimeQueueSurface::Cancel,
        "PO-007 domain reaches Cancel surface"
    );
    kani::cover!(
        surface == RuntimeQueueSurface::Resume,
        "PO-007 domain reaches Resume surface"
    );
    kani::cover!(
        surface == RuntimeQueueSurface::Inspect,
        "PO-007 domain reaches Inspect surface"
    );
    kani::cover!(
        depth < capacity,
        "PO-007 domain reaches not-full transition absence"
    );
    kani::cover!(
        depth >= capacity,
        "PO-007 domain reaches queue-full transition presence"
    );

    kani::assert(
        helper_runtime_queue_full_maps(depth, capacity) == (depth >= capacity),
        "runtime full helper matches depth >= capacity",
    );
    match runtime_queue_full_error_transition(depth, capacity, surface) {
        Some(transition) => {
            kani::assert(depth >= capacity, "queue-full transition exists only when full");
            kani::assert(transition.surface == surface, "queue-full transition preserves surface");
            kani::assert(transition.capacity == capacity, "queue-full transition preserves capacity");
            kani::assert(transition.depth == depth, "queue-full transition preserves depth");
            kani::assert(
                transition.rejected_without_admission,
                "queue-full transition records rejection without admission",
            );
        }
        None => {
            kani::assert(depth < capacity, "queue-full transition absent only below capacity");
        }
    }
}

/// PO-VB-DZIBX-QS-KANI-008.
#[kani::proof]
#[kani::unwind(24)]
fn po_vb_dzibx_qs_kani_008_bounded_state_import_observations() {
    let maybe_state = bounded_queue_state_u8();
    kani::assert(
        maybe_state.is_some(),
        "bounded QueueState generator imports valid concrete queue",
    );
    if let Some(state) = maybe_state {
        let expected_len = state.len();

        kani::cover!(expected_len == 0, "PO-008 domain reaches empty imported state");
        kani::cover!(
            expected_len == state.capacity(),
            "PO-008 domain reaches full imported state"
        );
        kani::cover!(
            state.capacity() == MAX_SYMBOLIC_QUEUE_CAPACITY,
            "PO-008 domain reaches maximum bounded capacity"
        );

        check_imported_state_observations(&state, expected_len);
    }
}

/// PO-VB-DZIBX-QS-KANI-009.
#[kani::proof]
#[kani::unwind(24)]
fn po_vb_dzibx_qs_kani_009_bounded_enqueue_transitions() {
    let maybe_action_state = bounded_queue_state_u8();
    kani::assert(
        maybe_action_state.is_some(),
        "bounded action enqueue generator imports valid state",
    );
    if let Some(action_state) = maybe_action_state {
        let before_len = action_state.len();
        let capacity = action_state.capacity();
        let item: u8 = kani::any();

        kani::cover!(before_len < capacity, "PO-009 domain reaches accepted enqueue");
        kani::cover!(before_len == capacity, "PO-009 domain reaches full enqueue rejection");

        let (after, decision) = action_enqueue_transition(action_state, item);
        kani::assert(after.capacity() == capacity, "action enqueue preserves capacity");
        match decision {
            EnqueueDecision::Accepted => {
                kani::assert(before_len < capacity, "action enqueue accepts only below capacity");
                kani::assert(after.len() == before_len + 1, "action enqueue increments length by one");
            }
            EnqueueDecision::QueueFull { capacity: observed } => {
                kani::assert(before_len >= capacity, "action enqueue rejects only full queues");
                kani::assert(observed == capacity, "action enqueue QueueFull preserves capacity");
                kani::assert(after.len() == before_len, "action enqueue full rejection preserves length");
            }
        }
        kani::assert(after.len() <= after.capacity(), "action enqueue preserves bounded invariant");
    }

    let maybe_command_state = bounded_queue_state_u8();
    kani::assert(
        maybe_command_state.is_some(),
        "bounded command enqueue generator imports valid state",
    );
    if let Some(command_state) = maybe_command_state {
        let before_len = command_state.len();
        let capacity = command_state.capacity();
        let command: u8 = kani::any();

        let (after, decision) = command_enqueue_transition(command_state, command);
        kani::assert(after.capacity() == capacity, "command enqueue preserves capacity");
        match decision {
            EnqueueDecision::Accepted => {
                kani::assert(before_len < capacity, "command enqueue accepts only below capacity");
                kani::assert(after.len() == before_len + 1, "command enqueue increments length by one");
            }
            EnqueueDecision::QueueFull { capacity: observed } => {
                kani::assert(before_len >= capacity, "command enqueue rejects only full queues");
                kani::assert(observed == capacity, "command enqueue QueueFull preserves capacity");
                kani::assert(after.len() == before_len, "command enqueue full rejection preserves length");
            }
        }
        kani::assert(after.len() <= after.capacity(), "command enqueue preserves bounded invariant");
    }
}

/// PO-VB-DZIBX-QS-KANI-010.
#[kani::proof]
#[kani::unwind(24)]
fn po_vb_dzibx_qs_kani_010_bounded_dequeue_pop_tick_transitions() {
    let maybe_action_state = bounded_queue_state_u8();
    kani::assert(
        maybe_action_state.is_some(),
        "bounded action dequeue generator imports valid state",
    );
    if let Some(action_state) = maybe_action_state {
        let before_len = action_state.len();
        let capacity = action_state.capacity();

        kani::cover!(before_len == 0, "PO-010 domain reaches empty transition branch");
        kani::cover!(before_len > 0, "PO-010 domain reaches pop transition branch");

        match action_dequeue_transition(action_state) {
            PopTransition::Empty { state } => {
                kani::assert(before_len == 0, "action dequeue Empty branch requires empty input");
                kani::assert(state.capacity() == capacity, "action dequeue Empty preserves capacity");
                kani::assert(state.len() == 0, "action dequeue Empty preserves zero length");
            }
            PopTransition::Popped { state, item: _ } => {
                kani::assert(before_len > 0, "action dequeue Popped branch requires nonempty input");
                kani::assert(state.capacity() == capacity, "action dequeue Popped preserves capacity");
                kani::assert(state.len() == before_len - 1, "action dequeue consumes exactly one item");
            }
        }
    }

    let maybe_command_state = bounded_queue_state_u8();
    kani::assert(
        maybe_command_state.is_some(),
        "bounded command pop generator imports valid state",
    );
    if let Some(command_state) = maybe_command_state {
        let before_len = command_state.len();
        let capacity = command_state.capacity();

        match command_pop_transition(command_state) {
            PopTransition::Empty { state } => {
                kani::assert(before_len == 0, "command pop Empty branch requires empty input");
                kani::assert(state.capacity() == capacity, "command pop Empty preserves capacity");
                kani::assert(state.len() == 0, "command pop Empty preserves zero length");
            }
            PopTransition::Popped { state, item: _ } => {
                kani::assert(before_len > 0, "command pop Popped branch requires nonempty input");
                kani::assert(state.capacity() == capacity, "command pop Popped preserves capacity");
                kani::assert(state.len() == before_len - 1, "command pop consumes exactly one item");
            }
        }
    }

    let maybe_tick_state = bounded_queue_state_u8();
    kani::assert(
        maybe_tick_state.is_some(),
        "bounded shard tick generator imports valid state",
    );
    if let Some(tick_state) = maybe_tick_state {
        let before_len = tick_state.len();
        let capacity = tick_state.capacity();

        match shard_tick_transition(tick_state) {
            ShardTickTransition::Empty { state } => {
                kani::assert(before_len == 0, "shard tick Empty branch requires empty input");
                kani::assert(state.capacity() == capacity, "shard tick Empty preserves capacity");
                kani::assert(state.len() == 0, "shard tick Empty preserves zero length");
            }
            ShardTickTransition::ConsumedOne { state, command: _ } => {
                kani::assert(before_len > 0, "shard tick ConsumedOne requires nonempty input");
                kani::assert(state.capacity() == capacity, "shard tick ConsumedOne preserves capacity");
                kani::assert(state.len() == before_len - 1, "shard tick consumes exactly one item");
            }
        }
    }
}
