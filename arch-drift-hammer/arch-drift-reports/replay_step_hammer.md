# Architectural Drift Report: `step.rs`

**File**: `crates/vb_core/src/replay/step.rs`
**Line Count**: 604 (VIOLATION: exceeds 300 max)
**Classification**: CRITICAL REFACTOR REQUIRED

---

## Executive Summary

The `step.rs` file is **604 lines** — a **201% violation** of the 300-line hard limit. It violates multiple Scott Wlaschin DDD principles: primitive obsession, workflow state modeling, and type-driven design. It MUST be split.

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 604 | 300 | **VIOLATION** |
| Excess lines | 304 | 0 | **+101%** |

**Required Splits**:
1. `step.rs` → `step.rs` (main entry) + `collect_pagination.rs` (Collect state machine) + `step_helpers.rs` (shared helpers)
2. Alternative: Split by node kind (Nop/SetConst/Copy in one, BuildObject/BuildList in another, Collect in third)

---

## 2. RESPONSIBILITY MAPPING

### Current Responsibilities (FRAGMENTED)

| Responsibility | Lines | Functions | DDD Violation |
|----------------|-------|-----------|---------------|
| Main replay dispatch | 118-192 | `replay_step`, `replay_step_with_collect` | Workflow not modeled |
| SuspensionKind/Action types | 15-57 | `SuspensionKind`, `ReplayAction` | Coupled to dispatch |
| Collect pagination state | 59-110 | `ReplayCollectState`, `ReplayCollectStates` | Primitive obsession |
| Collect replay | 194-391 | `replay_collect_*` (5 functions) | Should be dedicated type |
| Nop/Finish/Jump | 393-418 | `replay_nop`, `replay_finish`, `replay_jump` | Small, could stay |
| SetConst/Copy | 427-470 | `replay_set_const`, `replay_copy` | Small, could stay |
| EvalExpr | 472-488 | `replay_eval_expr` | Small, could stay |
| BuildObject | 490-538 | `replay_build_object` | Primitive obsession |
| BuildList | 540-585 | `replay_build_list` | Primitive obsession |
| Shared helpers | 300-600 | 10 helper functions | Scattered concerns |

### Correct DDD Boundaries

```
replay/
├── step.rs              (150 lines) - main entry, dispatch only
├── suspension.rs       (50 lines)  - SuspensionKind, ReplayAction
├── collect_pagination.rs (200 lines) - CollectState machine, page ops
├── step_builders.rs     (100 lines) - BuildObject, BuildList with typed slots
└── step_helpers.rs      (50 lines)  - advance_to_next, increment_replay_executed
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### Violation A: `replay_page_size` / `replay_item_limit` (Lines 330-345)

```rust
fn replay_page_size(raw: u32) -> Result<usize, ReplayError> {
    match raw {
        0 => Err(ReplayError::Internal { reason: "..." }),  // Stringly-typed error
        value => usize::try_from(value).map_err(|_| ...),
    }
}
```

**Problems**:
- `u32` for page_size/limit — should be `PageSize(u32)` and `ItemLimit(u32)` newtypes
- `usize` as cursor — should be `Cursor(usize)` 
- Stringly-typed error reasons — should be `CollectError::ZeroPageSize`, `CollectError::PageSizeOverflow`

**Fix**:
```rust
pub struct PageSize(u32);
pub struct ItemLimit(u32);
pub struct Cursor(usize);

impl PageSize {
    pub fn new(raw: u32) -> Result<Self, ReplayError> {
        if raw == 0 {
            Err(ReplayError::Collect(CollectError::ZeroPageSize))
        } else {
            usize::try_from(raw)
                .map_err(|_| ReplayError::Collect(CollectError::PageSizeOverflow))?;
            Ok(Self(raw))
        }
    }
}
```

### Violation B: `replay_build_object` / `replay_build_list` index loops (Lines 503-525, 553-570)

```rust
let mut index = 0usize;
while index < fields.len() {
    let (key, slot) = fields.get(index).ok_or(...)?;  // get+ok_or pattern
    // ...
    index = index.checked_add(1).ok_or(...)?;  // manual overflow check
}
```

**Problems**:
- Manual index iteration — should use `.iter().enumerate()` or `.chunks()`
- `index: usize` — should be `FieldIndex(usize)` or `ItemIndex(usize)`
- Duplicate pattern between BuildObject and BuildList — violates DRY

**Fix**: Extract to `fn write_fields<K, V>(run: &mut RunFrame, fields: &[(K, V)]) -> Result<Vec<...>, ReplayError>` with typed indices.

### Violation C: Inline error strings throughout

Lines 103, 207, 211, 216, 259, 270, 274, 292, 325, 333, 336, 343, 356, 377, 396, 407, 434, 439, 442, 458, 464, 481, 506, 512, 522, 528, 532, 548, 556, 562, 568, 576, 579, 588, 598.

**Every one of these stringly-typed errors is a primitive obsession violation.**

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### Violation 1: Missing Value Objects

**Current**: Raw `u32`, `usize`, `ListId` scattered as fields.

**Should Be**:
```rust
// Instead of ReplayCollectState with raw fields:
pub struct ReplayCollectState {
    source: ListId,
    current_page: ListId,
    cursor: usize,      // PRIMITIVE OBSESSION
    page_size: usize,   // PRIMITIVE OBSESSION
    item_count: usize,  // PRIMITIVE OBSESSION
    taint: Taint,
}

// Should be:
pub struct ReplayCollectState {
    source: ListId,
    current_page: PageListId,
    pagination: PaginationState,  // VALUE OBJECT
    taint: Taint,
}

pub struct PaginationState {
    cursor: Cursor,
    page_size: PageSize,
    item_count: ItemLimit,
}
```

### Violation 2: Collect Workflow Not Modeled as State Machine

The collect operation is a 4-phase workflow:
1. `CollectStart` → creates pagination state
2. `CollectPage` → jumps to body
3. `CollectNext` → paginates or finishes
4. `CollectFinish` → cleans up

**Current**: State transitions are implicit in function calls. No `enum CollectPhase { Start, Page, Next, Finish }`.

**Should Be**: Explicit `CollectMachine` type with `next(state) -> (action, state)` pattern.

### Violation 3: Duplicate Iteration Patterns

`replay_build_object` (lines 503-525) and `replay_build_list` (lines 553-570) are nearly identical:

```rust
// BOTH have this exact pattern:
let mut index = 0usize;
while index < items.len() {
    let slot = items.get(index).ok_or(ReplayError::Internal {
        reason: "...index checked by loop bound",
    })?;
    let value = *run.read_slot(*slot).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
        EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
        _ => ReplayError::Internal { reason: "..." },
    })?;
    let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
    accumulated_taint = join_taint(accumulated_taint, slot_taint);
    values.push(value);
    index = index.checked_add(1).ok_or(ReplayError::Internal {
        reason: "...item index overflow",
    })?;
}
```

**DDD Principle Violated**: "Duplication is far cheaper than the wrong abstraction."

### Violation 4: `advance_to_next` Duplication

Every non-suspend replay function (12+ functions) calls `advance_to_next` identically:

```rust
let next = advance_to_next(run, node)?;
Ok(ReplayAction::Continue(next))
```

**Should Be**: A combinator or macro, OR these functions should return `StepIdx` and the dispatch layer handles the wrapping.

---

## 5. PROPOSED REFACTOR

### File Split Plan

```
crates/vb_core/src/replay/
├── mod.rs           (update to include new modules)
├── step.rs          (~150 lines) - main entry, dispatch, suspension types
├── step_values.rs   (~100 lines) - PageSize, ItemLimit, Cursor, PaginationState
├── collect_paginate.rs (~180 lines) - Collect pagination machine
├── step_build.rs   (~100 lines) - BuildObject, BuildList with shared iteration
└── step_helpers.rs  (~50 lines) - advance_to_next, increment_replay_executed
```

### Value Objects to Create

| NewType | Underlying | Validation |
|---------|------------|------------|
| `PageSize(u32)` | `u32` | Non-zero, fits usize |
| `ItemLimit(u32)` | `u32` | Fits usize |
| `Cursor(usize)` | `usize` | None (controlled) |
| `FieldIndex(usize)` | `usize` | None (checked) |
| `ItemIndex(usize)` | `usize` | None (checked) |

### Error Types to Create

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectError {
    ZeroPageSize,
    PageSizeOverflow,
    LimitOverflow,
    PaginationStateMissing,
    SourceListMissing,
    SourceLengthChanged,
    CursorOverflow,
    CursorBeyondItemCount,
    InsertFailed,
}
```

---

## 6. RISK ASSESSMENT

| Risk | Level | Reason |
|------|-------|--------|
| Split complexity | HIGH | Many functions share state via `run`, `store`, `plan` |
| Test coupling | MEDIUM | Tests in `step_tests.rs` may break |
| API surface | LOW | Public functions are clearly delineated |

**Mitigation**: The public API (`replay_step`, `replay_step_with_collect`, `ReplayCollectStates`) stays the same. Only internal details change.

---

## 7. MANDATORY ACTIONS

1. **SPLIT FILE** into at least 3 parts:
   - `step.rs` (main + suspension types)
   - `collect_pagination.rs` (Collect state machine)
   - `step_values.rs` (PageSize, ItemLimit, Cursor, PaginationState newtypes)
   - `step_helpers.rs` OR inline helpers into callers

2. **CREATE VALUE OBJECTS** for all raw numeric types

3. **EXTRACT ERROR ENUM** for collect-specific errors

4. **ABSTRACT DUPLICATED LOOPS** in BuildObject/BuildList

5. **RETAIN API** - `replay_step` and `replay_step_with_collect` signatures MUST NOT change

---

## 8. VERIFICATION

After refactoring:
- [ ] All `.rs` files in `crates/vb_core/src/replay/` ≤ 300 lines
- [ ] No raw `u32` or `usize` used for domain concepts (page size, limit, cursor)
- [ ] No stringly-typed error reasons in `ReplayError::Internal`
- [ ] `cargo test -p vb_core` passes
- [ ] `moon ci` passes
