# Architectural Drift Report: `frame_pool.rs`

**File**: `crates/vb_runtime/src/frame_pool.rs`  
**Total Lines**: 805  
**Threshold**: 300  
**STATUS**: CRITICAL VIOLATION — 805 lines (268% over limit)

---

## Executive Summary

`frame_pool.rs` is a bounded object pool for `RunFrame` reuse. The implementation itself (~86 lines) is clean, but the file is massively inflated by **718 lines of redundant, repetitive tests**. The production code exhibits **primitive obsession** and **silent failure** patterns that violate Scott Wlaschin DDD principles.

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Total file lines | 805 |
| Maximum allowed | 300 |
| Violation | +505 lines (268%) |
| Implementation only | ~86 lines |
| Test code | ~718 lines |
| Ratio | 8.3:1 test-to-impl |

**Required Action**: Split into two files:
- `frame_pool.rs` (implementation + inline unit tests, target ~150 lines)
- `frame_pool_spec.rs` (BDD/integration tests, target ~300 lines)

---

## 2. Primitive Obsession Violations

### 2.1 Unwrapped Domain Primitives

```rust
pub struct FramePool {
    frames: Vec<RunFrame>,
    step_count: u16,    // ← Primitive obsession
    slot_count: u16,    // ← Primitive obsession
    capacity: usize,    // ← Primitive obsession
}
```

**Missing NewTypes**:

| Field | Current Type | Should Be |
|-------|--------------|-----------|
| `step_count` | `u16` | `StepCount(u16)` with `StepCount::new(u16) -> CoreResult<Self>` |
| `slot_count` | `u16` | `SlotCount(u16)` with `SlotCount::new(u16) -> CoreResult<Self>` |
| `capacity` | `usize` | `PoolCapacity(usize)` bounded to `[1, MAX_POOL_CAPACITY]` |

### 2.2 Constructor Accepts Naked Primitives

```rust
pub fn new(step_count: u16, slot_count: u16, capacity: usize) -> CoreResult<Self>
```

Callers can confuse argument order:
```rust
FramePool::new(slot_count, step_count, capacity)  // WAIT WHICH IS WHICH?
```

**Fix**: Use typed parameters:
```rust
pub fn new(step_count: StepCount, slot_count: SlotCount, capacity: PoolCapacity) -> CoreResult<Self>
```

---

## 3. Silent Failure — "Parse, Don't Validate" Violation

### 3.1 `release()` Silently Drops Mismatched Frames

```rust
pub fn release(&mut self, frame: RunFrame) {
    if frame.step_count() == self.step_count
        && frame.slot_count() == self.slot_count
        && self.frames.len() < self.capacity
    {
        self.frames.push(frame);
    }
    // Frame is dropped when the pool is full.
}
```

**Problem**: When a frame with wrong dimensions is released, or when the pool is full, the frame vanishes silently. Callers have no indication that their frame was discarded.

**Wlaschin Principle Violated**: "Make illegal states unrepresentable" — a caller releasing a frame to the wrong pool gets no error feedback, masking programmer errors.

**Evidence**: Test `frame_pool_release_wrong_dimension_frame_is_silently_dropped` (lines 544-557) explicitly tests this silent-drop behavior as if it's correct.

**Fix**: Return a `CoreResult` or at minimum log a warning:
```rust
pub fn release(&mut self, frame: RunFrame) -> CoreResult<()> {
    if frame.step_count() != self.step_count || frame.slot_count() != self.slot_count {
        return Err(CoreError::InvalidFrameDimension { ... });
    }
    if self.frames.len() >= self.capacity {
        return Err(CoreError::PoolAtCapacity);
    }
    self.frames.push(frame);
    Ok(())
}
```

---

## 4. Implicit State Machine — Not Modeled as Explicit Transitions

### 4.1 Frame Lifecycle is Implicit

The `FramePool` manages a resource lifecycle:
- **Available** (in pool) → **InUse** (taken) → **Available** (released)

But these states are implicit in `Vec<RunFrame>` internals, not modeled as a typed state machine.

### 4.2 No Ownership Tracking

There's no tracking of how many frames are "in flight" (taken but not yet released). The pool only tracks available frames, not total outstanding.

```rust
// Missing: outstanding frame count
// available() returns only recycled frames, not total borrowed
pub fn available(&self) -> usize { self.frames.len() }
```

---

## 5. Test Quality Issues

### 5.1 Extreme Verbosity — Every Test is ~15-20 Lines

Example (lines 633-656):
```rust
#[test]
fn frame_pool_take_release_take_preserves_pool_consistency_under_rapid_cycle() {
    let mut pool = new_pool(2, 1, 1);
    for i in 1u64..=10 {
        let frame = match pool.take(RunId::new(i), StepIdx::new(0)) {
            Ok(f) => f,
            Err(_) => return,
        };
        pool.release(frame);
    }
    assert_eq!(pool.available(), 1);
    let reused = pool.take(RunId::new(99), StepIdx::new(0));
    match reused {
        Ok(f) => {
            assert_eq!(f.run_id(), RunId::new(99));
            assert_eq!(f.pc(), StepIdx::new(0));
        }
        Err(_) => { assert!(false); }
    }
}
```

This should be a 5-line property test using `proptest`.

### 5.2 Duplicate Test Coverage

Tests like `take_allocates_when_empty` (129-133) and `frame_pool_take_allocates_fresh_when_no_recycled` (363-378) test identical behavior with different names.

### 5.3 Helper Function Redundancy

```rust
fn new_pool(step_count: u16, slot_count: u16, capacity: usize) -> FramePool {
    let result = FramePool::new(step_count, slot_count, capacity);
    assert_eq!(result.as_ref().map(|_| ()), Ok(()));
    match result {
        Ok(pool) => pool,
        Err(_) => unreachable!("asserted Ok above"),
    }
}
```

This helper masks error handling and makes tests pass silently when they shouldn't.

---

## 6. Missing Domain Types

### 6.1 `PoolCapacity` NewType

```rust
// Should exist in vb_core::ids or a new frame_pool::types module
pub struct PoolCapacity(usize);

impl PoolCapacity {
    pub const MAX: usize = 4_096;

    pub fn new(n: usize) -> CoreResult<Self> {
        if n == 0 || n > Self::MAX {
            Err(CoreError::ResourceLimitExceeded { resource: "frame_pool_capacity" })
        } else {
            Ok(Self(n))
        }
    }
}
```

### 6.2 `FrameDimensions` Value Object

`step_count` and `slot_count` always appear together and represent frame dimensions. They should be a single type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDimensions {
    pub step_count: StepCount,
    pub slot_count: SlotCount,
}
```

---

## 7. Recommendations

### Refactoring Plan

1. **Create `crates/vb_runtime/src/frame_pool/types.rs`** (NEW):
   - `PoolCapacity(usize)` — bounded `[1, 4096]`
   - `StepCount(u16)` — validated non-zero
   - `SlotCount(u16)` — validated
   - `FrameDimensions` — composite of step+slot

2. **Refactor `frame_pool.rs`** (TARGET: ~180 lines):
   - Import new types
   - Change constructor signature
   - `release()` returns `CoreResult<()>`
   - Inline minimal unit tests (not BDD scenarios)

3. **Create `crates/vb_runtime/src/frame_pool/spec.rs`** (NEW, ~300 lines):
   - Move all BDD/adversarial tests here
   - Use `proptest` for rapid-cycle and capacity tests
   - Keep `mod tests` for simple sanity checks

4. **Update `mod.rs`** to expose new module structure

---

## 8. Verdict

| Criterion | Status |
|-----------|--------|
| Line count < 300 | **FAIL** (805 lines) |
| Primitive obsession | **FAIL** (3 unwrapped types) |
| Parse, don't validate | **FAIL** (silent frame drop) |
| Explicit state machine | **FAIL** (implicit lifecycle) |
| Test quality | **WARN** (excessive verbosity) |

**OVERALL**: `frame_pool.rs` requires immediate refactoring. The implementation is sound but the file organization and type safety are inadequate for a DDD codebase.
