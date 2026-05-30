# ARCH-DRIFT REPORT: action_queue.rs

**File**: `crates/vb_runtime/src/action_queue.rs`
**Total Lines**: 643
**Violation**: 214% over the 300-line hard limit
**STATUS**: CRITICAL REFACTOR REQUIRED

---

## EXECUTIVE SUMMARY

This file implements a bounded action completion queue but commits severe architectural violations:

1. **Line count**: 643 lines (LIMIT: 300) — **RED**
2. **Primitive obsession**: `BackpressureWarning` exposes raw `usize` fields — **RED**
3. **Test/Code coupling**: 406 test lines (63%) mixed with production code — **RED**
4. **Missing newtypes**: Queue depth should be a typed `QueueDepth` — **RED**
5. **Single Responsibility bleed**: Queue operations, backpressure signaling, and capacity parsing are all in one file — **RED**

---

## LINE COUNT VIOLATION

| Section | Lines |占比 |
|---------|-------|-----|
| Production code (1–236) | 236 | 37% |
| Test code (237–643) | 406 | 63% |
| **TOTAL** | 643 | 100% |

**Required splits:**
- `action_queue/mod.rs` — type definitions, errors, capacity parsing (≤150 lines)
- `action_queue/queue.rs` — `BoundedActionCompletionQueue` impl (≤150 lines)
- `action_queue/backpressure.rs` — `BackpressureWarning` and channel setup (≤80 lines)
- `action_queue/test_harness.rs` — all unit tests (can stay as `mod tests` or move to integration test)

---

## PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION 1: `BackpressureWarning` exposes raw `usize`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    pub depth: usize,      // ← primitive obsession
    pub capacity: usize,  // ← should use typed capacity
}
```

**Problem**: `depth` and `capacity` are raw `usize` but the domain has typed representations:
- `depth` should be `QueueDepth(usize)` — a newtype
- `capacity` should be `ActionQueueCapacity` (which already exists at line 21!)

**Fix required**:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    pub depth: QueueDepth,
    pub capacity: ActionQueueCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueDepth(usize);

impl QueueDepth {
    pub fn get(self) -> usize { self.0 }
    pub fn as_capacity(&self) -> ActionQueueCapacity { ActionQueueCapacity(self.0) }
}
```

### VIOLATION 2: `enqueue()` success path returns `Ok(())` with no correlation token

```rust
pub fn enqueue(&self, ticket: ActionTicket) -> Result<(), ActionQueueError>
```

**Problem**: The only success variant is `Ok(())`. If a caller needs to correlate the enqueued ticket (e.g., for debugging, logging, or tracking), they cannot. The caller must re-supply the ticket to get the seq number back.

**Fix**: Either return the `SeqNo` on success, or introduce an `Enqueued` confirmation type.

### VIOLATION 3: `ActionTicket.capacity` is raw `u16`

From `vb_core/src/action.rs` line 152:
```rust
pub capacity: u16,
```

**Problem**: This is a retry capacity bound, but it's just a raw `u16`. It should be `RetryCapacity(u16)` or similar newtype to prevent mixing with other `u16` values.

**Note**: This is in `vb_core`, so fixing it is out of scope for THIS file, but it propagates primitive obsession HERE.

---

## SINGLE RESPONSIBILITY PRINCIPLE VIOLATIONS

### BLEED 1: Backpressure logic embedded in queue

The `enqueue()` method at lines 130–157 contains backpressure signaling logic:

```rust
let threshold = backpressure_threshold(self.capacity);
if depth >= threshold
    && let Some(ref tx) = self.backpressure_tx
{
    match tx.try_send(BackpressureWarning { ... }) {
        Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
    }
}
```

**Problem**: The queue should not know about backpressure. This is a cross-cutting concern that should be handled by a decorator or wrapper.

**Fix**: Extract backpressure into `BoundedActionCompletionQueueWithBackpressure` that wraps the base queue.

### BLEED 2: Capacity parsing duplicated across constructors

Lines 206–224 contain `parse_capacity()` which is used by both `new()` and `with_backpressure()`. This is fine, but the constructors themselves duplicate the `Mutex` initialization pattern.

**Suggested**: Extract to a builder or `From<usize>` implementation on `ActionQueueCapacity`.

---

## TEST CODE REVIEW (Lines 237–643)

### PROBLEM: Tests are 406 lines mixed into production file

This violates the principle that production code and tests should be in separate files (or at minimum, separate modules at the end of the file, not 63% of the file).

### DUPLICATED TEST PATTERNS

The following test patterns are repeated with only capacity values changing:

| Pattern | Lines | Tests |
|---------|-------|-------|
| `bounded_action_queue_new_*` | 254–299 | 6 tests |
| `bounded_action_queue_enqueue_*` | 334–352 | 2 tests |
| `action_queue_emits_backpressure_*` | 411–511 | 5 tests |

**These should be parameterized tests using `proptest` or `quickcheck`.**

### EDGE CASE TEST DOCUMENTED IN CODE

Lines 487–511 document an integer-division edge case:

```rust
// At depth=15 with threshold=15, warning fires (even though 15/19=78.9% < 80%)
// This is the integer-division edge case: the implementation rounds down the threshold
```

**This is good documentation but the test itself exposes a semantic bug**: The backpressure fires at 78.9% instead of 80% for capacity=19. This is a **known limitation** that should be documented as a `#[deprecated]` or `#[note]` in the code, not just in a test comment.

---

## REQUIRED REFACTORING PLAN

### File 1: `crates/vb_runtime/src/action_queue/mod.rs` (Target: ≤150 lines)

```rust
//! Action completion queue types and errors.

use std::sync::mpsc::{Receiver, SyncSender};

pub const MAX_ACTION_COMPLETION_QUEUE_CAPACITY: usize = 65_536;

// Newtype for queue depth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueueDepth(usize);

impl QueueDepth {
    pub fn new(depth: usize) -> Self { Self(depth) }
    pub fn get(self) -> usize { self.0 }
}

// Capacity type (already exists, move here)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionQueueCapacity(usize);

// Invalid capacity reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidActionQueueCapacity {
    Zero,
    AboveMaximum { maximum: usize },
}

// Queue errors
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionQueueError {
    QueueFull { capacity: ActionQueueCapacity },
    InvalidCapacity { requested: usize, reason: InvalidActionQueueCapacity },
}

// Backpressure warning (NEW: typed fields)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    pub depth: QueueDepth,
    pub capacity: ActionQueueCapacity,
}

// Re-export queue
pub use super::queue::BoundedActionCompletionQueue;
```

### File 2: `crates/vb_runtime/src/action_queue/queue.rs` (Target: ≤150 lines)

```rust
//! Bounded action completion queue implementation.

use super::*;
use std::collections::VecDeque;

pub struct BoundedActionCompletionQueue {
    inner: std::sync::Mutex<Inner>,
    capacity: ActionQueueCapacity,
    backpressure_tx: Option<SyncSender<BackpressureWarning>>,
}

struct Inner { items: VecDeque<ActionTicket> }

// All impl methods here...
```

### File 3: `crates/vb_runtime/src/action_queue/backpressure.rs` (Target: ≤80 lines)

```rust
//! Backpressure signaling for action queues.

use super::*;

pub struct BackpressureChannel {
    tx: SyncSender<BackpressureWarning>,
    rx: Receiver<BackpressureWarning>,
}

impl BackpressureChannel {
    pub fn new(capacity: ActionQueueCapacity) -> (Self, Self) { /* ... */ }
}
```

### File 4: `crates/vb_runtime/src/action_queue/test_harness.rs` (Target: Any length, but use proptest)

Move all existing tests here. Replace repetitive tests with:

```rust
#[test_strategy::proptest]
fn backpressure_fires_at_80_percent_capacity(#[values(10, 20, 50, 100)] capacity: usize) {
    // ...
}
```

---

## SUMMARY

| Issue | Severity | Fix Effort |
|-------|----------|------------|
| 643 lines (limit 300) | CRITICAL | Medium — split into 4 files |
| `BackpressureWarning` uses raw `usize` | HIGH | Low — add `QueueDepth` newtype |
| Tests 63% of file | HIGH | Medium — move to separate module |
| `enqueue()` returns no correlation token | MEDIUM | Low — return `SeqNo` on success |
| Backpressure bleed into queue impl | MEDIUM | Medium — extract decorator |
| Integer-division edge case undocumented in production code | MEDIUM | Low — add `#[deprecated]` note |

---

## VERDICT

**ARCH-DRIFT STATUS**: `REFACTOR REQUIRED`

This file must be split before it can be accepted into the codebase. The primitive obsession in `BackpressureWarning` is the most urgent fix because it creates a type-level hole: callers can construct `BackpressureWarning { depth: 999, capacity: 999 }` which has no connection to any actual queue state.

The 80% integer-division edge case (line 487–511) should be raised as a separate bead for product decision: should we use floating-point or rational math for the threshold, or document this as a known limitation?
