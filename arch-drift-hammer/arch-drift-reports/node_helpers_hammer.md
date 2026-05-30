# Architectural Drift Report: `node_helpers.rs`

**File**: `crates/vb_core/src/engine/node_helpers.rs`
**Total Lines**: 321
**Status**: 🚨 **VIOLATION - REFACTOR REQUIRED**

---

## Executive Summary

| Category | Status | Details |
|----------|--------|---------|
| **Line Count** | 🔴 FAIL | 321 lines (exceeds 300-line limit by 21 lines) |
| **DDD Cohesion** | 🟡 WARN | Production code is clean; test bloat is the problem |
| **Primitive Obsession** | 🟢 PASS | Properly uses `ConstIdx`, `SlotIdx`, `StepIdx` newtypes |

---

## Line Count Breakdown

| Section | Lines |占比 |
|---------|-------|-----|
| **Production Code** (1–73) | 63 | 19.6% |
| **Test Module** (75–321) | 247 | 76.9% |
| **Comments/Blank** | ~11 | 3.4% |
| **TOTAL** | **321** | 100% |

---

## Violation Details

### 1. Line Count Exceeded (CRITICAL)

**Problem**: File is 321 lines, exceeding the 300-line hard limit by **21 lines**.

**Root Cause**: The inline `#[cfg(test)]` module (lines 75–321) contains **247 lines** of test code. This is **76.9%** of the file.

---

## DDD Analysis (Scott Wlaschin)

### Production Code Assessment (Lines 1–73)

The 5 public helpers are **well-structured**:

| Function | Responsibility | DDD Score |
|----------|---------------|-----------|
| `set_const` | Write constant → slot, then advance | ✅ Clean |
| `copy_slot` | Copy value+taint between slots | ✅ Clean |
| `jump_to_next` | Option<StepIdx> → StepIdx validation | ✅ Clean |
| `jump_to` | Unconditional jump to StepIdx | ✅ Clean |
| `finish_run` | Emit final EngineSignal::Finished | ✅ Clean |

**Encapsulation**: `pub(super)` is correct — these are engine-internal helpers.

**Workflow Modeling**: These functions model **node execution transitions** — the workflow state machine. Each returns `EngineSignal` which is the state token.

### Primitive Obsession Check

| Primitive | Usage Location | Verdict |
|-----------|---------------|---------|
| `u16` | `test_frame(step_count: u16, slot_count: u16)` (line 93) | 🟡 Test-only, acceptable |
| Raw `i64` | None in production | 🟢 PASS |
| Raw `String` | None | 🟢 PASS |

**Verdict**: No primitive obsession violations in production code.

---

## Refactoring Prescription

### Required Action: Split Tests Out

**Problem**: Test code (247 lines) bloats a file meant for node helpers.

**Prescription**:

```
node_helpers.rs       (63 lines production + 10 line test module stub)
  ↓
node_helpers.rs       (73 lines: production code + #[cfg(test)] mod tests;)
node_helpers_test.rs  (247 lines: move full test module here)
```

Or better yet — since these are **internal engine helpers** with specialized test fixtures:

```
node_helpers.rs       (73 lines: production + inline test module stub)
  └── Move to: crates/vb_core/src/engine/
node_helpers_test.rs  (247 lines in tests/ subdirectory)
```

### Why Not Further Split Production?

The 63 lines of production code is **already minimal**:
- 5 focused functions
- 1 `use EngineSignal` import (needed at line 73 because of import order)
- Proper error propagation throughout

Splitting production further would damage **contextual cohesion** — these helpers are tightly related node execution primitives.

---

## Finding Summary

| # | Severity | Type | Location | Description |
|---|----------|------|----------|-------------|
| 1 | 🔴 CRITICAL | Line Count | Entire file (321 lines) | Exceeds 300-line limit |
| 2 | 🟡 WARNING | Test Bloat | Lines 75–321 | 247-line test module should be external |

---

## Recommended Fix

**Move test module to separate file** `node_helpers_test.rs` in the same directory:

1. Create `crates/vb_core/src/engine/node_helpers_test.rs`
2. Move lines 75–321 (the `#[cfg(test)] mod tests { ... }`) to the new file
3. Keep `node_helpers.rs` at ~73 lines (63 production + 10 stub)
4. Update `mod.rs` in `engine/` to include `mod node_helpers_test;` if tests need to be in-tree

**Result**:
- `node_helpers.rs`: 73 lines ✅ (under 300)
- `node_helpers_test.rs`: 247 lines (test files exempt from line limits)
- Total unchanged, but distribution now legal

---

## Conclusion

**STATUS**: 🚨 **VIOLATION DETECTED**

The file violates the 300-line architectural constraint. However, the production code is **DDD-clean** — the problem is purely structural (test bloat). Splitting the test module resolves the violation without any functional change.

**Action Required**: Move `#[cfg(test)] mod tests { ... }` to `node_helpers_test.rs`.
