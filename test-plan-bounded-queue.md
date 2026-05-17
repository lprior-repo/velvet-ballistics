# Test Plan: LETHAL-5 — Bounded Action Completion Queue

## Summary

- **Bead**: LETHAL-5
- **Requirement**: Section 4 mandates bounded action completion queue; not yet implemented.
- **Behaviors identified**: 5
- **Trophy allocation**: 8 unit / 4 integration / 1 e2e
- **Proptest invariants**: 2
- **Fuzz targets**: 0 (no parsing boundary — all inputs are typed)
- **Kani harnesses**: 2
- **Mutation checkpoints**: 6

---

## 1. Behavior Inventory

1. **Action queue rejects enqueue when at capacity** — returns `ActionQueueError::QueueFull { capacity: N }`
2. **Action queue accepts enqueue when below capacity** — returns `Ok(())`, length increases by 1
3. **Action queue emits backpressure warning at 80% capacity** — returns `Ok(())` and sends a warning notification
4. **Action queue tracks remaining capacity accurately** — `remaining_capacity()` returns correct count after enqueue/dequeue
5. **Action queue drains to empty** — after all completions are popped, `len()` is 0 and `remaining_capacity()` equals `capacity()`

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 8 | Pure queue logic: full/partial/empty states, boundary arithmetic, backpressure threshold, dequeue invariants |
| Integration | 4 | `Runtime::complete_action_with_output` wired to bounded queue; backpressure notification observed through runtime journal |
| E2E | 1 | Full submit → action → complete cycle with bounded queue full path |
| Static | ∞ | `forbid(unsafe_code)` + `cargo clippy -- -D warnings` on the new module |

---

## 3. BDD Scenarios

### Behavior: Queue rejects enqueue when full

**Scenario**: `fn action_queue_returns_queue_full_error_when_enqueue_at_capacity`

Given: A bounded action completion queue with capacity `N = 3`; 3 actions already enqueued (queue is full)
When: A 4th action completion is enqueued
Then: Returns `Err(ActionQueueError::QueueFull { capacity: 3 })`
And: Queue length remains 3

**Scenario**: `fn action_queue_returns_queue_full_error_when_enqueue_single_element_at_capacity_one`

Given: A bounded action completion queue with capacity `N = 1`; 1 action already enqueued
When: A 2nd action completion is enqueued
Then: Returns `Err(ActionQueueError::QueueFull { capacity: 1 })`

**Error variant — empty queue**:

Given: A bounded action completion queue with capacity `N = 3`; queue is empty
When: An action completion is enqueued
Then: Returns `Ok(())`
And: Queue length is 1

---

### Behavior: Queue accepts enqueue when below capacity

**Scenario**: `fn action_queue_accepts_enqueue_when_below_capacity`

Given: A bounded action completion queue with capacity `N = 4`; 2 actions already enqueued
When: A new action completion is enqueued
Then: Returns `Ok(())`
And: Queue length is 3

**Scenario**: `fn action_queue_accepts_enqueue_at_exactly_one_below_capacity`

Given: A bounded action completion queue with capacity `N = 5`; 4 actions already enqueued
When: A 5th action completion is enqueued
Then: Returns `Ok(())`
And: Queue length is 5

**Scenario**: `fn action_queue_len_increments_per_enqueue`

Given: A bounded action completion queue with capacity `N = 10`; queue is empty
When: 3 action completions are enqueued individually
Then: Each enqueue returns `Ok(())`
And: After 3 enqueues, queue length is exactly 3

---

### Behavior: Queue emits backpressure warning at 80% capacity

**Scenario**: `fn action_queue_emits_backpressure_warning_at_80_percent_capacity`

Given: A bounded action completion queue with capacity `N = 10`; 7 actions already enqueued (70%, below 80%)
When: An 8th action completion is enqueued (reaching exactly 80%)
Then: Returns `Ok(())`
And: A backpressure warning notification is emitted with the current depth and capacity

**Scenario**: `fn action_queue_emits_backpressure_warning_just_above_80_percent`

Given: A bounded action completion queue with capacity `N = 10`; 8 actions already enqueued (80%, exact)
When: A 9th action completion is enqueued (90%)
Then: Returns `Ok(())`
And: A backpressure warning notification is emitted

**Scenario**: `fn action_queue_does_not_emit_warning_below_80_percent`

Given: A bounded action completion queue with capacity `N = 10`; 7 actions already enqueued (70%)
When: An 8th action completion is enqueued (80%)
Then: Returns `Ok(())`
And: No backpressure warning is emitted

**Scenario**: `fn action_queue_backpressure_warning_contains_depth_and_capacity`

Given: A bounded action completion queue with capacity `N = 5`; 4 actions already enqueued (80%)
When: A 5th action completion is enqueued (100%)
Then: The backpressure warning contains the current depth (`4`) and capacity (`5`)

---

### Behavior: Queue tracks remaining capacity accurately

**Scenario**: `fn action_queue_remaining_capacity_decrements_after_enqueue`

Given: A bounded action completion queue with capacity `N = 8`; queue is empty
When: 3 action completions are enqueued
Then: `remaining_capacity()` returns 5

**Scenario**: `fn action_queue_remaining_capacity_increments_after_dequeue`

Given: A bounded action completion queue with capacity `N = 8`; 3 actions already enqueued
When: 1 action completion is dequeued
Then: `remaining_capacity()` returns 6

**Scenario**: `fn action_queue_remaining_capacity_equals_capacity_when_empty`

Given: A bounded action completion queue with capacity `N = 16`; queue is empty
When: `remaining_capacity()` is called
Then: Returns 16

**Scenario**: `fn action_queue_remaining_capacity_is_zero_when_full`

Given: A bounded action completion queue with capacity `N = 4`; 4 actions already enqueued
When: `remaining_capacity()` is called
Then: Returns 0

---

### Behavior: Queue drains to empty

**Scenario**: `fn action_queue_len_is_zero_after_draining_all_items`

Given: A bounded action completion queue with capacity `N = 5`; 5 actions already enqueued
When: All 5 action completions are dequeued in FIFO order
Then: Queue length is 0
And: `is_empty()` returns `true`
And: `remaining_capacity()` returns 5

**Scenario**: `fn action_queue_dequeue_returns_items_in_fifo_order`

Given: A bounded action completion queue with capacity `N = 3`; actions A, B, C enqueued in that order
When: Three dequeue operations are performed
Then: First dequeue returns A
And: Second dequeue returns B
And: Third dequeue returns C

**Scenario**: `fn action_queue_dequeue_returns_none_when_empty`

Given: A bounded action completion queue with capacity `N = 4`; queue is empty
When: A dequeue is performed
Then: Returns `None`

---

## 4. Proptest Invariants

### Proptest: `BoundedActionCompletionQueue::enqueue + dequeue round-trip`

**Invariant**: For any sequence of `enqueue` operations followed by the same number of `dequeue` operations, the queue returns to its original empty state (length = 0, remaining_capacity = capacity).

**Strategy**: `vec(any::<u8>(), 0..=capacity)` — generate a vector of arbitrary bytes (action payload proxies) up to the queue capacity.

**Anti-invariant**: Any operation on a queue created with capacity `0` must fail or panic.

---

### Proptest: `BoundedActionCompletionQueue::remaining_capacity`

**Invariant**: After any series of `enqueue` and `dequeue` operations, `remaining_capacity() = capacity() - len()` must hold.

**Strategy**: Interleave arbitrary `enqueue` and `dequeue` operations, tracking expected len manually.

**Anti-invariant**: Any single operation that would cause `len() > capacity` must be rejected with `QueueFull`.

---

## 5. Fuzz Targets

No fuzz targets are required for this module. The action completion queue operates on typed, structurally validated inputs (`ActionTicket` + `ActionOutputReady`), not raw bytes or untrusted deserialization boundaries. The `ArrayQueue` inside is already tested by `crossbeam_queue`'s own tests.

---

## 6. Kani Harnesses

### Kani Harness: `action_queue_capacity_invariant`

**Property**: `available <= capacity && available >= 0` after any series of enqueue/dequeue operations.

**Bound**: Search depth = 8 operations (alternating enqueue/dequeue), capacity = 10.

**Rationale**: The queue must NEVER allow `len() > capacity`. Arithmetic overflow in the capacity tracking could cause this invariant to be violated silently. This is a critical memory-safety adjacent invariant. Proptest can run millions of iterations but cannot exhaustively prove absence of overflow across all scheduling permutations.

**Implementation note**: The harness must use `kani::any()` for each enqueued action completion item — NOT hardcoded dummy data.

---

### Kani Harness: `backpressure_warning_exactly_at_80_percent`

**Property**: A backpressure warning is emitted if and only if `len >= (capacity * 8 / 10)` after an enqueue.

**Bound**: Capacity values in `{1, 2, 4, 5, 8, 10, 16, 100}`; enumerate all len values 0..=capacity.

**Rationale**: The 80% threshold is a hardcoded constant. A Kani proof covers all capacity values and all fill levels simultaneously, providing formal evidence that the warning fires at the correct boundary and never fires below it.

---

## 7. Mutation Checkpoints

Critical mutations that must be caught:

| Mutation | Test That Catches It |
|----------|----------------------|
| `enqueue` always returns `Ok(())` ignoring capacity check | `action_queue_returns_queue_full_error_when_enqueue_at_capacity` |
| `backpressure` check uses `>` instead of `>=` (fires at 81%+) | `action_queue_emits_backpressure_warning_at_80_percent_capacity` |
| `backpressure` never fires (check removed) | `action_queue_backpressure_warning_contains_depth_and_capacity` |
| `remaining_capacity` computes `capacity - len` without saturating (underflows) | `action_queue_remaining_capacity_is_zero_when_full` |
| `dequeue` returns item without decrementing len | `action_queue_len_is_zero_after_draining_all_items` |
| `is_full` checks `len == capacity + 1` instead of `len == capacity` | `action_queue_returns_queue_full_error_when_enqueue_at_capacity` |

**Threshold**: ≥90% mutation kill rate.

---

## 8. Combinatorial Coverage Matrix

### `BoundedActionCompletionQueue` (unit test group)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|----------------|------------|
| enqueue at capacity | `len == capacity` | `Err(QueueFull { capacity: N })` | unit |
| enqueue below capacity | `len < capacity` | `Ok(()); len += 1` | unit |
| enqueue at capacity-1 | `len == capacity-1` | `Ok(()); len == capacity` | unit |
| enqueue into empty queue | `len == 0` | `Ok(()); len == 1` | unit |
| dequeue from full queue | `len == capacity` | `Some(item); len -= 1` | unit |
| dequeue from empty queue | `len == 0` | `None` | unit |
| dequeue FIFO order | `A, B, C enqueued` | `A, B, C in order` | unit |
| `remaining_capacity` full | `len == capacity` | `0` | unit |
| `remaining_capacity` empty | `len == 0` | `capacity` | unit |
| `remaining_capacity` partial | `len == k` | `capacity - k` | unit |
| `is_full` true | `len == capacity` | `true` | unit |
| `is_full` false | `len < capacity` | `false` | unit |
| backpressure fires exactly at 80% | `len == capacity*8/10` | `Ok(()) + warning` | unit |
| backpressure fires above 80% | `len > capacity*8/10` | `Ok(()) + warning` | unit |
| backpressure does NOT fire below 80% | `len < capacity*8/10` | `Ok(()); no warning` | unit |
| backpressure warning payload | `capacity=5, len=4` | `Warning { depth: 4, capacity: 5 }` | unit |
| enqueue capacity=1 at capacity | `len == 1` | `Err(QueueFull { capacity: 1 })` | unit |
| enqueue capacity=1 below | `len == 0` | `Ok(()); len == 1` | unit |

### Integration: `Runtime::complete_action_with_output` with bounded queue

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|----------------|------------|
| complete_action fills queue to capacity | N completions into capacity N | Last returns `QueueFull` | integration |
| complete_action accepts below capacity | N-1 completions into capacity N | All return `Ok(())` | integration |
| backpressure warning visible in journal | 80%+ capacity reached | `RuntimeJournalEvent::BackpressureWarning` | integration |

### E2E: Full workflow with bounded queue

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|----------------|------------|
| submit → action scheduled → complete → workflow finishes, bounded queue full on last action | capacity=1 | Final completion returns `QueueFull` | e2e |

---

## 9. Error Taxonomy

The `ActionQueueError` enum must contain:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionQueueError {
    /// Queue has reached its bounded capacity; no more items can be enqueued.
    QueueFull {
        /// The fixed capacity of this queue.
        capacity: usize,
    },
}
```

This is the ONLY error variant required for the bounded action completion queue.

---

## 10. Open Questions

1. **Where is the action completion queue instantiated?** Is it a module-level singleton, per-runtime, or per-shard? The test plan assumes per-runtime (`Runtime::complete_action_with_output` uses a shared `ActionCompletionQueue`), but if it's per-shard the integration tests must adapt.

2. **What channel does the backpressure warning use?** The test plan assumes a `RuntimeJournalEvent::BackpressureWarning` variant. If there's an existing notification mechanism (metrics, tracing span, async mpsc), the test assertions must use that.

3. **What is the default queue capacity?** The plan uses illustrative values (4, 5, 10). The test-writer should pick a concrete, realistic default (e.g., 1024) and test at least one edge-case small capacity (e.g., 1 and 2).

4. **Is the queue shared across shards or per-shard?** `Runtime::complete_action_with_output` routes by `ticket.run`, so the queue could be per-shard. This affects how "queue full" is hit under load.

5. **Is `dequeue` drain-only or can it be called at arbitrary points?** The plan assumes dequeue is called by the shard tick loop to drain completed actions. If dequeue is not part of the public API, only `enqueue`-fail and `remaining_capacity` need testing.

---

## Exit Criteria

- Every BDD scenario above has a corresponding test with an exact assertion (no `is_ok()` / `is_err()` without value inspection).
- `ActionQueueError::QueueFull { capacity }` is the only error variant used.
- Backpressure warning is tested at exactly 80%, above 80%, and strictly below 80%.
- Proptest invariants hold after 10 000 iterations.
- Kani harnesses pass with the specified bounds.
- Mutation kill rate ≥ 90% on the `enqueue`, `remaining_capacity`, and `backpressure` code paths.
