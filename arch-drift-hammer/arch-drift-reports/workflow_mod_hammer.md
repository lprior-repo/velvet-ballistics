# ARCHITECTURAL DRIFT REPORT: workflow/mod.rs

**File:** `crates/vb_core/src/workflow/mod.rs`
**Total Lines:** 1909
**Violation:** CATASTROPHIC — 1909 lines against 300-line hard limit

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 1909 | 300 | **FAIL** |
| Overage | 1609 | 0 | **536% over** |

---

## 2. BOUNDARY DRIFT: DEFINITION vs EXECUTION

### Current State (MIXED)

The module mingles three distinct architectural concerns:

| Lines | Concern | Violation Type |
|-------|---------|----------------|
| 1–732 | Data structures (CompiledWorkflow, Node, ExprProgram, ResourceContract) | **Definition boundary** |
| 734–1827 | Validation logic (validate_*, check_*, apply_*) | **Execution boundary** |
| 1829–1909 | Lifecycle state machine | **Execution boundary** |

### Root Cause

The module violates the Single Responsibility Principle by treating `workflow/mod.rs` as a "workflow kitchen sink" instead of a properly bounded domain module.

### Required Split

```
workflow/
├── definition.rs    # ~200 lines: CompiledWorkflow, WorkflowParts, CompiledNode,
│                    #              CompiledNodeKind (enum only), ResourceContract,
│                    #              ExprProgram, AccessorProgram, PathSegment
├── validation.rs    # ~250 lines: All validate_* functions, check_* functions,
│                    #              apply_* functions, graph validation algorithms
├── lifecycle.rs    # ~80 lines:  LifecycleState, LifecycleCommand, RunState,
│                    #              check_lifecycle_transition
└── mod.rs          # ~50 lines:  Re-exports only
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### A. Untyped Numeric Indices (Lines 571–702)

`CompiledNodeKind` uses raw primitives where typed indices exist:

```rust
// VIOLATION: Raw u32 for limit
ForEachStart { limit: u32, ... }       // Line 607

// VIOLATION: Raw u16 for branch index
TogetherBranch { branch: u16, ... }    // Line 626

// VIOLATION: Raw u16 for max_attempts
RepeatStart { max_attempts: u16, ... } // Line 679
```

**Should be:**
```rust
struct StepLimit(u32);
struct BranchIndex(u16);
struct RetryLimit(u16);
```

### B. Untyped Collection Size (Lines 621, 634)

```rust
// VIOLATION: Raw u16 for count
TogetherStart { branches: Box<[StepIdx]>, join: StepIdx }
TogetherJoin { branch_count: u16, accumulator: SlotIdx }
```

**Should be:** `BranchCount(u16)`

### C. Untyped Page/Collection Parameters (Lines 641-644)

```rust
// VIOLATION: Raw u32 for page_size
CollectStart { page_size: u32, limit: u32, ... }
```

**Should be:** `PageSize(u32)`, `CollectionLimit(u32)`

### D. Raw u64 for Budget/Timing (Lines 205, 206, 213, 225)

```rust
ResourceContract {
    max_step_budget_per_tick: u64,      // Line 205
    max_transitions_per_tick: u64,     // Line 206
    max_blob_bytes: u64,               // Line 213
    ...
}
```

**Should be:** `StepBudget(u64)`, `TransitionBudget(u64)`, `ByteCount(u64)`

### E. Unguarded u32 Max Sentinel (Line 1325)

```rust
PathSegment::Index(index) => {
    if index == u32::MAX {  // VIOLATION: Magic number sentinel
        return Err(...)
    }
}
```

**Should be:** `const RESERVED_INDEX: u32 = u32::MAX;` in a const block or a newtype wrapper.

---

## 4. GOD ENUM: CompiledNodeKind

`CompiledNodeKind` (lines 562–732) spans **170 lines** with **40+ variants**. This is a classic God enum violating the Open/Closed principle — every new workflow primitive requires modifying this enum.

### Refactoring Target

```rust
// Split by category:
mod node_kinds {
    pub mod primitive { /* SetConst, Copy, EvalExpr, Nop, Jump, Finish */ }
    pub mod composite { /* BuildObject, BuildList */ }
    pub mod control { /* Choose, ChooseSlot, ForEach*, Together*, Collect*, Reduce*, Repeat* */ }
    pub mod async_ { /* WaitUntil, WaitEvent, Ask, AskResume */ }
    pub mod error { /* ErrorHandler, RetryCheck */ }
}
```

---

## 5. VALIDATION FUNCTION TURF

The module contains **50+ validation functions** (lines 734–1827). This is validation spaghetti — each function validates one aspect but they call each other in complex ways making the flow unreadable.

### Worst Offenders

| Function | Lines | Issue |
|----------|-------|-------|
| `validate_parts` | 734–752 | 19-line function calling 8 sub-validators |
| `validate_node_kind` | 936–1065 | 130-line match with no sub-categorization |
| `validate_forward_edges` | 1554–1577 | Graph algorithm embedded in module |
| `validate_reachability` | 1378–1447 | BFS algorithm embedded in module |

---

## 6. SCOTT WLASCHIN DDD ASSESSMENT

| Principle | Status | Evidence |
|-----------|--------|----------|
| **Types over primitives** | ❌ FAIL | Raw u16/u32/u64 throughout |
| **Make illegal states unrepresentable** | ⚠️ PARTIAL | Index types exist but not used in NodeKind |
| **Value objects for meaningful quantities** | ❌ FAIL | No StepLimit, BranchCount, ByteCount types |
| **Finite state machines for transitions** | ⚠️ PARTIAL | LifecycleState exists but embedded in mod.rs |
| **No God enums** | ❌ FAIL | CompiledNodeKind has 40+ variants |
| **Single responsibility** | ❌ FAIL | Definition + validation + lifecycle in one file |

---

## 7. PRESCRIBED REMEDIATION

### Phase 1: File Split (Before any logic changes)
```bash
# Create workflow/ directory structure
mkdir -p crates/vb_core/src/workflow/{definition,validation,lifecycle}

# Move in order of dependency (definition has no deps)
mv definition.rs  crates/vb_core/src/workflow/definition.rs
mv validation.rs crates/vb_core/src/workflow/validation.rs
mv lifecycle.rs crates/vb_core/src/workflow/lifecycle.rs
# mod.rs becomes re-exports only
```

### Phase 2: Value Object Introduction
Create newtypes for all raw numeric types in `CompiledNodeKind`:
- `StepLimit(u32)`
- `BranchIndex(u16)`, `BranchCount(u16)`
- `RetryLimit(u16)`
- `PageSize(u32)`, `CollectionLimit(u32)`
- `StepBudget(u64)`, `TransitionBudget(u64)`, `ByteCount(u64)`

### Phase 3: God Enum Decomposition
Break `CompiledNodeKind` into categorized enums with a trait-based visitor pattern.

### Phase 4: Validation Graph Extraction
Move all `validate_*` and `check_*` functions into a `ValidationContext` struct that carries the parts being validated, making the call chain explicit.

---

## 8. RISK ASSESSMENT

| Risk | Severity | Likelihood |
|------|----------|------------|
| Merge conflicts from concurrent edits | CRITICAL | HIGH |
| Validation logic bugs due to unreadability | HIGH | HIGH |
| Performance regressions undetected | HIGH | MEDIUM |
| New workflow primitives cause 1909-line file to grow | CRITICAL | HIGH |

---

## 9. VERDICT

**ARCHITECTURAL DRIFT: UNACCEPTABLE**

This file is a **6.4x overage** of the 300-line hard limit and violates every Scott Wlaschin DDD principle. It will continue to grow until it becomes unmaintainable.

**IMMEDIATE ACTION REQUIRED:** Split before any new beads can be worked on this module.

---

*Report generated by architectural-drift agent*
*Workspace: arch-drift-hammer*
*Target: vb_core/src/workflow/mod.rs*
