//! Boundary tests for queue-state transition semantics.
//!
//! These tests exercise the verified gaps around warning thresholds,
//! saturating arithmetic at the usize boundary, capacity=1 lifecycle,
//! empty-dequeue discipline, full-queue enqueue rejection, and shard-tick
//! consumption.
//!
//! Tests return `Result<(), String>` so that expected-variant violations
//! and assertion failures propagate as typed errors rather than panics,
//! satisfying the `clippy::panic = "forbid"` and
//! `clippy::panic_in_result_fn = "forbid"` zero-slippage policies. The
//! `check!` and `check_eq!` macros below replace the standard `assert!`
//! and `assert_eq!` macros for the same reason.
//!
//! Note: the pure observation tests in `Gap 1` and `Gap 2` (no
//! fallible helpers) keep the standard `assert_eq!` macros because
//! they do not return `Result`; those macros remain valid there.

use crate::{
    EnqueueDecision, PopTransition, QueueState, ShardTickTransition, action_dequeue_transition,
    action_enqueue_transition, remaining_capacity, shard_tick_transition, warning_payload,
};

/// Shorthand for the test error type.
type TestResult<T> = Result<T, String>;

/// Replacement for `assert!` that returns `Err(message)` instead of panicking.
macro_rules! check {
    ($condition:expr $(,)?) => {{
        if !$condition {
            return Err(format!(
                "assertion failed: {}",
                stringify!($condition)
            ));
        }
    }};
    ($condition:expr, $($arg:tt)+) => {{
        if !$condition {
            return Err(format!($($arg)+));
        }
    }};
}

/// Replacement for `assert_eq!` that returns `Err` with a diff message
/// instead of panicking.
macro_rules! check_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual = $actual;
        let expected = $expected;
        if actual != expected {
            return Err(format!(
                "assertion `{} == {}` failed\n  actual:   {:?}\n  expected: {:?}",
                stringify!($actual),
                stringify!($expected),
                actual,
                expected
            ));
        }
    }};
    ($actual:expr, $expected:expr, $($arg:tt)+) => {{
        let actual = $actual;
        let expected = $expected;
        if actual != expected {
            return Err(format!(
                "{}\n  actual:   {:?}\n  expected: {:?}",
                format!($($arg)+),
                actual,
                expected
            ));
        }
    }};
}

// ─── Gap 1: warning_payload when depth > capacity ──────────────────────────────
/// warning_payload returns None when depth exceeds capacity.
/// The guard `depth >= threshold && depth <= capacity` requires both conditions;
/// when `depth > capacity` the second fails regardless of threshold.
///
/// capacity=1 -> threshold=1 (1*8/10=0, clamped to 1)
/// depth=2 satisfies depth >= threshold but fails depth <= capacity.
#[test]
fn warning_payload_none_when_depth_exceeds_capacity_one() {
    // capacity=1, threshold(1)=1, depth=2: 2>=1 passes, 2<=1 fails -> None
    assert_eq!(warning_payload(1, 2), None);
}

/// warning_payload returns None when depth exceeds capacity for a larger gap.
///
/// capacity=5 -> threshold(5)=4 (5*8/10=4)
/// depth=10: 10>=4 passes, 10<=5 fails -> None
#[test]
fn warning_payload_none_when_depth_exceeds_capacity_large_gap() {
    assert_eq!(warning_payload(5, 10), None);
}

/// warning_payload returns None when depth is far above capacity.
///
/// capacity=3 -> threshold(3)=2 (3*8/10=2)
/// depth=5: 5>=2 passes, 5<=3 fails -> None
#[test]
fn warning_payload_none_when_depth_exceeds_capacity_three() {
    assert_eq!(warning_payload(3, 5), None);
}

/// warning_payload returns None when capacity is zero regardless of depth.
/// threshold(0)=0, depth=0: 0>=0 passes, 0<=0 passes, but we still expect None
/// because a zero-capacity queue is invalid and the guard semantics are
/// designed to only produce warnings for real bounded queues.
#[test]
fn warning_payload_none_when_capacity_zero() {
    assert_eq!(warning_payload(0, 0), None);
}

// ─── Gap 2: remaining_capacity at usize::MAX ───────────────────────────────────
/// remaining_capacity(usize::MAX, usize::MAX) must be exactly 0, not usize::MAX.
/// The function uses saturating_sub, which returns 0 when len >= capacity even
/// at the usize boundary.
#[test]
fn remaining_capacity_usize_max_eq_is_zero() {
    assert_eq!(
        remaining_capacity(usize::MAX, usize::MAX),
        0,
        "saturating_sub of equal usize::MAX values must be 0, not usize::MAX"
    );
}

/// remaining_capacity at usize::MAX with one item consumed must be usize::MAX - 1,
/// not saturating to zero. saturating_sub correctly decrements from the maximum.
#[test]
fn remaining_capacity_usize_max_minus_one() {
    assert_eq!(
        remaining_capacity(usize::MAX, 1),
        usize::MAX - 1,
        "remaining capacity should be usize::MAX - 1 when one slot is used"
    );
}

/// remaining_capacity(usize::MAX, usize::MAX - 1) must be 1.
/// The single available slot is correctly reported at the boundary.
#[test]
fn remaining_capacity_usize_max_minus_one_len() {
    assert_eq!(
        remaining_capacity(usize::MAX, usize::MAX - 1),
        1,
        "one slot remaining when len == usize::MAX - 1"
    );
}

// ─── Gap 3: QueueState capacity=1 lifecycle ───────────────────────────────────
/// Full lifecycle for a minimum-valid-capacity queue (capacity=1):
/// empty -> enqueue(accepted) -> full -> enqueue(rejected) ->
/// dequeue(item=42) -> empty -> enqueue(accepted) -> full ->
/// dequeue(item=7) -> empty.
#[test]
fn queue_state_capacity_one_full_lifecycle() -> TestResult<()> {
    // Start: empty
    let state = empty_state(1)?;
    check!(state.is_empty());
    check!(!state.is_full());
    check_eq!(state.len(), 0);
    check_eq!(state.capacity(), 1);

    // First enqueue: accepted
    let (state, decision) = action_enqueue_transition(state, 42);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    check!(state.is_full());
    check!(!state.is_empty());

    // Second enqueue: rejected, state unchanged
    let (state, decision) = action_enqueue_transition(state, 99);
    check_eq!(
        decision,
        EnqueueDecision::QueueFull { capacity: 1 },
        "second enqueue on a full capacity=1 queue must be rejected"
    );
    check_eq!(state.len(), 1, "rejected enqueue must not mutate the queue");

    // Dequeue: returns item 42, queue is now empty
    let (state, item) = expect_popped(
        action_dequeue_transition(state),
        "expected Popped at capacity=1 with one item",
    )?;
    check_eq!(item, 42);
    check!(state.is_empty());
    check!(!state.is_full());
    check_eq!(state.len(), 0);

    // Re-enqueue after dequeue: accepted again
    let (state, decision) = action_enqueue_transition(state, 7);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    check!(state.is_full());

    // Dequeue the second item
    let (_, item) = expect_popped(
        action_dequeue_transition(state),
        "expected Popped, got Empty",
    )?;
    check_eq!(item, 7);
    Ok(())
}

/// Capacity=1 queue: after dequeue recovers empty state, enqueue should
/// succeed with any new item -- the item value does not matter.
#[test]
fn queue_state_capacity_one_enqueue_after_dequeue_accepts_any_item() -> TestResult<()> {
    let state = empty_state(1)?;
    let (state, _) = action_enqueue_transition(state, 100);
    let (state, _) = expect_popped(action_dequeue_transition(state), "expected Popped")?;
    let (state, decision) = action_enqueue_transition(state, 200);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    Ok(())
}

// ─── Gap 4: action_dequeue_transition on empty queue ──────────────────────────
/// Dequeue from an empty capacity=1 queue returns Empty and preserves the
/// original empty state (capacity and empty length).
#[test]
fn action_dequeue_transition_empty_capacity_one() -> TestResult<()> {
    let state = expect_empty(
        action_dequeue_transition(empty_state(1)?),
        "expected Empty, got Popped",
    )?;
    check!(state.is_empty());
    check_eq!(state.capacity(), 1);
    check_eq!(state.len(), 0);
    Ok(())
}

/// Dequeue from an empty larger queue (capacity=10) also returns Empty
/// and preserves all state attributes.
#[test]
fn action_dequeue_transition_empty_larger_queue() -> TestResult<()> {
    let state = expect_empty(
        action_dequeue_transition(empty_state(10)?),
        "expected Empty, got Popped",
    )?;
    check!(state.is_empty());
    check_eq!(state.capacity(), 10);
    check_eq!(state.len(), 0);
    Ok(())
}

/// Dequeue on empty queue must never panic -- the function returns a PopTransition
/// rather than an Option or Result.
#[test]
fn action_dequeue_transition_empty_does_not_panic() -> TestResult<()> {
    // Three consecutive dequeues on the same empty state must all return Empty.
    expect_empty(
        action_dequeue_transition(empty_state(1)?),
        "expected Empty transition",
    )?;
    expect_empty(
        action_dequeue_transition(empty_state(1)?),
        "expected Empty transition on second call",
    )?;
    expect_empty(
        action_dequeue_transition(empty_state(1)?),
        "expected Empty transition on third call",
    )?;
    Ok(())
}

// ─── Gap 5: action_enqueue_transition with full queue ─────────────────────────
/// Enqueue on a full capacity=1 queue returns QueueFull and preserves the
/// original ticket in the queue.
#[test]
fn action_enqueue_transition_full_capacity_one_preserves_item() -> TestResult<()> {
    let full = state_with_items(1, &[42])?;
    let (state, decision) = action_enqueue_transition(full, 99);
    check_eq!(decision, EnqueueDecision::QueueFull { capacity: 1 });
    check_eq!(state.len(), 1, "rejected enqueue must not mutate the queue");
    let items: std::collections::VecDeque<u8> = state.into_vec_deque();
    check_eq!(items.into_iter().collect::<Vec<_>>(), vec![42]);
    Ok(())
}

/// Enqueue on a full capacity=2 queue returns QueueFull and preserves both items.
#[test]
fn action_enqueue_transition_full_capacity_two_preserves_both_items() -> TestResult<()> {
    let full = state_with_items(2, &[1, 2])?;
    let (state, decision) = action_enqueue_transition(full, 3);
    check_eq!(decision, EnqueueDecision::QueueFull { capacity: 2 });
    check_eq!(state.len(), 2);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![1, 2]);
    Ok(())
}

/// EnqueueDecision returned by action_enqueue_transition on a full queue
/// carries the correct capacity value (not some other value).
#[test]
fn action_enqueue_transition_full_carries_correct_capacity() -> TestResult<()> {
    let full = state_with_items(7, &[])?;
    // Fill it up to capacity 7
    let mut state = full;
    for i in 0..7 {
        let (s, decision) = action_enqueue_transition(state, i);
        check_eq!(decision, EnqueueDecision::Accepted);
        state = s;
    }
    check!(state.is_full());
    let (_state, decision) = action_enqueue_transition(state, 200);
    match decision {
        EnqueueDecision::QueueFull { capacity } => check_eq!(capacity, 7),
        EnqueueDecision::Accepted => {
            return Err("expected rejection on full queue".to_string());
        }
    }
    Ok(())
}

// ─── Gap 6: shard_tick_transition with consumed items ─────────────────────────
/// Shard tick on a queue with one item consumes that item and returns ConsumedOne.
#[test]
fn shard_tick_transition_consumes_single_item() -> TestResult<()> {
    let state = state_with_items(2, &[42])?;
    let (command, s) = expect_consumed_one(shard_tick_transition(state), "expected ConsumedOne")?;
    check_eq!(command, 42);
    check!(s.is_empty());
    check_eq!(s.len(), 0);
    Ok(())
}

/// Shard tick on a queue with multiple items consumes only the old front,
/// preserves the tail in order.
#[test]
fn shard_tick_transition_preserves_tail_order() -> TestResult<()> {
    let state = state_with_items(5, &[10, 20, 30])?;
    let (command, s) = expect_consumed_one(shard_tick_transition(state), "expected ConsumedOne")?;
    check_eq!(command, 10);
    check_eq!(s.len(), 2);
    check!(!s.is_empty());
    let items: Vec<u8> = s.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![20, 30]);
    Ok(())
}

/// Shard tick on an empty queue returns Empty (no command consumed).
#[test]
fn shard_tick_transition_empty_returns_empty() -> TestResult<()> {
    let state = expect_shard_empty(shard_tick_transition(empty_state(3)?), "expected Empty")?;
    check!(state.is_empty());
    check_eq!(state.capacity(), 3);
    check_eq!(state.len(), 0);
    Ok(())
}

/// Multiple shard ticks on a filled queue drain items in FIFO order.
#[test]
fn shard_tick_transition_multiple_ticks_fifo_order() -> TestResult<()> {
    let state = state_with_items(4, &[1, 2, 3])?;

    // Tick 1: consumes 1
    let (c1, s1) = expect_consumed_one(
        shard_tick_transition(state),
        "expected ConsumedOne on tick 1",
    )?;
    check_eq!(c1, 1);
    check_eq!(s1.len(), 2);

    // Tick 2: consumes 2
    let (c2, s2) =
        expect_consumed_one(shard_tick_transition(s1), "expected ConsumedOne on tick 2")?;
    check_eq!(c2, 2);
    check_eq!(s2.len(), 1);

    // Tick 3: consumes 3
    let (c3, s3) =
        expect_consumed_one(shard_tick_transition(s2), "expected ConsumedOne on tick 3")?;
    check_eq!(c3, 3);
    check_eq!(s3.len(), 0);

    // Tick 4: queue is empty, returns Empty
    expect_shard_empty(shard_tick_transition(s3), "expected Empty on tick 4")?;
    Ok(())
}

// ─── Helper constructors ──────────────────────────────────────────────────────

fn empty_state(capacity: usize) -> TestResult<QueueState<u8>> {
    QueueState::new(capacity, capacity)
        .map_err(|reason| format!("valid test capacity rejected: {reason:?}"))
}

fn state_with_items(capacity: usize, items: &[u8]) -> TestResult<QueueState<u8>> {
    let queue: std::collections::VecDeque<u8> = items.iter().copied().collect();
    QueueState::from_vec_deque(capacity, capacity, queue)
        .map_err(|reason| format!("valid test queue rejected: {reason:?}"))
}

/// Returns the inner `state` and `item` if the transition is `Popped`.
fn expect_popped<T>(transition: PopTransition<T>, message: &str) -> TestResult<(QueueState<T>, T)> {
    match transition {
        PopTransition::Popped { state, item } => Ok((state, item)),
        PopTransition::Empty { .. } => Err(message.to_string()),
    }
}

/// Returns the inner `state` if the transition is `Empty`.
fn expect_empty<T>(transition: PopTransition<T>, message: &str) -> TestResult<QueueState<T>> {
    match transition {
        PopTransition::Empty { state } => Ok(state),
        PopTransition::Popped { .. } => Err(message.to_string()),
    }
}

/// Returns the inner `command` and `state` if the transition is `ConsumedOne`.
fn expect_consumed_one<T>(
    transition: ShardTickTransition<T>,
    message: &str,
) -> TestResult<(T, QueueState<T>)> {
    match transition {
        ShardTickTransition::ConsumedOne { command, state } => Ok((command, state)),
        ShardTickTransition::Empty { .. } => Err(message.to_string()),
    }
}

/// Returns the inner `state` if the shard tick transition is `Empty`.
fn expect_shard_empty<T>(
    transition: ShardTickTransition<T>,
    message: &str,
) -> TestResult<QueueState<T>> {
    match transition {
        ShardTickTransition::Empty { state } => Ok(state),
        ShardTickTransition::ConsumedOne { .. } => Err(message.to_string()),
    }
}
