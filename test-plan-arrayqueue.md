# Test Plan: MAJOR-1 — ArrayQueue Lock-Free SPSC Migration

## Summary

- **Bead**: MAJOR-1
- **Mandate**: Section 50 — `ArrayQueue` (lock-free SPSC) replaces `crossbeam_channel`; `crossbeam_channel` is FORBIDDEN
- **Target implementation**: `ArrayQueue<T, RingFlagged>` — custom lock-free SPSC queue
- **Behaviors identified**: 7
- **Trophy allocation**: 2 unit / 4 integration / 1 static
- **Proptest invariants**: 3
- **Fuzz targets**: 1 (deserialization boundary)
- **Kani harnesses**: 2 (lock-freedom proofs)

---

## 1. Behavior Inventory

1. **MemoryIngress enqueues a frame without blocking** when the queue has capacity.
2. **MemoryIngress returns `Full` error** when the queue is at capacity (non-blocking).
3. **MemoryIngress dequeues a frame in FIFO order** when the queue is non-empty.
4. **MemoryIngress returns `None` on empty dequeue** (non-blocking).
5. **MemoryIngress returns `Disconnected` error** when the sender side is dropped.
6. **MemoryIngress reports accurate queue depth** via `len()`.
7. **MemoryIngress reports `is_empty()` correctly** — true when empty, false otherwise.

---

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|---|---|---|
| enqueue success | Integration | Real queue; producer/consumer interaction |
| enqueue Full error | Integration | Real queue; capacity boundary |
| dequeue FIFO | Integration | Real queue; ordering invariant |
| dequeue None on empty | Unit | Pure state check, no I/O |
| Disconnected error | Integration | Real queue; channel lifecycle |
| len() accuracy | Unit | Pure observer, no I/O |
| is_empty() accuracy | Unit | Pure observer, no I/O |
| **Static: no crossbeam_channel** | Static | `grep`/`cargo deny` enforcement |

**Rationale**: Most behaviors are integration tests because they validate real queue semantics with producer/consumer. Unit tests cover pure observer methods. The `crossbeam_channel` ban is enforced via static analysis.

---

## 3. BDD Scenarios

### Behavior 1: MemoryIngress enqueues a frame without blocking when queue has capacity

**Given**: A `MemoryIngress` queue with capacity 1, created via `MemoryIngress::bounded(capacity=1)`
**When**: A producer calls `try_submit(frame)` with a valid `IngressFrame`
**Then**: The call returns `Ok(())` immediately and the frame is enqueued

```
fn memory_ingress_try_submit_succeeds_when_queue_has_capacity()
```

**And**: `len()` returns 1 after the submit.

---

### Behavior 2: MemoryIngress returns `Full` error when queue is at capacity (non-blocking)

**Given**: A `MemoryIngress` queue with capacity 1, already containing 1 frame
**When**: A producer calls `try_submit(frame)` on the full queue
**Then**: The call returns `Err(IpcError::Full)`
**And**: The original frame remains in the queue (FIFO order preserved)

```
fn memory_ingress_try_submit_returns_full_when_queue_is_at_capacity()
```

**Error variant**:
- Given: A `MemoryIngress` queue at capacity
- When: `try_submit` is called
- Then: `IpcError::Full` is returned (NOT `Err(())`, NOT panic)

---

### Behavior 3: MemoryIngress dequeues a frame in FIFO order when queue is non-empty

**Given**: A `MemoryIngress` queue with capacity 2, containing frame1 then frame2
**When**: A consumer calls `try_recv()` twice
**Then**: First call returns `Ok(Some(frame1))`, second returns `Ok(Some(frame2))`

```
fn memory_ingress_try_recv_returns_fifo_order_when_queue_has_items()
```

**And**: After both dequeues, `is_empty()` returns `true` and `len()` returns 0.

---

### Behavior 4: MemoryIngress returns `None` on empty dequeue (non-blocking)

**Given**: A `MemoryIngress` queue with capacity 1, containing zero frames
**When**: A consumer calls `try_recv()` on the empty queue
**Then**: The call returns `Ok(None)` immediately (NOT an error, NOT a panic)

```
fn memory_ingress_try_recv_returns_none_when_queue_is_empty()
```

---

### Behavior 5: MemoryIngress returns `Disconnected` error when the sender side is dropped

**Given**: A `MemoryIngress` queue where the internal sender has been dropped via `disconnect_sender()`
**When**: A consumer calls `try_recv()`
**Then**: The call returns `Err(IpcError::Disconnected)`

```
fn memory_ingress_try_recv_returns_disconnected_when_sender_dropped()
```

**Note**: The current implementation uses `crossbeam_channel` internally, but after migration to `ArrayQueue<T, RingFlagged>`, the "disconnected" semantics must be preserved — `ArrayQueue` is SPSC and dropping the sender must signal disconnection to the consumer. This requires `RingFlagged` to track sender liveness.

---

### Behavior 6: MemoryIngress reports accurate queue depth via `len()`

**Given**: A `MemoryIngress` queue with capacity 3, containing 2 frames
**When**: `len()` is called
**Then**: It returns 2

```
fn memory_ingress_len_returns_exact_count_when_queue_has_two_frames()
```

---

### Behavior 7: MemoryIngress reports `is_empty()` correctly

**Given**: A `MemoryIngress` queue with capacity 1, containing 0 frames
**When**: `is_empty()` is called
**Then**: It returns `true`

**Given**: A `MemoryIngress` queue with capacity 1, containing 1 frame
**When**: `is_empty()` is called
**Then**: It returns `false`

```
fn memory_ingress_is_empty_returns_true_when_queue_has_no_frames()
fn memory_ingress_is_empty_returns_false_when_queue_has_one_frame()
```

---

## 4. Proptest Invariants

### Proptest: `try_submit` / `try_recv` cycle invariance

**Invariant**: For any sequence of `N` successful `try_submit` calls followed by `N` successful `try_recv` calls, the dequeued frames must be in the same order as submitted (FIFO). The queue must be empty after all dequeues.

**Strategy**:
- `capacity`: `any::<NonZeroUsize>` (capped to 1024 for test speed)
- `frame_count`: `1..=capacity`
- Generate `IngressFrame` values with arbitrary `RunId`, `WorkflowDigest`, and `Bytes`

**Anti-invariant**: Submitting `capacity + 1` frames must result in exactly `capacity` successes and 1 `Full` error, with no frames lost from the successful submits.

---

### Proptest: `len()` and `is_empty()` consistency

**Invariant**: `is_empty() == (len() == 0)` must hold for all states of the queue, regardless of interleaved send/receive operations.

**Strategy**:
- Arbitrary sequence of `try_submit` and `try_recv` operations
- After each operation, assert `is_empty() == (len() == 0)`

---

### Proptest: capacity boundary at capacity=1

**Invariant**: A capacity-1 queue must exhibit correct full/empty signaling: first submit succeeds, second submit returns `Full`, first recv returns the frame, second recv returns `None`.

**Strategy**:
- Capacity fixed at 1
- Sequence: submit×2, recv×2
- Assertions at each step

---

## 5. Fuzz Targets

### Fuzz Target: `IngressFrame::new` binary payload parsing

**Input type**: `bytes::Bytes` (raw frame payload)
**Risk**: Panic from out-of-bounds access, logic error in payload size validation, wrong error variant returned
**Corpus seeds**:
- Empty payload
- Payload at exactly `MaxPayloadBytes::DEFAULT` boundary (1 MiB)
- Payload at `MaxPayloadBytes::DEFAULT + 1`
- Payload of size 0
- Single byte payloads of varying values

**Target function**: `IngressFrame::new(run_id, workflow, payload, max)` where `payload: Bytes`

**Rationale**: `IngressFrame::new` is the parsing boundary where raw bytes enter the IPC system. Bugs here cause frame corruption or crashes.

---

## 6. Kani Harnesses

### Kani Harness: `RingFlagged` sender-dropped detection

**Property**: When the only sender in an SPSC queue is dropped, the receiver must observe `Disconnected` on the next `try_recv` call (not panic, not loop forever).

**Bound**: Queue capacity 1..=8, single sender, single receiver, sender dropped before any `try_recv`.

**Rationale**: `ArrayQueue` from `crossbeam_queue` does NOT have a disconnected state — it just reports empty. `RingFlagged` must add a flag to signal sender death. Formal verification is required because this is a protocol state machine: `Live → SenderDropped → Disconnected`, and the transition must be sound with respect to all interleavings.

**Harness sketch**:
```rust
#[kani::proof]
fn ring_flagged_disconnected_detected_on_sender_drop() {
    // Let capacity = kani::any::<NonZeroUsize>() bound to [1, 8]
    // Create RingFlaggedQueue<T> with capacity
    // Get sender and receiver handles
    // Drop sender
    // Assert next recv returns Disconnected (or equivalent)
}
```

---

### Kani Harness: `ArrayQueue` SPSC semantics — no data race

**Property**: In an SPSC queue, there is exactly one producer thread and one consumer thread. The producer never reads from the queue and the consumer never writes to the queue. Therefore, there can be no data race by construction.

**Bound**: Queue capacity 1..=8, bounded number of enqueue/dequeue operations (2..=16).

**Rationale**: The SPSC discipline guarantees no data races IF the implementation is correct. Kani can verify the discipline holds by checking that:
1. Producer side only writes to head/tail indices and data slots
2. Consumer side only reads from head/index and data slots
3. Atomic operations are properly ordered (no torn reads/writes)

**Note**: This harness verifies the `ArrayQueue<T, RingFlagged>` implementation, not `crossbeam_queue::ArrayQueue` itself (which is trusted). The custom `RingFlagged` wrapper is the novel component.

---

## 7. Mutation Checkpoints

**Critical mutations to survive**:

| Mutation | Target | Catch mechanism |
|---|---|---|
| Replace `ArrayQueue::push` with unconditional loop | `try_submit` → always succeeds even when full | `try_submit_returns_full_when_queue_is_at_capacity` |
| Remove sender-dropped flag check | `try_recv` → returns `None` instead of `Disconnected` | `try_recv_returns_disconnected_when_sender_dropped` |
| Swap head/tail index on consumer side | FIFO ordering broken | `try_recv_returns_fifo_order_when_queue_has_items` |
| Remove capacity boundary check | `try_submit` never returns `Full` | `try_submit_returns_full_when_queue_is_at_capacity` |
| Use `usize::MAX` instead of actual `len()` | `len()` always returns wrong value | `len_returns_exact_count` + proptest invariant |

**Threshold**: ≥ 90% mutation kill rate on `ingress.rs` and `bounded.rs`.

---

## 8. Combinatorial Coverage Matrix

### Unit: `IngressFrame::new`

| Scenario | Input | Expected Output | Layer |
|---|---|---|---|
| happy path | valid `Bytes`, valid `RunId`, `WorkflowDigest` | `Ok(IngressFrame)` | unit |
| empty payload at min max | `Bytes::new()`, `NonZeroUsize::MIN` | `Ok(IngressFrame)` | unit |
| payload at exactly max | `Bytes::from(vec![0u8; DEFAULT])`, `DEFAULT` | `Ok(IngressFrame)` | unit |
| payload over max | `Bytes::from(vec![0u8; DEFAULT+1])`, `DEFAULT` | `Err(IpcError::PayloadTooLarge)` | unit |
| zero max | `Bytes::new()`, `NonZeroUsize::MIN` | `Ok(IngressFrame)` | unit |

### Integration: `MemoryIngress` (SPSC contract)

| Scenario | Input | Expected Output | Layer |
|---|---|---|---|
| capacity 1: submit×1 | 1 frame | `Ok(())`, `len()==1` | integration |
| capacity 1: submit×2 | 2 frames | 1st `Ok(())`, 2nd `Err(Full)` | integration |
| capacity N: submit N then recv N | N frames | FIFO order, `len()==0` after | integration |
| recv on empty | 0 frames | `Ok(None)` | integration |
| sender drop then recv | sender dropped | `Err(Disconnected)` | integration |
| submit/recv interleaved | alternating ops | correct FIFO | integration |

---

## 9. Static Analysis Gates

1. **`cargo deny` check**: `crossbeam_channel` must NOT appear in `vb_ipc/Cargo.toml` or any transitive dependency's `Cargo.lock`. Add to `deny.toml`:
   ```toml
   [bans]
   name = "crossbeam-channel"
   ```
2. **`grep` sweep**: Zero occurrences of `crossbeam_channel` in `crates/vb_ipc/src/**/*.rs` after migration.
3. **`#![forbid(unsafe_code)]`**: `ingress.rs` must remain `#![forbid(unsafe_code)]` — the lock-freedom guarantee comes from the type system, not unsafe blocks.

---

## Open Questions

1. **`RingFlagged` semantics for disconnection**: `crossbeam_queue::ArrayQueue` has no concept of "disconnected" — it simply returns `None` when empty. Does `RingFlagged` use a `bool` flag (sender dropped = `true`) alongside the array? If so, does `try_recv` check the flag before checking the array? **This needs architectural clarification before Kani harness can be finalized.**

2. **SPSC discipline enforcement**: The lock-freedom guarantee assumes single-producer/single-consumer usage. Should `MemoryIngress` use `!Send`/`!Sync` markers to enforce this at the type level, or is discipline enforced at the call site? **Answer affects Kani proof scope.**

3. **Integration test threading model**: Should integration tests spawn separate threads for producer/consumer, or use a single-threaded sequential interleaving? Both are valid SPSC tests but probe different bug classes.

4. **Migration vs. greenfield**: Should the test plan cover both the `crossbeam_channel` baseline (proving current behavior) and the `ArrayQueue<T, RingFlagged>` replacement (proving new behavior matches)? Or is this plan only for the post-migration state?
