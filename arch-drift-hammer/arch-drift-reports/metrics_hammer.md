# Architectural Drift Report: `crates/vb_ipc/src/metrics.rs`

**Agent:** arch-drift-hammer  
**Date:** 2026-05-29  
**File:** `crates/vb_ipc/src/metrics.rs`  
**Line Count:** 995 / 300 limit → **VIOLATION: 3.3x over limit**

---

## Executive Summary

| Category | Finding | Severity |
|---|---|---|
| Line Count | 995 lines (limit: 300) | 🔴 CRITICAL |
| Primitive Obsession | 19 raw numeric fields with no newtypes | 🔴 CRITICAL |
| Test Module Bloat | 918 of 995 lines are tests (92%) | 🔴 CRITICAL |
| Domain Invariants | Zero validation on bounded fields (fill %, shard_id) | 🔴 CRITICAL |
| NaN Equality | `assert_eq!` on `f32::NAN` fails post-roundtrip | 🟡 WARN |
| No Refinement Types | `trace_ring_fill_pct` allows NaN, -1.0, >100.0 | 🟡 WARN |

---

## 1. LINE COUNT VIOLATION

```
Total:    995 lines
Limit:    300 lines
Overflow: 695 lines (3.3x the limit)

Breakdown:
  Lines  1-76:   Production type definitions (structs + derives) — 76 lines
  Lines 77-995: Test module — 918 lines (92% of file!)
```

**Root Cause:** All tests were dumped into the same file as the types being tested. A well-structured Rust file would have `metrics.rs` contain types only, with tests moved to `metrics/tests/` or `tests/metrics_tests.rs` at the crate level.

**Required Action:** Split tests into `tests/metrics_postcard_tests.rs` (roundtrip tests) and keep only types + inline `#[cfg(test)]` unit assertions in `metrics.rs`. Target ≤ 150 lines for the types file.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

Every field in every struct is a raw primitive. None are validated newtypes.

### ShardMetrics

| Field | Raw Type | Should Be | Invariant |
|---|---|---|---|
| `shard_id` | `u32` | `ShardId` | `0..=MAX_SHARD_ID` |
| `active_runs` | `u32` | `ActiveRunCount` | `>= 0` |
| `ready_queue_depth` | `u32` | `QueueDepth` | `>= 0` |
| `action_queue_depth` | `u32` | `QueueDepth` | `>= 0` |
| `timer_count` | `u32` | `TimerCount` | `>= 0` |
| `frame_pool_free` | `u32` | `FrameCount` | `0..=frame_pool_total` |
| `frame_pool_total` | `u32` | `FrameCount` | `>= frame_pool_free` |
| `trace_ring_fill_pct` | `f32` | `TraceRingFillPct` | `0.0..=100.0`, no NaN |
| `steps_total` | `u64` | `StepCount` | `>= 0` |
| `actions_total` | `u64` | `ActionCount` | `>= 0` |

### JournalMetrics

| Field | Raw Type | Should Be | Invariant |
|---|---|---|---|
| `writer_queue_depth` | `u32` | `QueueDepth` | `>= 0` |
| `total_events` | `u64` | `EventCount` | `>= 0` |
| `total_runs` | `u64` | `RunCount` | `>= 0` |

### IpcMetrics

| Field | Raw Type | Should Be | Invariant |
|---|---|---|---|
| `connected_clients` | `u32` | `ClientCount` | `>= 0` |
| `commands_processed` | `u64` | `CommandCount` | `>= 0` |

### AggregateMetrics

| Field | Raw Type | Should Be | Invariant |
|---|---|---|---|
| `runs_active` | `u32` | `ActiveRunCount` | `>= 0` |
| `runs_waiting` | `u32` | `WaitingRunCount` | `>= 0` |
| `runs_failed_total` | `u64` | `RunCount` | `>= 0` |
| `runs_finished_total` | `u64` | `RunCount` | `>= 0` |

**Required Action:** Create `crates/vb_ipc/src/metrics/types.rs` with all newtype wrappers. Use `pub struct ShardId(u32)` pattern with `impl From<u32> for ShardId`, `impl From<ShardId> for u32`, and a `fn new(val: u32) -> Option<ShardId>` that validates bounds.

---

## 3. MISSING DOMAIN INVARIANTS

### `trace_ring_fill_pct` — Bounded But Unvalidated

```rust
// Current: accepts ANY f32 including:
// - NaN (tested at line 384)
// - Negative values like -1.0 (tested at line 413)
// - Values > 100.0 (NOT tested but allowed by type)
pub trace_ring_fill_pct: f32,
```

The comment says `0.0 - 100.0` but the type does not enforce this. The test at line 384 explicitly tests NaN roundtrip — but `assert_eq!(NaN, NaN)` is **always false** in IEEE 754 floating point. The test will pass because Postcard preserves the bit pattern, but the assertion `assert!(decoded.trace_ring_fill_pct.is_nan())` only checks one side of equality, not that the full struct equals itself.

**Required Action:** Create `TraceRingFillPct(f32)` with `fn new(val: f32) -> Option<TraceRingFillPct>` that rejects NaN and values outside `0.0..=100.0`.

### `shard_id` — Unbounded

No upper bound is enforced. A `shard_id` of `u32::MAX` would be accepted despite being nonsensical.

**Required Action:** Add `const MAX_SHARD_ID: u32 = 65535` (or whatever the actual limit is) and validate in the `ShardId::new()` constructor.

---

## 4. TEST MODULE BLOAT

918 lines of tests for 5 structs. All tests follow the same 3 patterns:
1. **Roundtrip test:** encode → decode → assert_eq (22 tests)
2. **Inequality test:** clone → mutate one field → assert_ne (16 tests)
3. **Edge case test:** max values, NaN (5 tests)

These are mechanically generated test patterns, not meaningful behavioral tests. They prove serialization works, not that the domain model is correct.

**Required Action:** 
- Move all `#[test]` functions to `crates/vb_ipc/tests/metrics_postcard_tests.rs`
- Keep only one representative roundtrip test inline in `metrics.rs` as a smoke check
- Replace the field-by-field inequality tests with a single `#[quickcheck]` property test

---

## 5. ARCHITECTURAL CORRUPTION

```
Current structure:
metrics.rs (995 lines)
  ├── types (76 lines) ← DATA
  └── tests (918 lines) ← BEHAVIOR

Should be:
metrics.rs (≤150 lines) — types only + one inline smoke test
metrics/types.rs (new, ≤200 lines) — newtype wrappers
tests/metrics_postcard_tests.rs (new, ≤200 lines) — roundtrip tests
```

---

## 6. REQUIRED REFACTORING STEPS

### Step 1: Create newtype wrappers (`metrics/types.rs`)
```rust
pub struct ShardId(u32);
pub struct ActiveRunCount(u32);
pub struct QueueDepth(u32);
pub struct TimerCount(u32);
pub struct FrameCount(u32);
pub struct TraceRingFillPct(f32); // validated 0.0..=100.0, no NaN
pub struct StepCount(u64);
pub struct ActionCount(u64);
pub struct EventCount(u64);
pub struct RunCount(u64);
pub struct ClientCount(u32);
pub struct CommandCount(u64);
pub struct WaitingRunCount(u32);
```

### Step 2: Rewrite structs using newtypes
```rust
pub struct ShardMetrics {
    pub shard_id: ShardId,
    pub active_runs: ActiveRunCount,
    pub ready_queue_depth: QueueDepth,
    pub action_queue_depth: QueueDepth,
    pub timer_count: TimerCount,
    pub frame_pool_free: FrameCount,
    pub frame_pool_total: FrameCount,
    pub trace_ring_fill_pct: TraceRingFillPct,
    pub steps_total: StepCount,
    pub actions_total: ActionCount,
}
```

### Step 3: Move tests to `tests/metrics_postcard_tests.rs`

### Step 4: Verify total lines — `metrics.rs` ≤ 150, `metrics/types.rs` ≤ 200

---

## 7. VERDICT

| Rule | Status |
|---|---|
| `< 300 lines per file` | 🔴 FAIL (995 lines) |
| No primitive obsession | 🔴 FAIL (19 primitives) |
| Parse, don't validate | 🔴 FAIL (zero validation) |
| Workflows as state transitions | ⚪ N/A (pure data structs) |
| DDD cohesion | 🔴 FAIL (anemic domain model) |

**STATUS: REFACTOR REQUIRED** — File must be split, newtypes introduced, and tests relocated before this file can pass architectural review.
