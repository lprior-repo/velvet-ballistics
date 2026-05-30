# Architectural Drift Report: `vb_core/src/errors.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/errors.rs`
**Total Lines**: 2055
**Violation**: CATASTROPHIC — exceeds 300-line limit by **6.8x**

---

## Executive Summary

This file is a **DDD bounded-context collision**. It crams 6+ distinct error domains into a single 2055-line file. The `CoreError` enum is not a "single error type" — it is a **god enum** that violates:

1. **Single Responsibility Principle** — one file handling execution, lifecycle, resource, capability, and collect errors
2. **Bounded Context Separation** — error types from different subdomains are mixed without boundary markers
3. **<300 Line Rule** — 2055 lines is 6.8x over the hard limit
4. **Test Isolation** — 1319 lines of tests (64%) pollute the production code file

---

## Structural Analysis

### File Layout

| Lines | Content | Lines |
|-------|---------|-------|
| 1–13 | Module doc, imports | 13 |
| 14–161 | Standalone error structs (unused duplicates) | 148 |
| 163–498 | `CoreError` enum definition | 336 |
| 500–734 | `CoreError` impl block (`diagnostic_code`, `runtime_code`, `symbolic_code`) | 235 |
| 736–2055 | Tests | **1319** |

### Duplicate Definitions (CRITICAL WASTE)

These 7 structs are defined **both** as standalone types AND embedded inside `CoreError` variants with **identical fields**:

| Standalone Struct | Embedded Variant | Lines Wasted |
|-------------------|------------------|--------------|
| `CollectEvidenceCapacityExceeded` (L64) | `CoreError::CollectEvidenceCapacityExceeded` (L408) | ~13 |
| `LifecycleStorageUnavailable` (L79) | `CoreError::LifecycleStorageUnavailable` (L422) | ~12 |
| `LifecycleDuplicateRequest` (L92) | `CoreError::LifecycleDuplicateRequest` (L434) | ~12 |
| `LifecycleStaleRequest` (L107) | `CoreError::LifecycleStaleRequest` (L448) | ~12 |
| `LifecycleInvalidTransition` (L122) | `CoreError::LifecycleInvalidTransition` (L462) | ~12 |
| `JournalWriteFailure` (L137) | `CoreError::JournalWriteFailure` (L476) | ~12 |
| `ReplayCorruption` (L150) | `CoreError::ReplayCorruption` (L488) | ~12 |

**Total wasted lines on duplicate definitions**: ~85 lines of pure redundancy.

---

## Bounded Context Map

`CoreError` contains errors from **6 distinct bounded contexts**:

### Context 1: Execution (29 variants)
Program counter, step transitions, slots, expressions, constants, stack, types.
```
InvalidProgramCounter, MissingNextStep, SlotOutOfBounds, SlotUninitialized,
ExprOutOfBounds, ConstOutOfBounds, MissingOutputSlot, StepStateOutOfBounds,
TypeMismatch, NonBoolCondition, DivisionByZero, NonFiniteNumber,
StepBudgetExhausted, StepCounterOverflow, ExpressionStackOverflow,
ExpressionStackUnderflow, UnsupportedPrimitive, UnsupportedAccessorTraversal,
ObjectFieldNotFound, ListIndexOutOfBounds, InternalInvariantViolation,
RepeatExhausted, CollectPageLimitExceeded, CollectItemLimitExceeded,
CollectTimeLimitExceeded, TogetherBranchLimitExceeded, ParallelLimitExceeded,
IterationLimitExceeded, InvalidCompiledWorkflow
```

### Context 2: Resource/Budget (5 variants)
```
QueueFull, ResourceLimitExceeded, AllocationFailed, BudgetExceeded, BudgetParse
```

### Context 3: Capability/Security (1 variant)
```
CapabilityDenied
```

### Context 4: Collect/Evidence (4 variants)
```
CollectPageOrderViolation, CollectExtraHydrationFailed,
CollectEvidenceCapacityExceeded, CollectPageLimitExceeded,
CollectItemLimitExceeded, CollectTimeLimitExceeded
```

### Context 5: Lifecycle/Persistence (6 variants)
```
LifecycleStorageUnavailable, LifecycleDuplicateRequest, LifecycleStaleRequest,
LifecycleInvalidTransition, JournalWriteFailure, ReplayCorruption
```

### Context 6: Handle/ID Resolution (4 variants)
```
SymbolOutOfBounds, ListOutOfBounds, ObjectOutOfBounds, BlobOutOfBounds
```

---

## Violations

### 1. Line Count (CRITICAL)
- **Actual**: 2055 lines
- **Limit**: 300 lines
- **Ratio**: 6.8x over limit

### 2. Primitive Obsession (MINOR)
`CoreError::ResourceLimitExceeded { resource: &'static str }` uses `&'static str` instead of a newtype. Same with `BudgetExceeded { budget: &'static str }`. These string fields should be newtype wrappers.

### 3. Error Handling Logic vs Error Definitions (MODERATE)
The `diagnostic_code()` and `runtime_code()` methods (lines 631–716) are **not** thin enum accessors — they are effectively a second data model that mirrors the enum variants. If a new variant is added but the match arms are not updated, this code silently breaks. This is a maintenance trap.

### 4. Standalone Struct Duplication (MODERATE)
Lines 64–160 define 7 structs that are **never used as standalone types**. They only exist as embedded variants in `CoreError`. They should be removed and the variants should use inline anonymous structs or the variants should be split into their own modules.

### 5. Test File Pollution (CRITICAL)
1319 of 2055 lines (64%) are tests. Tests should **never** be in the same file as production code definitions. They belong in `tests/` subdirectory or a separate `errors_tests.rs` file behind a `#[cfg(test)]` module in `mod.rs`.

---

## Refactoring Prescription

### Step 1: Remove Duplicate Structs (Save ~85 lines)
Delete lines 64–160. The standalone structs are unused dead code.

### Step 2: Extract Tests (Save ~1319 lines)
Move all tests (lines 736–2055) to `crates/vb_core/tests/test_errors.rs` or similar.

### Step 3: Split by Bounded Context (Save ~700+ lines)

After removal of duplicates and tests, split into:

```
crates/vb_core/src/errors/
├── mod.rs              # Re-exports + CoreError alias
├── execution.rs        # ExecutionContext errors (~200-250 lines)
├── resource.rs         # Resource/Budget errors (~80 lines)
├── capability.rs       # CapabilityDenied (~30 lines)
├── collect.rs          # Collect/Evidence errors (~80 lines)
├── lifecycle.rs        # Lifecycle/Persistence errors (~120 lines)
└── handles.rs          # Handle resolution errors (~60 lines)
```

### Step 4: Fix Primitive Obsession
```rust
// Instead of:
ResourceLimitExceeded { resource: &'static str }

// Use:
pub struct ResourceName(pub &'static str);
ResourceLimitExceeded { resource: ResourceName }
```

### Step 5: Consider `thiserror` v2 `#[source]` for Chain
Currently errors don't chain. Consider adding `#[source]` for errors that wrap underlying I/O or parse failures.

---

## Verdict

**SEVERITY: CATASTROPHIC**

This file violates the architectural contract on every axis:
- 2055 lines vs 300 line limit (6.8x over)
- 6 bounded contexts in one god enum
- 1319 lines of tests in production code file
- 7 duplicate struct definitions
- Primitive obsession in error field types

**IMMEDIATE ACTION REQUIRED**. This cannot be allowed to persist.

---

## Recommended Fix Commands

```bash
# 1. Create errors/ directory
mkdir -p crates/vb_core/src/errors

# 2. Move tests to integration test file
# 3. Delete duplicate standalone structs
# 4. Split CoreError into context-specific enums
# 5. Update mod.rs to re-export
# 6. Update all call sites to use new module paths
```
