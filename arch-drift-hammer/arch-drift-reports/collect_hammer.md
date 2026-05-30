# Architectural Drift Report: `collect.rs`

**File:** `crates/vb_runtime/src/primitives/collect.rs`
**Severity:** CRITICAL
**Line Count:** 876 (violates <300 line mandate by 192%)
**Drift Type:** Primitive Obsession + God Module

---

## Executive Summary

This file is a **god module** containing 5+ distinct responsibility domains totaling 876 lines. It violates the <300 line structural rule by **292%**. Furthermore, the pervasive use of raw primitive types (`u32`, `usize`, `u64`) where domain types should be used constitutes **severe primitive obsession** that makes illegal states representable.

---

## Responsibility Map

| Lines | Responsibility | Violation |
|-------|---------------|-----------|
| 23-45 | `CollectPaginationState` struct - state data | Primitive obsession: raw `usize`/`u64` fields |
| 48-58 | `CollectPageLineage` - ordering tracking | Acceptable value struct |
| 60-297 | `CollectStates` + impl - state CRUD + lineage | God module clustering |
| 160-223 | Hydration/deserialization logic | Should be separate module |
| 226-296 | Journal event processing | Should be separate module |
| 304-383 | Validation functions | Utility-level primitives |
| 388-415 | `collect_start` primitive op | Fine-grained |
| 419-429 | `collect_page` primitive op | Fine-grained |
| 527-553 | `collect_next` primitive op | Fine-grained |
| 603-617 | `collect_finish` primitive op | Fine-grained |
| 431-776 | All helper functions + plans | Scattered utilities |
| 778-872 | Kani verification harness | Should be behind feature gate in separate file |
| 874-876 | Test routing | Acceptable |

---

## Primitive Obsession Violations (Root Cause)

### VIOLATION 1: `CollectPaginationState` Field Types (Lines 23-45)

```rust
pub struct CollectPaginationState {
    pub cursor: usize,       // RAW USIZE - no domain wrapper
    pub page_size: usize,   // RAW USIZE - no domain wrapper
    pub item_count: usize,  // RAW USIZE - no domain wrapper
    pub limit: usize,       // RAW USIZE - no domain wrapper
    pub time_limit_ms: Option<u64>,  // RAW U64
    pub start_millis: u64,  // RAW U64
}
```

**Problem:** These fields accept ANY `usize`/`u64` without invariants. Callers can construct impossible states:
- `page_size = 0` (invalid by business rule)
- `cursor > item_count` (impossible cursor position)
- `item_count > limit` (data exceeds limit)
- `time_limit_ms = 0` (meaningless time limit)

**Fix:** Replace with refined types:
```rust
pub struct PageSize(usize);           // Always > 0
pub struct ItemCount(usize);          // Always >= 0
pub struct Cursor(usize);             // Always valid position
pub struct ItemLimit(usize);          // Always > 0
pub struct TimeLimitMs(u64);           // Always > 0 if Some
pub struct StartMillis(u64);           // Always valid epoch ms
```

### VIOLATION 2: `CollectStartPlan` Raw Primitives (Lines 431-440)

```rust
struct CollectStartPlan {
    page_size: usize,
    limit: usize,
    // ...
}
```

**Problem:** Raw `usize` fields accepted without invariants. Validation happens AFTER construction via separate functions. Between construction and validation, the plan is in an illegal state.

### VIOLATION 3: Function Parameters as Raw Primitives (Lines 388-398)

```rust
pub fn collect_start(
    // ...
    limit: u32,
    page_size: u32,
    // ...
)
```

**Problem:** `u32` parameters from callers (workflow bytecode) flow directly into domain logic without first being converted to validated domain types. The `page_size_from()` and `validate_item_limit()` functions do exist, but they're **ad-hoc guards**, not enforced by the type system.

### VIOLATION 4: Return Types as Raw Primitives

Functions like `checked_add_usize` (line 769) return raw `usize` with error-on-overflow semantics encoded as `EngineError`. This is ad-hoc arithmetic checking where overflow should either be impossible (proven) or handled by a `CheckedUsize` wrapper.

### VIOLATION 5: `CollectPageOrderViolationKind` as Raw Enum

While less severe, this enum is used as a raw discriminator for page ordering violations. The `classify_observed_page` function (lines 143-158) uses pattern matching on this enum without any encapsulation.

---

## Structural Violations

### GOD MODULE CLUSTERING

**Lines 1-876 in a single file** constitutes a god module. The 5 distinct responsibilities should be in separate files:

1. `primitives/collect/state.rs` - `CollectPaginationState`, `CollectStates`, `CollectPageLineage`
2. `primitives/collect/hydration.rs` - Serialization, journal recovery, validation
3. `primitives/collect/primitives.rs` - `collect_start`, `collect_page`, `collect_next`, `collect_finish`
4. `primitives/collect/plans.rs` - `CollectStartPlan`, `CollectNextPlan` builders
5. `primitives/collect/verification.rs` - Kani harnesses (behind `#[cfg(kani)]`)

---

## Recommended Refactoring

### Phase 1: Domain Type Wrappers

```rust
// In vb_core or vb_runtime::primitives::collect::types

use vb_core::errors::EngineError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize(usize);

impl PageSize {
    pub fn new(raw: u32) -> Result<Self, EngineError> {
        if raw == 0 {
            return Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect page_size must be nonzero",
            });
        }
        usize::try_from(raw).map(Self)?;
        Ok(Self(raw as usize))
    }
    pub fn get(self) -> usize { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCount(usize);

impl ItemCount {
    pub fn new(count: usize) -> Self { Self(count) }
    pub fn get(self) -> usize { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    value: usize,
    bound: usize,
}

impl Cursor {
    pub fn new(value: usize, bound: usize) -> Result<Self, EngineError> {
        if value > bound {
            return Err(EngineError::InternalInvariantViolation {
                reason: "cursor beyond bound",
            });
        }
        Ok(Self { value, bound })
    }
    pub fn get(self) -> usize { self.value }
    pub fn bound(self) -> usize { self.bound }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemLimit(usize);

impl ItemLimit {
    pub fn new(raw: u32) -> Result<Self, EngineError> {
        usize::try_from(raw).map_err(|_| EngineError::CollectItemLimitExceeded)?;
        Ok(Self(raw as usize))
    }
    pub fn get(self) -> usize { self.0 }
}
```

### Phase 2: Replace Primitives in Structs

Replace `CollectPaginationState` fields with domain types.

### Phase 3: Split File

Extract to 4 files per responsibility map above.

---

## Severity Assessment

| Violation | Severity | Impact |
|-----------|----------|--------|
| 876 lines in one file | **CRITICAL** | Impossible to review, test, or maintain |
| `CollectPaginationState` raw fields | **CRITICAL** | Illegal states representable |
| `CollectStartPlan` raw fields | **HIGH** | Plan can be built invalid |
| Function params as raw `u32`/`usize` | **HIGH** | Primitives flow into domain without validation |
| Kani harness inline | **MEDIUM** | Bloats module, should be behind feature |
| `checked_add_usize` scattered | **MEDIUM** | Ad-hoc overflow handling |

---

## Conclusion

This file is a **structural time bomb**. The 876-line length makes it unmaintainable, and the primitive obsession means the type system cannot prevent illegal pagination states. Any refactoring must:

1. **Immediately:** Extract domain type wrappers for `PageSize`, `Cursor`, `ItemCount`, `ItemLimit`, `TimeLimit`
2. **Immediately:** Replace raw primitive fields in `CollectPaginationState`
3. **Soon:** Split into 4-5 separate files per responsibility
4. **Eventually:** Move Kani harness behind feature gate

**This file requires a mandatory go-skill bead for full refactoring.**

---

*Report generated by: architectural-drift enforcer*
*Workspace: arch-drift-hammer*
