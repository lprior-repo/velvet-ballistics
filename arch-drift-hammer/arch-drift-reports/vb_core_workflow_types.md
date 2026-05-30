# Architectural Drift Report: `vb_core/src/workflow/types.rs`

## ⚠️ FILE NOT FOUND

**Requested file:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/workflow/types.rs`

**Actual file analyzed:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/workflow/mod.rs` (1909 lines)

---

## Analysis Summary

| Metric | Value | Status |
|--------|-------|--------|
| **Requested File** | `workflow/types.rs` | ❌ DOES NOT EXIST |
| **Actual File** | `workflow/mod.rs` | ✅ EXISTS |
| **Line Count** | 1909 | ❌ **SEVERE VIOLATION** (>300) |
| **DDD Cohesion** | Low | ❌ VIOLATION |
| **Priority** | CRITICAL | 🚨 IMMEDIATE ACTION REQUIRED |

---

## 1. Line Count Violation

**File:** `workflow/mod.rs`  
**Lines:** 1909  
**Limit:** <300  
**Violation:** Yes — **637% over limit**

---

## 2. DDD Cohesion Analysis

### Types Defined in Single File (15 total):

| Type | Lines | Cohesion Issue |
|------|-------|----------------|
| `CompiledWorkflow` | ~50 | Core domain entity |
| `ResourceContract` | ~65 | Separate domain concept |
| `ExprBranch` | ~10 | Expression sub-domain |
| `SlotBranch` | ~10 | Expression sub-domain |
| `WorkflowParts` | ~25 | Construction/builder pattern |
| `AccessorProgram` | ~10 | Accessor sub-domain |
| `PathSegment` | ~10 | Value path sub-domain |
| `WorkflowError` | ~116 | Error domain |
| `ExprProgram` | ~127 | Expression sub-domain |
| `ExprOp` | ~107 | Expression sub-domain |
| `CompiledNode` | ~22 | Node sub-domain |
| `CompiledNodeKind` | ~200+ | Node sub-domain |
| `LifecycleState` | ~27 | Lifecycle state machine |
| `LifecycleCommand` | ~10 | Lifecycle commands |
| `RunState` | ~20 | Run snapshot |

**Cohesion Smell:** Multiple distinct DDD bounded contexts (Workflow, Expression, Accessor, Lifecycle, Error) are jammed into a single 1909-line file.

---

## 3. Identified Violations

### 🚨 CRITICAL (Must Fix)

1. **File Size Exceeded** — 1909 lines vs 300 line max
   - Category: `<300-line rule`
   - Impact: Unmaintainable, unreviewable

2. **Low DDD Cohesion** — Multiple bounded contexts in one file
   - Category: `DDD cohesion`
   - Impact: Violates single responsibility principle
   - Expected: One file per bounded context or aggregate root

3. **Primitive Obsession Indicators**
   - `u16`, `u32` used directly for counts/indices without newtype wrappers
   - `Box<[T]>` slices without domain-specific collection types

4. **WorkflowError is 116 lines** — Should be extracted to `errors.rs` or own file

### ⚠️ WARNING

5. **ExprOp is 107 lines** — Contains 100+ variants, consider splitting by operation category (arithmetic, comparison, logical, etc.)

6. **CompiledNodeKind is 200+ lines** — Should be split into multiple files by node category

---

## 4. Recommended Refactoring

### Phase 1: Split by Sub-Domain

```
workflow/
├── mod.rs              # 1909 lines → ~100 lines (reexports only)
├── compiled_workflow.rs   # CompiledWorkflow + WorkflowParts
├── resource_contract.rs   # ResourceContract
├── expression/
│   ├── mod.rs         # ExprProgram, ExprBranch
│   ├── ops.rs         # ExprOp enum
│   └── accessor.rs    # AccessorProgram, PathSegment
├── nodes/
│   ├── mod.rs         # CompiledNode
│   └── kinds.rs       # CompiledNodeKind
├── lifecycle.rs       # LifecycleState, LifecycleCommand, RunState
└── error.rs           # WorkflowError
```

### Phase 2: Apply Scott Wlaschin DDD

- [ ] Replace raw `u16`/`u32` indices with newtype wrappers
- [ ] Extract value objects for `PathSegment`
- [ ] Model state transitions as explicit functions

---

## 5. Priority Assessment

| Priority | Level |
|----------|-------|
| **Severity** | CRITICAL |
| **Effort** | High (multi-file refactor) |
| **Risk** | Medium (breaking API change) |
| **Priority** | **P0 — Immediate** |

---

## 6. Files Needing Attention

```
workflow/mod.rs (1909 lines) — PRIMARY TARGET
```

**Note:** The file `types.rs` does not exist. The user may have meant `workflow/mod.rs` or may need to create `types.rs` as a re-export module after splitting.

---

*Report generated: 2026-05-29*  
*Tool: architectural-drift skill*  
*Rule set: <300 lines, DDD cohesion, Scott Wlaschin*
