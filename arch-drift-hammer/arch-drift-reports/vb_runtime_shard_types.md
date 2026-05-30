# Architectural Drift Report: `vb_runtime/src/shard/types.rs`

**File**: `crates/vb_runtime/src/shard/types.rs`
**Analysis Date**: 2026-05-29
**Status**: CRITICAL DRIFT

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **883** | 300 | ❌ OVER LIMIT |

**Violation**: File exceeds 300-line limit by **583 lines (194% over)**

---

## 2. DDD Cohesion Analysis

### Domain Concepts Identified (7 distinct aggregates in one file)

| Domain | Types | Lines | Cohesion |
|--------|-------|-------|----------|
| Timer | `PendingTimerKind`, `PendingTimer` | 29-78 | ✓ Isolated |
| Command | `ShardCommand` (15 variants) | 82-210 | ⚠️ Large enum |
| Ask | `AskTicket`, `AskAnswer` | 212-268 | ✓ Isolated |
| Run State | `RunState` | 270-287 | ✓ Isolated |
| Introspection | `InspectSnapshot`, `InspectResponse`, `InspectHandle`, `IntrospectionRegistry`, `InspectSnapshotFormatter` | 289-530 | ❌ Mixed concerns |
| Command Queue | `ShardCommandQueue`, validators | 532-643 | ⚠️ Infrastructure |
| Shard Core | `Shard`, `ShardStatus`, `ShardHealth`, `ShardConfig` | 645-742 | ⚠️ Config/Status mixed |
| Runtime State Machine | `RuntimeState`, `RuntimeEvent` | 744-808 | ❌ State machine mixed |
| Resume | `ResumeStatus`, `ResumeResult`, `ResumeError` | 810-883 | ❌ Error domain mixed |

### Cohesion Verdict: **SMELL - God Module**

The file behaves as a "God Module" containing 7+ bounded contexts forced into proximity due to historical accretion.

---

## 3. Violations

### Line Count Violation
- **883 lines** vs **300 line maximum**
- Severity: **CRITICAL**

### DDD Cohesion Violations

| # | Violation | Type | Severity |
|---|-----------|------|----------|
| 1 | Timer domain (PendingTimer) mixed with Command/Queue domains | Boundary blur | HIGH |
| 2 | `ShardCommand` enum (15 variants, 130 lines) is an God Enum | Primitive obsession / Large enum | HIGH |
| 3 | Introspection types (`InspectHandle`, `IntrospectionRegistry`) contain mutable shared state (`Arc<Mutex<...>>`) | Infrastructure leak | HIGH |
| 4 | `InspectSnapshotFormatter` is a pure formatting utility混入domain types | Separation concern | MEDIUM |
| 5 | `RuntimeState` / `RuntimeEvent` state machine types embedded in types module | State machine mixed with data | HIGH |
| 6 | `ResumeError` error domain embedded in types file | Error taxonomy mixed | MEDIUM |
| 7 | Queue domain (`ShardCommandQueue`) embedded - should be in `queue.rs` | Infrastructure separation | MEDIUM |

### Primitive Obsession
- `u64` used for `generation`, `epoch` without newtype wrappers
- `u64` for correlation IDs without semantic typing

---

## 4. Recommended Split

```
src/shard/
├── types/
│   ├── mod.rs           (re-exports only)
│   ├── timer.rs         (PendingTimerKind, PendingTimer) ~50 lines
│   ├── command.rs       (ShardCommand enum) ~130 lines
│   ├── ask.rs           (AskTicket, AskAnswer) ~57 lines
│   ├── run_state.rs     (RunState) ~18 lines
│   ├── introspection.rs (InspectSnapshot, InspectResponse, InspectHandle, IntrospectionRegistry) ~212 lines
│   ├── queue.rs         (ShardCommandQueue, validators) ~112 lines
│   ├── shard.rs         (Shard, ShardStatus, ShardHealth, ShardConfig) ~78 lines
│   ├── runtime_state.rs (RuntimeState, RuntimeEvent) ~65 lines
│   └── resume.rs        (ResumeStatus, ResumeResult, ResumeError) ~74 lines
```

---

## 5. Priority Assessment

| Factor | Score | Notes |
|--------|-------|-------|
| Line limit violation | CRITICAL | 883 > 300 |
| DDD cohesion | HIGH | 7 domains in 1 file |
| Maintainability risk | HIGH | Cannot reason about single file |
| Downstream impact | MEDIUM | Breaks `mod.rs` re-exports |

**Overall Priority**: **P0 - CRITICAL**

---

## 6. Summary

```json
{
  "lines_count": 883,
  "limit": 300,
  "over_by_pct": 194,
  "violations": [
    "Line count exceeds 300-line limit (883 lines)",
    "7 distinct DDD bounded contexts in single file",
    "God Enum: ShardCommand with 15 variants",
    "Infrastructure leak: Arc<Mutex> in domain types",
    "State machine types embedded in types module",
    "Primitive obsession: u64 for generation/epoch"
  ],
  "ddd_smell": "God Module /职责过多",
  "priority": "P0"
}
```
