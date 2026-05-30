# Architectural Drift Report: `counters.rs`

**File**: `crates/vb_runtime/src/counters.rs`
**Total Lines**: 539
**Production Lines**: ~52 (1-52, 476-539)
**Test Lines**: ~420 (inline, 54-474)
**Status**: 🔨 HAMMERED — MULTIPLE CRITICAL VIOLATIONS

---

## VIOLATION 1: <300 LINE RULE (CRITICAL)

**Finding**: File is **539 lines**, exceeding the 300-line hard limit by 239 lines (79.7% over).

**Root Cause**: 420 lines of inline tests (`#[cfg(test)]` block, lines 54–474) are embedded in the production source file.

**Hammer Judgment**:
- Tests are NOT production code. They are executable specifications that belong in `tests/`.
- The workspace convention (`crates/*/tests/*.rs`, `crates/*/benches/*.rs`) exists precisely to prevent this.
- The `vb_runtime/tests/` directory already hosts 16 integration test files. Nothing prevents `counters_integration.rs` from existing there.
- Inline unit tests are tolerable for tiny types (5–20 line structs), not 420-line test blocks.

**Required Refactor**:
```
MOVE: lines 54-474  →  crates/vb_runtime/tests/counters_bdd_tests.rs
KEEP: lines 1-52    →  crates/vb_runtime/src/counters.rs (production)
KEEP: lines 476-539 →  crates/vb_runtime/src/counters.rs (metric snapshots)
```

---

## VIOLATION 2: PRIMITIVE OBSESSION — `CounterSnapshot` (HIGH)

**Finding**: `CounterSnapshot` (lines 476–487) exposes four raw `u64` fields.

```rust
pub struct CounterSnapshot {
    pub runs_submitted: u64,      // primitive: RunCount
    pub runs_completed: u64,      // primitive: RunCount
    pub runs_failed: u64,        // primitive: RunCount
    pub steps_executed: u64,     // primitive: StepCount
}
```

**Domain Problem**:
- `runs_submitted`, `runs_completed`, `runs_failed` are conceptually the same type: **a run count**. They differ in *meaning* (lifecycle stage), not *unit*.
- `steps_executed` is a different unit: **step count**. Cannot be added to a run count — yet the struct allows it freely.
- No `RunCount`, `StepCount`, or `SubmittedCount` newtype exists.
- Arithmetic on these fields happens in tests (lines 150–156: `saturating_add`) without a domain boundary.

**Scott Wlaschin Violation**: "Make illegal states unrepresentable." With `u64` for everything, nothing prevents `snap.runs_submitted + snap.steps_executed` (adding apples and oranges).

**Required Refactor**:
```rust
/// Run count — always non-negative, bounded by u32::MAX for practical limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunCount(u64);

/// Step count — always non-negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepCount(u64);

/// Snapshot of all shard counters.
pub struct CounterSnapshot {
    pub runs_submitted: RunCount,
    pub runs_completed: RunCount,
    pub runs_failed: RunCount,
    pub steps_executed: StepCount,
}
```

---

## VIOLATION 3: PRIMITIVE OBSESSION — `ShardMetricsSnapshot` (HIGH)

**Finding**: `ShardMetricsSnapshot` (lines 501–522) bundles seven raw primitives:

```rust
pub struct ShardMetricsSnapshot {
    pub shard_id: u32,                    // primitive: ShardId
    pub active_runs: u32,                // primitive: ActiveRunCount
    pub command_queue_depth: u32,        // primitive: QueueDepth
    pub command_queue_remaining: u32,    // primitive: QueueRemaining
    pub pending_timers: u32,              // primitive: TimerCount
    pub frame_pool_free: u32,            // primitive: PoolFree
    pub frame_pool_total: u32,            // primitive: PoolCapacity
    pub trace_ring_fill_pct: f32,        // primitive: FillPercentage
    pub counters: CounterSnapshot,
}
```

**Domain Problems**:
- `shard_id` is an identifier, not a count — mixing identity with quantity.
- `command_queue_depth + command_queue_remaining = frame_pool_total` (implied invariant), but nothing enforces this.
- `frame_pool_free <= frame_pool_total` (implied invariant) not enforced.
- `trace_ring_fill_pct` as raw `f32` allows any value 0.0–∞; logically it must be 0.0–100.0.
- Six distinct domain concepts collapsed into one undifferentiated bag of `u32`.

**Required Refactor**:
```rust
/// Shard identifier — stable across the shard's lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShardId(u32);

/// Queue depth — non-negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueueDepth(u32);

/// Frame pool capacity — non-negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolCapacity(u32);

/// Frame pool free slots — non-negative, never exceeds capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolFree(u32);

/// Trace ring fill percentage — always 0.0..=100.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillPercentage(f32);

impl FillPercentage {
    pub const fn new(raw: f32) -> Self {
        Self(raw.clamp(0.0, 100.0))
    }
}
```

---

## VIOLATION 4: PRIMITIVE OBSESSION — `RuntimeMetricsSnapshot` (HIGH)

**Finding**: `RuntimeMetricsSnapshot` (lines 524–539):

```rust
pub struct RuntimeMetricsSnapshot {
    pub shards: Vec<ShardMetricsSnapshot>,  // no encapsulation
    pub runs_active: u32,                    // primitive
    pub runs_waiting: u32,                  // primitive
    pub runs_failed_total: u64,             // primitive
    pub runs_finished_total: u64,           // primitive
    pub steps_total: u64,                   // primitive
}
```

**Domain Problems**:
- `Vec<ShardMetricsSnapshot>` leaks the container. Should be `Box<[ShardId]>` or a domain collection.
- `runs_failed_total` and `runs_finished_total` should derive from aggregating `CounterSnapshot` values, not be duplicated.
- `steps_total` is a `StepCount` — should not be a raw `u64`.

---

## VIOLATION 5: MISSING VALUE OBJECT BEHAVIOR (MEDIUM)

**Finding**: `CounterSnapshot` is labeled a "value object" but has no behavior beyond field access.

Per DDD, a value object should:
1. Have no identity (✓ — it's `Copy`)
2. Be immutable when extracted (✗ — fields are `pub`)
3. Have domain logic attached (✗ — just raw fields)

**Current**:
```rust
// A data bag, not a value object
pub struct CounterSnapshot { pub runs_submitted: u64, ... }
```

**Missing behaviors** that should exist:
- `CounterSnapshot::zero()` or `default()` — explicit zero state
- `CounterSnapshot::saturating_add(other: &Self) -> Self` — safe aggregation
- `CounterSnapshot::runs_total() -> u64` — derived metric
- `CounterSnapshot::success_rate() -> f64` — computed ratio
- `impl Add for CounterSnapshot` — aggregation

---

## VIOLATION 6: NO ERROR DOMAIN (LOW)

**Finding**: No `CounterError` type exists.

For a counter subsystem, relevant errors include:
- Overflow detection (though `AtomicU64::fetch_add` wraps, we may want to *know* it happened)
- Uninitialized read (if ever accessed before being set)

**Missing**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterError {
    Overflow { field: &'static str, attempted: u64 },
}
```

---

## VIOLATION 7: ORDERING::RELAXED WITHOUT DOCUMENTATION (LOW)

**Finding**: All atomic operations use `Ordering::Relaxed` with no rationale.

```rust
self.runs_submitted.fetch_add(1, Ordering::Relaxed);
```

**Problem**: For metrics that will be snapshotted and compared, `Relaxed` means no happens-before guarantee across threads. If thread A writes `runs_completed = 1` and thread B reads a snapshot, the other fields in that snapshot may be from a *different* moment in time.

**If intentional**, the struct needs:
```rust
/// All atomic operations use Relaxed ordering because [EXPLAIN WHY].
/// Snapshot consistency across fields is NOT guaranteed; this is acceptable because [REASON].
```

---

## SUMMARY TABLE

| Violation | Severity | Lines Affected | Refactor Cost |
|-----------|----------|----------------|---------------|
| >300 line file | CRITICAL | 420 (tests) | Move to `tests/` |
| Primitive: `u64` for run counts | HIGH | 476-487 | Newtypes + arithmetic traits |
| Primitive: `u32` for shard metrics | HIGH | 501-522 | Newtypes + invariants |
| Primitive: `Vec<>` in RuntimeMetrics | HIGH | 524-539 | Domain collection |
| Missing value object behavior | MEDIUM | 476-487 | Add domain methods |
| No error domain | LOW | 1-52 | Add `CounterError` |
| `Relaxed` without docs | LOW | 34-51 | Add safety documentation |

---

## RECOMMENDED REFACTOR SEQUENCE

1. **Immediately**: Move test block (lines 54–474) to `crates/vb_runtime/tests/counters_bdd_tests.rs`. File drops to ~66 lines.
2. **Phase 2**: Add `RunCount`, `StepCount`, `ShardId`, `QueueDepth`, `FillPercentage` newtypes in a new `primitives.rs` or in `counters.rs` (under 300 lines post-split).
3. **Phase 3**: Attach behavior to `CounterSnapshot`: `zero()`, `saturating_add()`, `runs_total()`, `success_rate()`.
4. **Phase 4**: Add `CounterError` enum and `impl TryFrom<CounterSnapshot> for RuntimeMetricsSnapshot` with overflow awareness.
5. **Phase 5**: Document `Ordering::Relaxed` safety case or migrate to `SeqCst` if consistency is required.

---

## ARCHITECTURAL HEALTH SCORE

| Dimension | Score | Notes |
|-----------|-------|-------|
| Line count compliance | 0/10 | 539 lines, 79.7% over limit |
| Primitive obsession | 3/10 | 12 raw primitive types exposed |
| Value object discipline | 4/10 | Copy/Debug but no domain behavior |
| Test isolation | 1/10 | Tests inline in production source |
| Error domain | 2/10 | No error type |
| Documentation | 3/10 | No safety guarantees documented |

**Overall: 2.2/10 — REQUIRES IMMEDIATE HAMMER**

---

*Generated by: arch-drift-hammer*
*Target: `crates/vb_runtime/src/counters.rs`*
*Date: 2026-05-29*
