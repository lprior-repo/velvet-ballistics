# Test Plan: vb-0253.1 — ShardCommandQueue Wrapper

## Summary
- Bead: vb-0253.1
- Feature: Wrap `crossbeam_queue::ArrayQueue<ShardCommand>` behind a domain-named `ShardCommandQueue` boundary in `vb_runtime`
- READY obligations: 6 (verify-standard)
- DEFERRED_GLOBAL: 15 (Verus proofs, TLA+, proptest — require implementation first)
- Behaviors identified: 8
- Trophy allocation: 6 unit / 2 integration / 0 e2e / 0 static (cargo test only)

---

## 1. Behavior Inventory

| # | Behavior | Subject | Action | Outcome |
|---|----------|---------|--------|---------|
| B1 | QueueFull error returned deterministically | `enqueue` | called when queue is at capacity | Returns `Err(RuntimeError::QueueFull)` |
| B2 | Failed enqueue leaves state unchanged | `enqueue` | called when queue is at capacity | `len()`, `remaining_capacity()`, `is_full()` unchanged from before call |
| B3 | Queue length starts at 0 | fresh queue | construction | `len() == 0` |
| B4 | Queue length increments on enqueue | `enqueue` | successful call | `len()` increases by exactly 1 |
| B5 | Remaining capacity decrements on enqueue | `enqueue` | successful call | `remaining_capacity()` decreases by exactly 1 |
| B6 | is_full is false initially | fresh queue | construction | `is_full() == false` |
| B7 | is_full is true at capacity | `enqueue` | queue filled to capacity | `is_full() == true` |
| B8 | Capacity returns configured value | `capacity()` | after construction | Returns value passed to `new(capacity)` |

---

## 2. Trophy Allocation

| Obligation | Layer | Behavior | Evidence |
|------------|-------|----------|----------|
| TEST-QUEUEFULL-001 | cargo test (unit) | B1 | `chunk_026::vb1u88_queue_full_at_capacity_boundary` |
| TEST-QUEUEFULL-002 | cargo test (unit) | B2 | `chunk_025::vb1u88_invariant_queue_len_never_exceeds_capacity` (line 169) |
| TEST-QUEUE-STATUS-001 | cargo test (unit) | B3, B4 | `chunk_011::shard_command_queue_len_starts_at_zero`, `shard_command_queue_len_increments_on_enqueue` |
| TEST-QUEUE-STATUS-002 | cargo test (unit) | B5, B6, B7 | `chunk_012::shard_remaining_capacity_decrements_on_enqueue`, `shard_is_queue_full_returns_false_initially`, `shard_is_queue_full_returns_true_when_at_capacity` |
| TEST-CAPACITY-001 | cargo test (unit) | B8 | `impl_tests/chunk_001::enqueue_and_capacity_tracking`, `shard_command_queue_capacity_returns_configured_value` |
| API-COMPAT-001 | cargo test (semver) | API surface | `cargo semver-checks --workspace --package vb_runtime` |

---

## 3. BDD Scenarios

### B1: QueueFull returned when at capacity

**Scenario: `enqueue_returns_queue_full_when_at_capacity`**
```
Given: a ShardCommandQueue with capacity=2, 2 commands enqueued
When:  enqueue(ShardCommand::Shutdown) is called a third time
Then:  Err(RuntimeError::QueueFull) is returned
```

Test: `chunk_026::vb1u88_queue_full_at_capacity_boundary`
```rust
// line 3-18
let config = ShardConfig { command_queue_capacity: 2, ... };
let shard = Shard::new(config);
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));
```

**Scenario: QueueFull is deterministic (same state → same result)**
```
Given: a ShardCommandQueue at capacity
When:  enqueue is called twice in the same state
Then:  both calls return Err(RuntimeError::QueueFull)
```

---

### B2: Failed enqueue leaves queue state unchanged

**Scenario: `queue_state_unchanged_after_queue_full`**
```
Given: a ShardCommandQueue with capacity=3, 3 commands enqueued (full)
When:  a fourth enqueue fails with QueueFull
Then:  len() == 3, remaining_capacity() == 0, is_full() == true (unchanged from before the failed call)
```

Test: `chunk_025::vb1u88_invariant_queue_len_never_exceeds_capacity`
```rust
// lines 155-172
let config = ShardConfig { command_queue_capacity: 3, ... };
let shard = Shard::new(config);
for _ in 0..3 {
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
}
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Err(RuntimeError::QueueFull));
assert!(shard.command_queue.len() <= shard.command_queue.capacity());
```

Note: The test at line 171 uses direct field access `shard.command_queue.len()`. After wrapper introduction, this must use `shard.command_queue.len()` through the public wrapper API (i.e., `ShardCommandQueue::len()`). The direct field access test must be updated to use `shard.command_queue_len()` public method. **This is a known finding flagged in domain-model-review.md (finding #2).**

---

### B3: Queue length starts at 0

**Scenario: `shard_command_queue_len_starts_at_zero`**
```
Given: a fresh Shard constructed with ShardConfig
When:  command_queue_len is queried before any enqueue
Then:  the result is 0
```

Test: `chunk_011::shard_command_queue_len_starts_at_zero`
```rust
// lines 254-260
let config = small_config();
let shard = Shard::new(config);
assert_eq!(shard.command_queue_len(), 0);
```

---

### B4: Queue length increments on enqueue

**Scenario: `shard_command_queue_len_increments_on_enqueue`**
```
Given: a Shard with capacity=4, empty queue
When:  enqueue(Shutdown) is called twice
Then:  command_queue_len is 1 after first enqueue, 2 after second
```

Test: `chunk_011::shard_command_queue_len_increments_on_enqueue`
```rust
// lines 263-279
let config = ShardConfig { command_queue_capacity: 4, ... };
let shard = Shard::new(config);
assert_eq!(shard.command_queue_len(), 0);
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.command_queue_len(), 1);
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.command_queue_len(), 2);
```

---

### B5: Remaining capacity decrements on enqueue

**Scenario: `shard_remaining_capacity_decrements_on_enqueue`**
```
Given: a Shard with capacity=4
When:  enqueue(Shutdown) is called twice
Then:  remaining_capacity is 2 after second enqueue (started at 4)
```

Test: `chunk_012::shard_remaining_capacity_decrements_on_enqueue`
```rust
// lines 3-19
let config = ShardConfig { command_queue_capacity: 4, ... };
let shard = Shard::new(config);
assert_eq!(shard.remaining_capacity(), 4);
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.remaining_capacity(), 3);
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.remaining_capacity(), 2);
```

---

### B6: is_full is false initially

**Scenario: `shard_is_queue_full_returns_false_initially`**
```
Given: a fresh Shard
When:  is_queue_full is queried
Then:  false is returned
```

Test: `chunk_012::shard_is_queue_full_returns_false_initially`
```rust
// lines 40-46
let config = small_config();
let shard = Shard::new(config);
assert_eq!(shard.is_queue_full(), false);
```

---

### B7: is_full is true at capacity

**Scenario: `shard_is_queue_full_returns_true_when_at_capacity`**
```
Given: a Shard with capacity=2, 2 commands enqueued (full)
When:  is_queue_full is queried
Then:  true is returned
```

Test: `chunk_012::shard_is_queue_full_returns_true_when_at_capacity`
```rust
// lines 48-64
let config = ShardConfig { command_queue_capacity: 2, ... };
let shard = Shard::new(config);
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
assert_eq!(shard.is_queue_full(), true);
```

---

### B8: Capacity returns configured value

**Scenario: `shard_command_queue_capacity_returns_configured_value`**
```
Given: a Shard constructed with command_queue_capacity=512
When:  command_queue_capacity() is called
Then:  512 is returned
```

Test: `chunk_012::shard_command_queue_capacity_returns_configured_value`
```rust
// lines 67-79
let config = ShardConfig { command_queue_capacity: 512, ... };
let shard = Shard::new(config);
assert_eq!(shard.command_queue_capacity(), 512);
```

---

## 4. Proptest Invariants

**Note:** Proptest obligations (PROPTEST-INV-002, PROPTEST-INV-003, PROPTEST-POST-002) are **DEFERRED_GLOBAL** — they require implementation to exist first and will run from state 7 onwards after implementation.

```
### PROPTEST-INV-002: len_bounded
Invariant: 0 <= len() <= capacity() holds across randomized enqueue/pop sequences
Strategy: any(capacity in 1..=1024), random enqueue/pop interleaving

### PROPTEST-INV-003: remaining_capacity_correct
Invariant: remaining_capacity() == capacity() - len() holds after any sequence of enqueue/pop
Strategy: same as INV-002

### PROPTEST-POST-002: enqueue contract
Invariant: enqueue returns Ok iff space available; Err(RuntimeError::QueueFull) when full; state unchanged on Err
Strategy: fill to random level, attempt enqueue, verify exact state change or lack thereof
```

---

## 5. Fuzz Targets

No new fuzz targets introduced by this wrapper. The `ArrayQueue` is already battle-tested in `crossbeam`. No new parsing or deserialization boundaries are introduced.

---

## 6. Kani Harnesses

No Kani harnesses are part of the READY obligations. The Verus proofs (DEFERRED_GLOBAL) cover the invariant obligations formally.

---

## 7. Mutation Checkpoints

**Threshold: ≥90% mutation kill rate**

Critical mutations that tests must catch:

| Mutation | Target behavior | Required test |
|---------|-----------------|---------------|
| Change `enqueue` to ignore push failure | B1: QueueFull not returned when full | `vb1u88_queue_full_at_capacity_boundary` |
| Change `enqueue` to update len on failure | B2: State changed after QueueFull | `vb1u88_invariant_queue_len_never_exceeds_capacity` |
| Remove len increment on successful enqueue | B4: len not incremented | `shard_command_queue_len_increments_on_enqueue` |
| Remove remaining_capacity decrement on enqueue | B5: remaining_capacity not decremented | `shard_remaining_capacity_decrements_on_enqueue` |
| Wrong is_full condition (e.g., `len > 0`) | B6, B7: is_full wrong | `shard_is_queue_full_returns_false_initially`, `shard_is_queue_full_returns_true_when_at_capacity` |

---

## 8. Combinatorial Coverage Matrix

### Queue Status Methods (chunk_011, chunk_012)

| Scenario | Input | Expected output | Test |
|----------|-------|----------------|------|
| len starts at 0 | fresh shard | 0 | `shard_command_queue_len_starts_at_zero` |
| len increments after 1 enqueue | 1 successful enqueue | 1 | `shard_command_queue_len_increments_on_enqueue` |
| len increments after 2 enqueues | 2 successful enqueues | 2 | `shard_command_queue_len_increments_on_enqueue` |
| remaining_capacity starts at capacity | fresh shard, cap=4 | 4 | `shard_remaining_capacity_decrements_on_enqueue` |
| remaining_capacity decrements | 2 successful enqueues, cap=4 | 2 | `shard_remaining_capacity_decrements_on_enqueue` |
| remaining_capacity is 0 when full | 2 enqueues, cap=2 | 0 | `shard_remaining_capacity_is_zero_when_full` |
| is_full is false initially | fresh shard | false | `shard_is_queue_full_returns_false_initially` |
| is_full is true at capacity | 2 enqueues, cap=2 | true | `shard_is_queue_full_returns_true_when_at_capacity` |
| capacity returns configured value | cap=512 | 512 | `shard_command_queue_capacity_returns_configured_value` |

### QueueFull Behavior (chunk_025, chunk_026)

| Scenario | Input | Expected output | Test |
|----------|-------|----------------|------|
| enqueue returns QueueFull at capacity | cap=2, 2 enqueues, 3rd attempt | Err(RuntimeError::QueueFull) | `vb1u88_queue_full_at_capacity_boundary` |
| len unchanged after QueueFull | cap=3, 3 enqueues, 4th attempt | len=3 (unchanged) | `vb1u88_invariant_queue_len_never_exceeds_capacity` |
| remaining_capacity unchanged after QueueFull | cap=3, 3 enqueues, 4th attempt | remaining=0 (unchanged) | verified by `shard_remaining_capacity_is_zero_when_full` + 4th enqueue failure |
| is_full unchanged after QueueFull | cap=3, 3 enqueues, 4th attempt | is_full=true (unchanged) | verified by `shard_is_queue_full_returns_true_when_at_capacity` + 4th enqueue failure |

### Configuration Validation (impl_tests/chunk_001, chunk_012)

| Scenario | Input | Expected output | Test |
|----------|-------|----------------|------|
| capacity=512 returns configured value | ShardConfig{cap=512} | command_queue_capacity()==512 | `shard_command_queue_capacity_returns_configured_value` |
| capacity=1 minimum valid | ShardConfig::new(1,...) | Ok | `config_new_accepts_min_valid_capacity` |
| capacity=0 rejected | ShardConfig::new(0,...) | Err(CommandQueueCapacityExceeded) | `config_new_rejects_zero_capacity` |
| capacity>MAX rejected | ShardConfig::new(65537,...) | Err(CommandQueueCapacityExceeded) | `config_new_rejects_capacity_exceeding_max` |

---

## 9. Open Questions

| # | Question | Resolution |
|---|----------|------------|
| OQ-1 | `chunk_025.rs` line 171 uses `shard.command_queue.len()` direct field access. After `ArrayQueue` moves to `ShardCommandQueue` wrapper, does this test still compile? | Likely NO — the field type changes from `ArrayQueue` to `ShardCommandQueue`. The test must use `shard.command_queue_len()` public method. Flagged in domain-model-review.md finding #2. |
| OQ-2 | The existing tests in chunk_011/012 call `shard.enqueue()` and `shard.tick()`. Are these Shard methods that wrap ShardCommandQueue, or do they directly use ArrayQueue? | These are Shard's public methods. After the wrapper is introduced, Shard will delegate to ShardCommandQueue internally. The tests should not need to change if Shard's public API is preserved. |
| OQ-3 | Does `impl_tests/chunk_001::queue_full_at_capacity_boundary` (line 175-192) conflict with `chunk_026::vb1u88_queue_full_at_capacity_boundary`? | No conflict — they test the same behavior from different angles. `impl_tests/chunk_001` tests through Shard's public API; `chunk_026` also uses Shard's public API. |

---

## 10. Known Findings

| Finding | Source | Impact | Required Action |
|---------|---------|--------|-----------------|
| chunk_025.rs line 171 direct field access `shard.command_queue.len()` | domain-model-review.md finding #2 | Test will fail to compile after wrapper introduction | Update to use `shard.command_queue_len()` |
| PROPTEST and Verus obligations are DEFERRED_GLOBAL | proof-obligations.planned.jsonl | Not executable until implementation exists | Run from state 7 after implementation |
