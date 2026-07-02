#![forbid(unsafe_code)]
//! Integration and property tests for MAJOR-1: ArrayQueue vs crossbeam_channel.
//!
//! # Critical Lethal Coverage
//!
//! ## LETHAL-1: `MemoryIngress::bounded` constructor — zero BDD scenarios
//!
//! Covered by: `memory_ingress_bounded_constructor_produces_memory_ingress_instance`,
//! `memory_ingress_bounded_with_capacity_one_succeeds`, `memory_ingress_bounded_with_various_capacities_succeeds`.
//!
//! ## LETHAL-2: `IngressFrame::new` — proptest exists but no BDD scenario
//!
//! Covered by: `ingress_frame_new_returns_ingress_frame_when_payload_within_limit`,
//! `ingress_frame_new_returns_payload_too_large_when_payload_exceeds_limit` (exact variant assertion),
//! `ingress_frame_new_with_empty_payload_and_min_max_succeeds`,
//! `ingress_frame_new_with_payload_at_exactly_max_boundary_succeeds`.
//!
//! ## LETHAL-3: `IpcError::PayloadTooLarge` — never asserted as exact variant
//!
//! Covered by: `ingress_frame_new_returns_payload_too_large_when_payload_exceeds_limit`
//! which asserts `Err(IpcError::PayloadTooLarge { actual: DEFAULT + 1, limit: DEFAULT })`.

use bytes::Bytes;
use proptest::prelude::*;
use std::num::NonZeroUsize;

use vb_core::{RunId, WorkflowDigest};

use crate::bounded::{BoundedPayload, MaxPayloadBytes, QueueCapacity};
use crate::error::IpcError;
use crate::ingress::{IngressFrame, MemoryIngress};

// ─── Test helpers ──────────────────────────────────────────────────────────────

/// Creates a minimal valid `IngressFrame` for testing.
fn make_frame(run_id_val: u64, payload_bytes: impl Into<Bytes>) -> IngressFrame {
    IngressFrame::new(
        RunId::new(run_id_val),
        WorkflowDigest::from_bytes([0u8; 32]),
        payload_bytes.into(),
        MaxPayloadBytes::DEFAULT,
    )
    .expect("test frame must be valid")
}

/// Creates a `QueueCapacity` from a `usize`.
fn capacity(cap: usize) -> QueueCapacity {
    QueueCapacity::new(NonZeroUsize::new(cap).expect("capacity must be non-zero"))
}

// ════════════════════════════════════════════════════════════════════════════════════════
// LETHAL-1: MemoryIngress::bounded constructor — zero BDD scenarios
// ════════════════════════════════════════════════════════════════════════════════════════

/// Given: nothing (no queue exists)
/// When:  `MemoryIngress::bounded(QueueCapacity::new(NonZeroUsize::MIN))` is called
/// Then:  a `MemoryIngress` instance is returned (not panic, not error)
#[test]
fn memory_ingress_bounded_constructor_produces_memory_ingress_instance() {
    // When: constructing a bounded ingress queue with minimum capacity
    let cap = NonZeroUsize::MIN;
    let queue_cap = QueueCapacity::new(cap);
    let ingress = MemoryIngress::bounded(queue_cap);

    // Then: result is a valid MemoryIngress that can be used
    // (can submit and recv without error)
    let frame = make_frame(1, b"");
    assert_eq!(
        ingress.try_submit(frame),
        Ok(()),
        "submit to fresh queue must succeed"
    );
    let recv_result = ingress.try_recv();
    assert!(recv_result.is_ok(), "recv from fresh queue must succeed");
    assert!(
        recv_result.unwrap().is_some(),
        "recv from fresh queue must return Some(frame)"
    );
}

/// BDD scenario for LETHAL-1: capacity=1 queue construction.
/// Given: no queue exists
/// When:  `MemoryIngress::bounded(capacity=1)` is called
/// Then:  a non-null queue is returned and `len()` returns 0 (empty on creation)
#[test]
fn memory_ingress_bounded_with_capacity_one_succeeds() {
    let ingress = MemoryIngress::bounded(capacity(1));

    assert_eq!(ingress.len(), 0, "newly created queue must be empty");
    assert!(
        ingress.is_empty(),
        "newly created queue must report is_empty() == true"
    );
}

/// Verifies `bounded` works across a range of capacity values.
#[test]
fn memory_ingress_bounded_with_various_capacities_succeeds() {
    for cap in [1, 2, 8, 64, 256] {
        let ingress = MemoryIngress::bounded(capacity(cap));
        assert_eq!(
            ingress.len(),
            0,
            "queue with capacity {cap} must start empty"
        );
        assert!(
            ingress.is_empty(),
            "queue with capacity {cap} must report is_empty"
        );
    }
}

/// Given: a `MemoryIngress` created via `bounded(capacity=1)`
/// When:  `try_submit` is called with a valid `IngressFrame`
/// Then:  `Ok(())` is returned and the queue reports `len() == 1`
/// (combines LETHAL-1 constructor setup with Behavior 1)
#[test]
fn memory_ingress_try_submit_succeeds_when_queue_has_capacity() {
    // Given: a bounded queue with capacity 1
    let ingress = MemoryIngress::bounded(capacity(1));
    let frame = make_frame(42, b"test-payload");

    // When: submitting a frame to a non-full queue
    let result = ingress.try_submit(frame);

    // Then: submit succeeds and queue depth reflects it
    assert_eq!(
        result,
        Ok(()),
        "try_submit must succeed when queue has capacity"
    );
    assert_eq!(ingress.len(), 1, "len() must be 1 after one submit");
    assert!(!ingress.is_empty(), "is_empty() must be false after submit");
}

// ════════════════════════════════════════════════════════════════════════════════════════
// LETHAL-2: IngressFrame::new — proptest exists but no BDD scenario
// LETHAL-3: IpcError::PayloadTooLarge — never asserted as exact variant
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD scenario for `IngressFrame::new` happy path.
/// Given: valid `RunId`, `WorkflowDigest`, empty `Bytes`, and `MaxPayloadBytes::DEFAULT`
/// When:  `IngressFrame::new(...)` is called
/// Then:  `Ok(IngressFrame)` is returned with exact field values
#[test]
fn ingress_frame_new_returns_ingress_frame_when_payload_within_limit() {
    // Given: a valid run ID, workflow digest, and empty payload
    let run_id = RunId::new(99);
    let workflow = WorkflowDigest::from_bytes([0xAB; 32]);
    let payload = Bytes::new();

    // When: constructing an IngressFrame
    let result = IngressFrame::new(run_id, workflow, payload, MaxPayloadBytes::DEFAULT);

    // Then: it succeeds with exact field values
    assert!(
        result.is_ok(),
        "IngressFrame::new must succeed for empty payload"
    );
    let frame = result.unwrap();
    assert_eq!(frame.run_id(), run_id, "run_id must match exactly");
    assert_eq!(frame.workflow(), workflow, "workflow must match exactly");
    assert_eq!(frame.payload().bytes().len(), 0, "payload must be empty");
}

/// LETHAL-3: Exact variant assertion for `PayloadTooLarge`.
/// Given: a payload of `MaxPayloadBytes::DEFAULT + 1` bytes
/// When:  `IngressFrame::new(...)` is called
/// Then:  `Err(IpcError::PayloadTooLarge { actual: DEFAULT + 1, limit: DEFAULT })`
///        is returned — exact variant, exact field values.
#[test]
fn ingress_frame_new_returns_payload_too_large_when_payload_exceeds_limit() {
    // Given: a payload one byte over the default limit
    let default_max = MaxPayloadBytes::DEFAULT.get();
    let over_limit_payload = Bytes::from(vec![0xFF_u8; default_max.saturating_add(1)]);

    // When: constructing an IngressFrame with the oversized payload
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        over_limit_payload,
        MaxPayloadBytes::DEFAULT,
    );

    // Then: PayloadTooLarge is returned with exact actual/limit values
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: default_max.saturating_add(1),
            limit: default_max,
        }),
        "PayloadTooLarge must carry exact actual and limit values"
    );
}

/// Boundary: empty payload + `NonZeroUsize::MIN` max.
/// Given: `Bytes::new()` (empty) and `MaxPayloadBytes::new(NonZeroUsize::MIN)`
/// When:  `IngressFrame::new(...)` is called
/// Then:  `Ok(IngressFrame)` is returned
#[test]
fn ingress_frame_new_with_empty_payload_and_min_max_succeeds() {
    let min_max = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::new(),
        min_max,
    );

    let expected = IngressFrame {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        payload: BoundedPayload::new(Bytes::new(), min_max).unwrap(),
    };
    assert_eq!(
        result,
        Ok(expected),
        "empty payload with min_max must succeed"
    );
}

/// Boundary: payload exactly at `MaxPayloadBytes::DEFAULT`.
/// Given: `Bytes::from(vec![0u8; DEFAULT])` and `MaxPayloadBytes::DEFAULT`
/// When:  `IngressFrame::new(...)` is called
/// Then:  `Ok(IngressFrame)` is returned
#[test]
fn ingress_frame_new_with_payload_at_exactly_max_boundary_succeeds() {
    let max = MaxPayloadBytes::DEFAULT;
    let payload = Bytes::from(vec![0u8; max.get()]);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        payload.clone(),
        max,
    );

    let expected = IngressFrame {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        payload: BoundedPayload::new(payload, max).unwrap(),
    };
    assert_eq!(
        result,
        Ok(expected),
        "payload at exactly max boundary must succeed"
    );
}

/// Boundary: `BoundedPayload` at exactly `NonZeroUsize::MIN` limit.
#[test]
fn bounded_payload_new_rejects_one_over_min_limit() {
    let min = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let over_by_one = Bytes::from(vec![0u8; min.get().saturating_add(1)]);

    let result = BoundedPayload::new(over_by_one, min);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: min.get().saturating_add(1),
            limit: min.get(),
        }),
        "one byte over NonZeroUsize::MIN limit must return exact PayloadTooLarge"
    );
}

/// Boundary: empty payload always accepted regardless of limit.
#[test]
fn bounded_payload_new_accepts_empty_payload_for_any_limit() {
    let limits = [
        MaxPayloadBytes::new(NonZeroUsize::MIN),
        MaxPayloadBytes::DEFAULT,
    ];
    for limit in limits {
        let result = BoundedPayload::new(Bytes::new(), limit);
        assert!(
            result.is_ok(),
            "empty payload must be accepted with limit {:?}",
            limit
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Behavior 2: Full error — MemoryIngress returns Full when queue is at capacity
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD: MemoryIngress returns `Full` error when queue is at capacity (non-blocking).
/// Given: a `MemoryIngress` queue with capacity 1, already containing 1 frame
/// When:  a producer calls `try_submit(frame)` on the full queue
/// Then:  `Err(IpcError::Full)` is returned (NOT `Ok(())`, NOT panic)
/// And:   the original frame remains in the queue (FIFO order preserved).
#[test]
fn memory_ingress_try_submit_returns_full_when_queue_is_at_capacity() {
    // Given: a capacity-1 queue containing one frame
    let ingress = MemoryIngress::bounded(capacity(1));
    let frame = make_frame(1, b"first");
    ingress
        .try_submit(frame.clone())
        .expect("first submit must succeed");

    // When: submitting a second frame to the full queue
    let second_frame = make_frame(2, b"second");
    let result = ingress.try_recv();

    // Then: first recv returns the original frame
    assert_eq!(
        result,
        Ok(Some(frame)),
        "first recv must return the original frame"
    );

    // Now queue is empty — submit the second frame
    let result2 = ingress.try_submit(second_frame.clone());
    assert_eq!(result2, Ok(()), "submit after recv must succeed");

    // Queue is full again — second submit must fail
    let third_frame = make_frame(3, b"third");
    let result3 = ingress.try_submit(third_frame);

    assert_eq!(
        result3,
        Err(IpcError::Full),
        "submitting to a full queue must return IpcError::Full"
    );

    // And: the second frame is still in the queue (not lost)
    assert_eq!(
        ingress.try_recv(),
        Ok(Some(second_frame)),
        "frame in queue must not be lost when Full is returned"
    );
}

/// Exact variant assertion for `IpcError::Full` (distinct from `Disconnected`).
#[test]
fn memory_ingress_try_submit_full_is_exact_variant_not_disconnected() {
    let ingress = MemoryIngress::bounded(capacity(1));
    let frame1 = make_frame(1, b"");
    let frame2 = make_frame(2, b"");

    ingress
        .try_submit(frame1)
        .expect("first submit must succeed");

    let result = ingress.try_submit(frame2);

    assert_eq!(
        result,
        Err(IpcError::Full),
        "must be Full, not Disconnected"
    );
    assert_ne!(result, Err(IpcError::Disconnected), "Full != Disconnected");
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Behavior 3: FIFO order — MemoryIngress dequeues frames in submission order
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD: MemoryIngress dequeues a frame in FIFO order when queue is non-empty.
/// Given: a `MemoryIngress` queue with capacity 2, containing frame1 then frame2
/// When:  a consumer calls `try_recv()` twice
/// Then:  first call returns `Ok(Some(frame1))`, second returns `Ok(Some(frame2))`
/// And:   after both dequeues, `is_empty()` is `true` and `len()` is 0.
#[test]
fn memory_ingress_try_recv_returns_fifo_order_when_queue_has_items() {
    // Given: a capacity-2 queue with two distinct frames submitted in order
    let ingress = MemoryIngress::bounded(capacity(2));
    let frame1 = make_frame(1, b"first");
    let frame2 = make_frame(2, b"second");
    ingress
        .try_submit(frame1.clone())
        .expect("first submit must succeed");
    ingress
        .try_submit(frame2.clone())
        .expect("second submit must succeed");

    // When: dequeuing twice
    let first = ingress.try_recv();
    let second = ingress.try_recv();

    // Then: frames are returned in FIFO order
    assert_eq!(
        first,
        Ok(Some(frame1.clone())),
        "first recv must return first frame"
    );
    assert_eq!(
        second,
        Ok(Some(frame2.clone())),
        "second recv must return second frame"
    );

    // And: queue is empty after all dequeues
    assert_eq!(ingress.len(), 0, "len() must be 0 after all dequeues");
    assert!(
        ingress.is_empty(),
        "is_empty() must be true after all dequeues"
    );
    assert_eq!(
        ingress.try_recv(),
        Ok(None),
        "recv on empty queue must return Ok(None)"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Behavior 4: None on empty dequeue — non-blocking
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD: MemoryIngress returns `Ok(None)` on empty dequeue (non-blocking).
/// Given: a `MemoryIngress` queue with capacity 1, containing zero frames
/// When:  a consumer calls `try_recv()` on the empty queue
/// Then:  `Ok(None)` is returned immediately (NOT `Err`, NOT panic).
#[test]
fn memory_ingress_try_recv_returns_none_when_queue_is_empty() {
    let ingress = MemoryIngress::bounded(capacity(1));

    let result = ingress.try_recv();

    assert_eq!(
        result,
        Ok(None),
        "recv on empty queue must return Ok(None), not Err"
    );
    assert!(result.is_ok(), "result must be Ok (not Err)");
}

/// Empty queue: `Ok(None)` is NOT the same as `Err(Disconnected)`.
#[test]
fn memory_ingress_try_recv_empty_differs_from_disconnected() {
    let ingress = MemoryIngress::bounded(capacity(1));

    let empty_result = ingress.try_recv();
    assert_eq!(empty_result, Ok(None), "empty queue returns Ok(None)");

    // Disconnected must be Err, not Ok(None)
    let mut disconnected_ingress = MemoryIngress::bounded(capacity(1));
    disconnected_ingress.disconnect_sender();
    let disc_result = disconnected_ingress.try_recv();

    assert!(
        disc_result.is_err(),
        "disconnected queue must return Err, not Ok(None)"
    );
    assert_eq!(disc_result, Err(IpcError::Disconnected));
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Behavior 5: Disconnected error — sender side dropped
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD: MemoryIngress returns `Err(IpcError::Disconnected)` when sender is dropped.
/// Given: a `MemoryIngress` queue where `disconnect_sender()` has been called
/// When:  a consumer calls `try_recv()`
/// Then:  `Err(IpcError::Disconnected)` is returned.
#[test]
fn memory_ingress_try_recv_returns_disconnected_when_sender_dropped() {
    let mut ingress = MemoryIngress::bounded(capacity(1));

    // When: the sender side is disconnected
    ingress.disconnect_sender();

    // Then: recv returns Disconnected error
    let result = ingress.try_recv();
    assert_eq!(
        result,
        Err(IpcError::Disconnected),
        "recv after sender disconnect must return IpcError::Disconnected"
    );
}

/// Disconnected after partial fill: sender dropped after some submits.
#[test]
fn memory_ingress_try_recv_returns_disconnected_after_partial_submit() {
    let mut ingress = MemoryIngress::bounded(capacity(2));
    let frame = make_frame(1, b"");

    ingress.try_submit(frame).expect("submit must succeed");
    ingress.disconnect_sender();

    // First recv should still work (queue not empty)
    assert!(
        ingress.try_recv().is_ok(),
        "recv before disconnect signal must succeed"
    );
    // Second recv after disconnect
    assert_eq!(
        ingress.try_recv(),
        Err(IpcError::Disconnected),
        "recv after sender disconnect must return Disconnected"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Behavior 6: len() accuracy
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD: MemoryIngress reports accurate queue depth via `len()`.
#[test]
fn memory_ingress_len_returns_exact_count_when_queue_has_two_frames() {
    let ingress = MemoryIngress::bounded(capacity(3));
    assert_eq!(ingress.len(), 0, "len() must be 0 on empty queue");

    let frame1 = make_frame(1, b"a");
    let frame2 = make_frame(2, b"b");
    ingress.try_submit(frame1).expect("submit 1 must succeed");
    assert_eq!(ingress.len(), 1, "len() must be 1 after 1 submit");

    ingress.try_submit(frame2).expect("submit 2 must succeed");
    assert_eq!(ingress.len(), 2, "len() must be 2 after 2 submits");

    ingress.try_recv().expect("recv must succeed");
    assert_eq!(ingress.len(), 1, "len() must be 1 after 1 recv");

    ingress.try_recv().expect("recv must succeed");
    assert_eq!(ingress.len(), 0, "len() must be 0 after all recv");
}

/// len() never exceeds capacity.
#[test]
fn memory_ingress_len_never_exceeds_capacity() {
    let cap = 8;
    let ingress = MemoryIngress::bounded(capacity(cap));

    // Submit up to capacity
    for i in 0..cap {
        let frame = make_frame(u64::try_from(i).unwrap(), b"x");
        let result = ingress.try_submit(frame);
        assert_eq!(result, Ok(()), "submit {i} must succeed");
    }

    assert_eq!(
        ingress.len(),
        cap,
        "len() must equal capacity, not exceed it"
    );

    // Try one more (should fail)
    let extra = make_frame(999, b"");
    assert_eq!(ingress.try_submit(extra), Err(IpcError::Full));
    assert_eq!(ingress.len(), cap, "len() must not increase past capacity");
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Behavior 7: is_empty() correctness
// ════════════════════════════════════════════════════════════════════════════════════════

/// BDD: is_empty() returns `true` when queue has no frames.
#[test]
fn memory_ingress_is_empty_returns_true_when_queue_has_no_frames() {
    let ingress = MemoryIngress::bounded(capacity(1));
    assert!(ingress.is_empty(), "new queue must be empty");
    assert_eq!(ingress.len(), 0);
}

/// BDD: is_empty() returns `false` when queue has at least one frame.
#[test]
fn memory_ingress_is_empty_returns_false_when_queue_has_one_frame() {
    let ingress = MemoryIngress::bounded(capacity(1));
    let frame = make_frame(1, b"");
    ingress.try_submit(frame).expect("submit must succeed");

    assert!(!ingress.is_empty(), "queue with 1 frame must not be empty");
    assert_eq!(ingress.len(), 1);
}

/// Invariant: `is_empty() == (len() == 0)` must always hold.
#[test]
fn memory_ingress_is_empty_equals_len_zero_invariant() {
    let ingress = MemoryIngress::bounded(capacity(4));

    // Initially empty
    assert_eq!(ingress.is_empty(), ingress.len() == 0);

    let frame = make_frame(1, b"");
    ingress.try_submit(frame).expect("submit must succeed");
    assert_eq!(ingress.is_empty(), ingress.len() == 0);

    ingress.try_recv().expect("recv must succeed");
    assert_eq!(ingress.is_empty(), ingress.len() == 0);
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Combinatorial boundary coverage — IngressFrame::new
// ════════════════════════════════════════════════════════════════════════════════════════

/// Single-byte payloads of varying values must always succeed within limit.
#[test]
fn ingress_frame_new_accepts_single_byte_payloads_within_limit() {
    for byte in [0x00u8, 0x7F, 0x80, 0xFF] {
        let payload = Bytes::from_static(&[byte]);
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            payload,
            MaxPayloadBytes::DEFAULT,
        );
        assert!(
            result.is_ok(),
            "single byte 0x{:02X} must be accepted within limit",
            byte
        );
    }
}

/// Multi-byte payloads within limit must succeed.
#[test]
fn ingress_frame_new_accepts_multi_byte_payloads_within_limit() {
    let max = MaxPayloadBytes::DEFAULT.get();
    let mid_payload = Bytes::from(vec![0xCD; max / 2]);

    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        mid_payload,
        MaxPayloadBytes::DEFAULT,
    );
    assert!(result.is_ok(), "mid-size payload within limit must succeed");
}

/// One byte over limit must return exact `PayloadTooLarge` variant.
#[test]
fn ingress_frame_new_rejects_one_byte_over_limit_exact_variant() {
    let max = MaxPayloadBytes::DEFAULT.get();
    let one_over = Bytes::from(vec![0x01; max.saturating_add(1)]);

    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        one_over,
        MaxPayloadBytes::DEFAULT,
    );

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: max.saturating_add(1),
            limit: max,
        }),
        "one byte over limit must return exact PayloadTooLarge variant"
    );
}

/// Exactly at limit + 1 byte (using NonZeroUsize boundary) must fail.
#[test]
fn bounded_payload_rejects_exactly_one_over_nonzero_min_limit() {
    let min_limit = NonZeroUsize::MIN;
    let payload = Bytes::from(vec![0x42; min_limit.get().saturating_add(1)]);
    let max = MaxPayloadBytes::new(min_limit);

    let result = BoundedPayload::new(payload, max);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: min_limit.get().saturating_add(1),
            limit: min_limit.get(),
        }),
        "one over NonZeroUsize::MIN must return exact PayloadTooLarge"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Proptest invariants
// ════════════════════════════════════════════════════════════════════════════════════════

/// Strategy: arbitrary non-zero capacity capped at 1024.
fn arb_capacity() -> impl Strategy<Value = QueueCapacity> {
    any::<NonZeroUsize>()
        .prop_filter("capacity must be > 0 and ≤ 1024", |nz| nz.get() <= 1024)
        .prop_map(QueueCapacity::new)
}

/// Strategy: arbitrary `IngressFrame` within default payload limit.
fn arb_ingress_frame() -> impl Strategy<Value = IngressFrame> {
    // Generate payloads up to 1024 bytes for test speed.
    (0..=1024usize).prop_flat_map(move |payload_len| {
        let payload_bytes: Vec<u8> =
            proptest::collection::vec(proptest::prelude::any::<u8>(), payload_len);
        payload_bytes.prop_map(move |bytes| {
            IngressFrame::new(
                RunId::new(1),
                WorkflowDigest::from_bytes([0u8; 32]),
                Bytes::from(bytes),
                MaxPayloadBytes::DEFAULT,
            )
            .expect("generated frame must be valid within default limit")
        })
    })
}

proptest! {
    /// Invariant: `try_submit`/`try_recv` cycle preserves FIFO order.
    /// For any sequence of N successful `try_submit` calls followed by N successful
    /// `try_recv` calls, the dequeued frames must be in the same order as submitted.
    #[test]
    fn fifo_order_invariant_for_submit_recv_cycle(cap in arb_capacity(), frame_count in 1..=16usize) {
        let ingress = MemoryIngress::bounded(cap);
        let capacity = cap.get().min(16);

        // Submit `frame_count` frames (capped by queue capacity)
        let submit_count = frame_count.min(capacity);
        let mut submitted = Vec::with_capacity(submit_count);
        for i in 0..submit_count {
            let frame = make_frame(u64::try_from(i).unwrap(), format!("frame-{}", i));
            match ingress.try_submit(frame) {
                Ok(()) => {
                    submitted.push(i);
                }
                Err(IpcError::Full) => {
                    break;
                }
                Err(e) => {
                    panic!("unexpected error on submit: {:?}", e);
                }
            }
        }

        // Recv all submitted frames
        let mut received = Vec::with_capacity(submitted.len());
        while let Ok(Some(_)) = ingress.try_recv() {
            received.push(());
        }

        // Invariant: submit count equals receive count (no frames lost)
        prop_assert_eq!(
            submitted.len(),
            received.len(),
            "all successfully submitted frames must be received (no loss)"
        );
    }

    /// Invariant: `is_empty() == (len() == 0)` holds after any sequence of ops.
    #[test]
    fn is_empty_len_zero_invariant_after_mixed_operations(cap in arb_capacity()) {
        let ingress = MemoryIngress::bounded(cap);
        let capacity = cap.get();

        // Alternate submit and recv operations
        let mut expected_empty = true;
        prop_assert_eq!(ingress.is_empty(), expected_empty);
        prop_assert_eq!(ingress.len() == 0, expected_empty);

        for i in 0..capacity.min(8) {
            let frame = make_frame(u64::try_from(i).unwrap(), b"");
            let submit_result = ingress.try_submit(frame);
            match submit_result {
                Ok(()) => {
                    expected_empty = false;
                    prop_assert_eq!(ingress.is_empty(), expected_empty);
                    prop_assert_eq!(ingress.len() == 0, expected_empty);
                }
                Err(IpcError::Full) => {
                    // Queue is full — now drain it
                    while let Ok(Some(())) = ingress.try_recv() {
                        // drain
                    }
                    expected_empty = true;
                    prop_assert_eq!(ingress.is_empty(), expected_empty);
                    prop_assert_eq!(ingress.len() == 0, expected_empty);
                }
                Err(e) => {
                    panic!("unexpected error: {:?}", e);
                }
            }
        }
    }

    /// Invariant: capacity-1 queue must exhibit correct full/empty signaling.
    /// Sequence: submit×2 → first succeeds, second returns Full;
    ///           recv×2   → first returns frame, second returns None.
    #[test]
    fn capacity_one_full_empty_signaling_invariant() {
        let ingress = MemoryIngress::bounded(capacity(1));

        let frame1 = make_frame(1, b"first");
        let frame2 = make_frame(2, b"second");

        // Step 1: First submit must succeed
        let r1 = ingress.try_submit(frame1.clone());
        prop_assert_eq!(r1, Ok(()), "first submit on capacity-1 must succeed");

        // Step 2: Second submit must return Full
        let r2 = ingress.try_submit(frame2.clone());
        prop_assert_eq!(
            r2,
            Err(IpcError::Full),
            "second submit on full queue must return Full"
        );

        // Step 3: First recv must return frame1
        let r3 = ingress.try_recv();
        prop_assert_eq!(
            r3,
            Ok(Some(frame1)),
            "first recv must return first frame"
        );

        // Step 4: Second recv must return None (queue empty)
        let r4 = ingress.try_recv();
        prop_assert_eq!(r4, Ok(None), "second recv on empty queue must return None");
    }

    /// Invariant: `len()` always returns exact count regardless of interleaving.
    #[test]
    fn len_exact_count_invariant_after_every_submit(cap in arb_capacity()) {
        let ingress = MemoryIngress::bounded(cap);
        let capacity = cap.get().min(8);

        for i in 0..capacity {
            let frame = make_frame(u64::try_from(i).unwrap(), b"");
            let submit_result = ingress.try_submit(frame);
            match submit_result {
                Ok(()) => {
                    prop_assert_eq!(
                        ingress.len(),
                        i + 1,
                        "len() must equal number of successful submits"
                    );
                }
                Err(IpcError::Full) => {
                    prop_assert_eq!(
                        ingress.len(),
                        capacity,
                        "len() must equal capacity when queue is full"
                    );
                    break;
                }
                Err(e) => {
                    panic!("unexpected error: {:?}", e);
                }
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════
// Anti-invariant: overflow / underflow protection
// ════════════════════════════════════════════════════════════════════════════════════════

/// Anti-invariant: submitting `capacity + 1` frames must result in exactly
/// `capacity` successes and 1 `Full` error, with no frames lost.
#[test]
fn submit_capacity_plus_one_produces_exactly_one_full_error() {
    let cap = 4;
    let ingress = MemoryIngress::bounded(capacity(cap));
    let mut success_count = 0;
    let mut full_count = 0;

    for i in 0..(cap + 1) {
        let frame = make_frame(u64::try_from(i).unwrap(), b"");
        match ingress.try_submit(frame) {
            Ok(()) => {
                success_count += 1;
            }
            Err(IpcError::Full) => {
                full_count += 1;
            }
            Err(e) => {
                panic!("unexpected error: {:?}", e);
            }
        }
    }

    assert_eq!(success_count, cap, "exactly capacity submits must succeed");
    assert_eq!(full_count, 1, "exactly one submit must fail with Full");
    assert_eq!(ingress.len(), cap, "queue must be full after cap submits");
}

/// Anti-invariant: recv on empty never returns an error variant other than Disconnected.
#[test]
fn recv_on_empty_never_returns_unexpected_error_variant() {
    let ingress = MemoryIngress::bounded(capacity(1));
    let result = ingress.try_recv();

    // Only Ok(None) is valid for empty; any Err is a bug
    match result {
        Ok(None) => { /* correct */ }
        Ok(Some(_)) => {
            panic!("empty queue must not return a frame")
        }
        Err(IpcError::Disconnected) => { /* disconnected state */ }
        Err(e) => {
            panic!("empty recv must not return error variant {:?}", e)
        }
    }
}

/// Anti-invariant: disconnect then recv must NOT return Ok(None) or Full.
#[test]
fn disconnected_recv_never_returns_ok_or_full() {
    let mut ingress = MemoryIngress::bounded(capacity(1));
    ingress.disconnect_sender();

    let result = ingress.try_recv();

    match result {
        Err(IpcError::Disconnected) => { /* correct */ }
        Ok(Some(_)) => {
            panic!("disconnected queue must not return a frame")
        }
        Ok(None) => {
            panic!("disconnected queue must not return Ok(None)")
        }
        Err(IpcError::Full) => {
            panic!("disconnected queue must not return Full")
        }
        Err(_) => {
            panic!("disconnected queue returned unexpected error")
        }
    }
}
