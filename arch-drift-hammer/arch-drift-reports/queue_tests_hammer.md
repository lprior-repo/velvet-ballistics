# ARCHITECTURAL DRIFT REPORT
## Target: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/queue/tests.rs`

---

## CRITICAL VIOLATIONS

### 1. FILE SIZE: 1071 LINES (LIMIT: 300)

**SEVERITY: CATASTROPHIC**

This file is **3.57x over the limit**. It contains 45 test functions in a single monolithic module.

---

## PRIMITIVE OBSESSION VIOLATIONS

### 2. Raw `u64` Sequence Numbers
```rust
fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),  // wraps u64 at runtime, not compile-time
```
- `seq: u64` parameter accepts raw integers
- No compile-time guarantee that values are valid
- Callers use magic numbers: `make_event(run, 0)`, `make_event(run, 1)`, etc.

### 3. Raw Capacity/Batch Size Integers
```rust
JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)  // "2, 2" - what do they mean?
JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)  // "8, 4"
JournalWriterQueue::new(3, 3, StorageLimits::DEFAULT)  // "3, 3"
```
- `new(capacity: u32, batch_size: u32, ...)` takes raw primitives
- Tests pass literal integers with no semantic meaning
- No type-level distinction between capacity and batch_size

### 4. Raw `RunId::new(N)` with Magic Numbers
```rust
let run = RunId::new(1);
let run = RunId::new(10);
let run = RunId::new(20);
let run = RunId::new(30);
let run = RunId::new(40);
// ... 45 different magic numbers
```
- RunId is constructed with arbitrary integers
- No test-specific RunId type or builder
- Tests share no identity semantics

### 5. Raw `StorageLimits::DEFAULT`
```rust
JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)
```
- Always uses DEFAULT - no variation testing
- No test-specific StorageLimits builder

---

## DDD COHESION VIOLATIONS

### 6. Monolithic Test Module
All 45 tests live in a single `internal_tests` module with no grouping.

**Test Categories (inferred):**
| Category | Count | Lines |
|----------|-------|-------|
| Construction validation | 3 | 34-57 |
| Enqueue behavior | 5 | 60-116 |
| Pending counts tracking | 5 | 117-170 |
| Shutdown behavior | 4 | 172-236 |
| Flush behavior | 8 | 249-548 |
| Mixed profile | 8 | 550-651 |
| Capacity limits | 5 | 654-695 |
| Drain behavior | 4 | 698-795 |
| Partial flush counts | 2 | 797-825 |
| BatchBuilder | 4 | 827-849 |
| Edge cases | 7 | 851-1070 |

**Should be split into:**
- `construction_tests.rs`
- `enqueue_tests.rs`
- `shutdown_tests.rs`
- `flush_tests.rs`
- `capacity_tests.rs`
- `batch_builder_tests.rs`

### 7. No Test Fixtures/Builders
Every test duplicates:
```rust
let (_temp, journal) = temp_journal();
let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT)
    .expect("queue creation should succeed");
let run = RunId::new(40);
```
**DRY violation:** ~200 lines of repeated setup code.

### 8. Tests Are Not Behavior-Focused
Test names describe methods, not behaviors:
- `flush_batch_strict_only_drains_in_batches` - method name
- `drain_all_mixed_tiers_across_multiple_batches` - method name

**Should describe business rules:**
- "Strict events flush immediately regardless of batch size"
- "Shutdown always drains all pending events"

### 9. No Given/When/Then Structure
Tests mix setup, action, and assertion:
```rust
let queue = JournalWriterQueue::new(2, 2, ...)  // Given
    .expect("queue creation should succeed");
let run = RunId::new(1);
queue.enqueue_journaled(make_event(run, 0))  // When
    .expect("first enqueue should succeed");
let result = queue.enqueue_journaled(make_event(run, 1));  // Then
assert!(matches!(result, Err(JournalError::QueueFull)));
```

---

## STRUCTURAL DRIFT

### 10. Test Scope Creep
This file has grown from simple unit tests to **integration tests**:
- Creates temp directories
- Opens Fjall journals
- Writes and reads back events
- Validates journal persistence

These should live in `crates/workspace_tests/` as integration tests, not in `vb_storage/src/`.

### 11. No Test Data Builders
```rust
fn make_event(run: RunId, seq: u64) -> JournalEvent { ... }
```
This helper is primitive-obsessed:
- Takes raw `u64` instead of `EventSeq`
- Returns `JournalEvent` but callers only use `seq()` and `run`

### 12. Clippy Suppressions Are a Smell
```rust
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::unwrap_used,  // 45 unwrap/expect in tests!
    ...
)]
```
45 `unwrap_or_else|panic!` calls in tests - no error path testing.

---

## RECOMMENDATIONS

### Immediate (Hammer)
1. **Split into 6 files** at minimum: construction, enqueue, shutdown, flush, capacity, batch_builder
2. **Create test builders** for `TestQueue`, `TestEvent`, `TestRun`
3. **Replace magic numbers** with named constants

### Short-term
4. Move integration-style tests (journal replay) to `workspace_tests/`
5. Add property-based tests for capacity boundary conditions
6. Create scenario tests with Given/When/Then format

### Long-term
7. Add compile-time verified `EventSeq` type (non-zero, bounded)
8. Create `QueueConfig` type to replace `(capacity, batch_size)` tuple
9. Build test fixture library for queue tests

---

## EVIDENCE

```
File: crates/vb_storage/src/queue/tests.rs
Lines: 1071
Limit: 300
Overflow: 771 lines (357%)
Test functions: 45
Modules: 1
Suppressions: 9
unwrap/expect: ~45
```

---

**VERDICT: GUILTY OF CATASTROPHIC ARCHITECTURAL DRIFT**

This file must be decomposed into a test module tree before any further feature work.
