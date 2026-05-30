# Architectural Drift Report: vb_runtime_primitives_mod

## File Analyzed
**Path:** `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/primitives/mod.rs`

## Line Count
| Module | Lines |
|--------|-------|
| `mod.rs` (this file) | 17 |
| `collect.rs` | 876 |
| `for_each.rs` | 107 |
| `helpers.rs` | 526 |
| `reduce.rs` | 1,025 |
| `repeat.rs` | 968 |
| `retry.rs` | ~1,500+ |
| `together.rs` | 148 |
| `wait_ask.rs` | 726 |
| **Total** | **5,893+** |

**Status:** `mod.rs` itself passes (< 300 lines). Submodules violate file-size constraint.

## DDD Cohesion Analysis

### Bounded Context: Workflow Primitives
The `primitives` module contains execution primitives for the Velvet Ballistics workflow engine:
- **collect** — Pagination with page lineage tracking
- **for_each** — List iteration with fanout limits  
- **helpers** — Shared utilities (expect_list, jump_to, tail_items, etc.)
- **reduce** — Accumulator-based fold over lists
- **repeat** — Retry loop with packed I64 state encoding
- **retry** — RetryPolicy/RetryState state machine with bit-packed encoding
- **together** — Parallel branch fork/join with accumulator
- **wait_ask** — Suspension primitives (wait_until, wait_event, ask)

### Cohesion Verdict: **WEAK**
While all modules relate to "workflow execution primitives," they bundle unrelated concerns:
- `collect` + `reduce` + `for_each` = iteration primitives (cohesive)
- `repeat` + `retry` = retry policy primitives (cohesive)
- `wait_ask` = suspension primitives (cohesive)
- `together` = parallel branching (cohesive)
- `helpers` = cross-cutting utilities (legitimate shared module)

The current grouping forces all iteration primitives into one bucket, mixing:
1. Pagination with time limits (`collect`)
2. Simple iteration (`for_each`)
3. Accumulation (`reduce`)

## Violations

### 1. File Size Violations (PRIMARY)
- `collect.rs`: 876 lines (exceeds 300)
- `reduce.rs`: 1,025 lines (exceeds 300)
- `repeat.rs`: 968 lines (exceeds 300)
- `retry.rs`: ~1,500+ lines (exceeds 300)

### 2. Primitive Obsession (DDD)
| Location | Issue |
|----------|-------|
| `collect.rs:34-40` | `cursor: usize`, `page_size: usize`, `item_count: usize`, `limit: usize` — raw numeric types |
| `repeat.rs:13` | `REPEAT_SHIFT: u32 = 32` — magic constant for bit encoding |
| `retry.rs:219-230` | Manual bit-packing `encode()` uses raw `checked_shl(32)`, `checked_shl(16)` |

### 3. Bit-Packed State Encoding (SECONDARY)
- `repeat.rs:encode_repeat_state` — Packs max_attempts/current_attempt into I64
- `retry.rs:RetryState::encode` — Packs delay_ms/attempt/remaining into I64
- **Risk:** Fragile encoding violates "Parse, don't validate" — decoding can succeed on corrupted state

### 4. Missing NewType Wrappers
- No `PageSize`, `Cursor`, `ItemCount`, `Limit` types in `collect.rs`
- No `AttemptCounter`, `MaxAttempts` types in `repeat.rs`
- No `DelayMs`, `RetryCount` types in `retry.rs`

## DDD Smell

**Category:** Primitive Obsession + Data Clump

The `CollectPaginationState` struct (collect.rs:24-45) is a classic Data Clump:
```rust
pub struct CollectPaginationState {
    pub run_id: RunId,
    pub collector_slot: SlotIdx,
    pub source: ListId,
    pub current_page: ListId,
    pub cursor: usize,        // Should be Cursor newtype
    pub page_size: usize,      // Should be PageSize newtype
    pub item_count: usize,     // Should be ItemCount newtype
    pub limit: usize,          // Should be Limit newtype
    pub time_limit_ms: Option<u64>,
    pub start_millis: u64,
}
```

## Priority

| Issue | Severity | Effort |
|-------|----------|--------|
| File size violations (collect, reduce, repeat, retry) | **HIGH** | High (massive split needed) |
| Primitive obsession in collect.rs | **MEDIUM** | Medium (newtype wrappers) |
| Primitive obsession in repeat.rs | **MEDIUM** | Medium (newtype wrappers) |
| Primitive obsession in retry.rs | **MEDIUM** | Medium (newtype wrappers) |
| Bit-packed state encoding | **LOW** | Low (well-documented, tested) |

## Recommendations

### Immediate (within scope)
1. **Split `collect.rs`** into:
   - `collect/pagination_state.rs` — CollectPaginationState, CollectStates
   - `collect/handlers.rs` — collect_start, collect_page, collect_next, collect_finish
   - `collect/pagination.rs` — Page lineage, validation logic

2. **Split `reduce.rs`** into:
   - `reduce/state.rs` — ReduceStartPlan
   - `reduce/handlers.rs` — reduce_start, reduce_next, reduce_finish

3. **Split `repeat.rs`** into:
   - `repeat/state.rs` — AttemptCounter, encode/decode
   - `repeat/handlers.rs` — repeat_start, repeat_attempt, repeat_check, repeat_finish

4. **Split `retry.rs`** into:
   - `retry/policy.rs` — RetryPolicy, DelayStrategy, RetryPolicyError
   - `retry/state.rs` — RetryState, encode/decode
   - `retry/decision.rs` — RetryDecision, is_failure_retriable, evaluate_retry, compute_delay
   - `retry/handlers.rs` — retry_start, retry_on_failure, exhaustion_error

### Medium Term
5. Add newtype wrappers for numeric primitives (Cursor, PageSize, AttemptCounter, DelayMs)

---

**Report Generated:** 2026-05-29  
**Analyzer:** architectural-drift skill  
**Status:** VIOLATIONS FOUND
