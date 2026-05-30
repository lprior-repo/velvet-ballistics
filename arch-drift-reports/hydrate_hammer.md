# Architectural Drift Report: `hydrate.rs`

**File**: `crates/vb_storage/src/recovery/hydrate.rs`  
**Line Count**: 502 (exceeds 300-line limit by 202 lines)  
**Status**: 🔴 CRITICAL VIOLATION - MUST SPLIT

---

## Executive Summary

This file is a **hydration orchestra conductor** that juggles 4 distinct responsibilities:
1. **Snapshot validation** (precondition proof surfaces)
2. **Frame reconstruction** (from snapshot + tail OR events-only)
3. **Action replay tracking** (idempotency enforcement via `ActionReplayTracker`)
4. **Event classification** (counting state-mutating events)

At 502 lines, it violates the **<300 line rule** by 202 lines and exhibits **primitive obsession**, **stringly-typed errors**, and **imperative workflow encoding** instead of explicit state-transition functions.

---

## 🚨 Critical Violations

### 1. LINE COUNT EXCEEDED: 502 > 300

**Impact**: Unreviewable, unmaintainable, violates architectural contract.

**Required Splits** (in priority order):

| Split | File | Responsibility | Est. Lines |
|-------|------|-----------------|------------|
| 1 | `hydrate_snapshot.rs` | `hydrate_run_frame` + all snapshot-related validation | ~180 |
| 2 | `hydrate_events.rs` | `hydrate_run_frame_from_events` + events-only path | ~160 |
| 3 | `hydrate_shared.rs` | Shared: `TailEventMetadata`, `SnapshotRecoveryInputViolation`, `validate_*` helpers, `apply_*` helpers, `increment_executed`, `count_state_events` | ~140 |
| 4 | `hydrate_counts.rs` | `count_state_event` and event classification | ~90 |

---

### 2. PRIMITIVE OBSESSION

#### Violation A: `u64` for Executed Counter

**Location**: `increment_executed` (lines 302-313), `count_state_events` (lines 398-410)

```rust
fn increment_executed(frame: &mut vb_core::RunFrame, run_id: RunId, executed: u64) -> RecoveryResult<()>
fn count_state_events(events: &[JournalEvent], run_id: RunId) -> RecoveryResult<u64>
```

**Problem**: Raw `u64` for a domain concept "executed action count". Should be a NewType:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutedCount(u64);

impl ExecutedCount {
    pub const fn new(n: u64) -> Self;
    pub const fn get(self) -> u64;
    pub const fn zero() -> Self;
}
```

#### Violation B: `u64` loop in `increment_executed`

**Location**: Lines 307-311

```rust
for _ in 0..executed {
    frame.increment_executed()...
}
```

**Problem**: Raw `u64` iteration. If `executed` is a NewType with `Increment for ExecutedCount`, this becomes:

```rust
for _ in ExecutedCount::new(executed).iter() { ... }
```

Or better: `RunFrame::increment_executed_n(executed)` that handles the loop internally.

#### Violation C: `format!` Stringly-Typed Error Details

**Locations**: Lines 255-258, 265-269, 281, 296, 368, 383, 394, 498

```rust
detail: format!("tail event run_id mismatch: expected {expected:?}, found {actual:?}")
```

**Problem**: Stringly-typed errors prevent programmatic error recovery. Should be structured:

```rust
RecoveryErrorDetail::TailEventRunMismatch { expected: RunId, actual: RunId }
```

**Note**: `RecoveryError` is defined in `types.rs` - this is likely already structured. Verify `RecoveryError::ReplayDivergence` uses structured details, not raw strings.

---

### 3. ANEMIC ERROR ENUM: `SnapshotRecoveryInputViolation`

**Location**: Lines 93-110

```rust
pub(crate) enum SnapshotRecoveryInputViolation {
    SnapshotRunMismatch { snapshot_run: RunId, snapshot_seq: EventSeq },
    TailRunMismatch { expected: RunId, actual: RunId },
    TailSeqNotAfterSnapshot { snapshot_seq: EventSeq, actual_seq: EventSeq },
    NoRecoveryData { run: RunId },
}
```

**Problem**: This is a **good** enum but the conversion function `snapshot_input_violation_to_error` (lines 243-275) destroys its semantics by converting to stringly-typed `RecoveryError::ReplayDivergence` with `detail: String`.

**Fix**: Either:
1. Make `RecoveryError` variants carry `SnapshotRecoveryInputViolation` directly
2. Create a true domain error type `SnapshotRecoveryError` with all context preserved

---

### 4. WORKFLOW ENCODING: Imperative vs. State-Transition

**Location**: `hydrate_run_frame` (lines 182-201)

```rust
pub fn hydrate_run_frame(...) -> RecoveryResult<vb_core::RunFrame> {
    validate_snapshot_recovery_inputs(snapshot, tail_events, run_id)?;  // Step 1
    let snapshot_slots = decode_snapshot_slots(&snapshot.slots, &snapshot.taint, run_id)?;  // Step 2
    let (step_count, slot_count, first_step) = derive_dimensions_from_snapshot_and_tail(...)?;  // Step 3
    ensure_nonzero_step_count(step_count)?;  // Step 4
    let mut frame = vb_core::RunFrame::new(...)?;  // Step 5
    apply_snapshot_slots(&mut frame, &snapshot_slots)?;  // Step 6
    let mut tracker = ActionReplayTracker::new();  // Step 7
    let executed = apply_tail_events(&mut frame, tail_events, &mut tracker)?;  // Step 8
    increment_executed(&mut frame, run_id, executed)?;  // Step 9
    Ok(frame)
}
```

**Problem**: This is an imperative sequence of operations, not an explicit state machine. Each step transitions the system state, but the state transitions are implicit.

**DDD Principle**: Workflows should be modeled as explicit state transitions. Consider:

```rust
enum HydrationState {
    Validating,
    DecodingSlots,
    DerivingDimensions,
    BuildingFrame,
    ApplyingSnapshot,
    ApplyingTail,
    Complete,
}
```

Or better: Use a step-by-step builder pattern where each method consumes the previous state and produces the next.

---

## ⚠️ Warnings (Non-Blocking but Should Fix)

### A. Repetitive `map_err` Boilerplate

Every error site does:
```rust
.map_err(|_| RecoveryError::ReplayDivergence { step: vb_core::StepIdx::ZERO, detail: ... })
```

**Suggestion**: Create a helper:
```rust
fn frame_error(detail: impl Into<String>) -> RecoveryError {
    RecoveryError::ReplayDivergence { step: vb_core::StepIdx::ZERO, detail: detail.into() }
}
```

Then use `.map_err(|_| frame_error("snapshot slot write out of bounds"))`.

### B. `count_state_event` Nested Match Complexity

**Location**: Lines 412-478

The function has 9 distinct `JournalEvent` variants being matched, with deeply nested logic. This is a **code smell** - the function is doing too much.

**Suggestion**: Break into per-variant handler functions:
- `handle_action_scheduled(...) -> bool`
- `handle_action_scheduled_ticket(...) -> bool`
- `handle_action_completed_event(...) -> bool`
- etc.

### C. `apply_*` Functions Are All Identical Patterns

`apply_snapshot_slots` (287-300), `apply_seed_slots` (374-387) are nearly identical:
```rust
fn apply_X(frame: &mut RunFrame, entries: &[RecoveredSlotEntry]) -> RecoveryResult<()> {
    for entry in entries {
        frame.write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| ...)?;
    }
    Ok(())
}
```

**Suggestion**: Extract to:
```rust
fn apply_slot_entries(frame: &mut RunFrame, entries: &[RecoveredSlotEntry]) -> RecoveryResult<()>
```

---

## ✅ What's Good

1. **`TailEventMetadata`** (lines 74-90): Good NewType abstraction for run/seq pair.
2. **`hydrate_snapshot_tail_*` proof surfaces** (lines 21-65): Excellent "Parse, don't validate" approach with pure boolean checks.
3. **`const fn` validation** (lines 112-166): Good for compile-time guarantees where possible.
4. **`#[must_use]` on public functions**: Correct API design.

---

## Required Actions

1. **SPLIT FILE** into 4 parts as specified in the table above
2. **Create `ExecutedCount` NewType** for the executed counter
3. **Enforce structured errors** - no `format!` in error details
4. **Extract shared helpers** to `hydrate_shared.rs`
5. **Re-run architectural drift check** after refactor

---

## Evidence

- File line count: 502
- Functions: 15+ public/private
- Responsibility clusters: 4 distinct domains
- Primitive obsession instances: 3 confirmed (u64 for count, format! strings)
- State-transition violations: 1 major (hydrate_run_frame imperative sequence)

**VERDICT**: 🔴 **MUST REFACTOR** before any further development on this file.
