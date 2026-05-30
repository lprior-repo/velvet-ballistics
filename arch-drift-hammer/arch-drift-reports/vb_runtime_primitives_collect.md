# Architectural Drift Report: `vb_runtime_primitives_collect`

## File Identification
- **Path**: `crates/vb_runtime/src/primitives/collect.rs`
- **Total Lines**: 876 (VIOLATION: exceeds 300-line limit)
- **Priority**: CRITICAL

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 876 | 300 | ❌ OVER by 192% |
| Production code | ~768 | 300 | ❌ OVER |
| Kani harness | 91 | N/A | Should be isolated |
| Test reference | 1 | N/A | Should be isolated |

---

## 2. DDD Cohesion Analysis

### Domain Bounded Context
- **Aggregate**: `CollectPaginationState` - pagination state for workflow collect primitive
- **Value Objects**: `CollectPageLineage` - tracks page history for order violations
- **Domain Service**: `CollectStates` - side table managing pagination state per (RunId, SlotIdx)
- **Operations**: `collect_start`, `collect_page`, `collect_next`, `collect_finish`

### What the File Does (Single Responsibility Violation)
1. **State Management** (lines 23-58): Domain types definition
2. **State Operations** (lines 60-297): `CollectStates` impl block with 15 methods
3. **Journal Hydration** (lines 304-310, 226-296): Persistence recovery logic
4. **Validation** (lines 312-362, 577-587, 751-767): State and input validation
5. **Operation Handlers** (lines 385-617): `collect_start`, `collect_page`, `collect_next`, `collect_finish`
6. **Helper Functions** (lines 629-776): Time, arithmetic, copy utilities
7. **Kani Harness** (lines 782-872): Verification proof
8. **Test Reference** (lines 874-876): Module path

### DDD Smells Identified

| Smell | Severity | Description |
|-------|----------|-------------|
| **God Object** | CRITICAL | File bundles 8 distinct responsibilities that should be separate modules |
| **Feature Envy** | HIGH | `CollectStates` methods couple tightly to `CollectPaginationState` internals |
| **Mixed Abstraction Levels** | HIGH | Domain logic interleaved with I/O (hydration, journal events) |
| **Verification Proximity** | MEDIUM | Kani harness embedded inline instead of `verification/` or feature-gated module |
| **Test Module Path** | LOW | Test reference at bottom creates non-obvious coupling |

---

## 3. Structural Violations

### 3.1 File Size Violation
```
876 lines found, 300 lines required
Excess: 576 lines (192% over limit)
```

### 3.2 Kani Harness Not Isolated
The `#[cfg(kani)] mod kani_collect_verification` (lines 782-872) is embedded in the production module rather than:
- Being behind a feature flag (e.g., `vb_runtime/kani-collect-pagination`)
- Living in `verification/kani/` directory
- Compiled only when `kani` feature is enabled

### 3.3 Test Path Coupling
```rust
#[cfg(test)]
#[path = "../collect_tests.rs"]
mod tests;
```
This creates implicit coupling to a sibling module at `../collect_tests.rs`.

---

## 4. Recommended Refactoring (Minimum Viable)

Convert `collect.rs` into a directory module:

```
primitives/collect/
├── mod.rs          (~100 lines) - Re-exports, public API
├── state.rs        (~80 lines)  - CollectPaginationState, CollectPageLineage
├── table.rs        (~150 lines) - CollectStates, lineage management
├── hydration.rs    (~100 lines) - Journal/durable frame hydration
├── handlers.rs     (~200 lines) - collect_start, collect_page, collect_next, collect_finish
├── validation.rs   (~80 lines)  - All validate_* functions
├── helpers.rs      (~50 lines)  - Utility functions (checked_add, millis_since_epoch, etc.)
└── kani.rs         (~100 lines) - Kani verification harness (feature-gated)
```

---

## 5. Evidence

```rust
// Line 23-58: Domain types
pub struct CollectPaginationState { ... }
pub struct CollectStates { ... }
struct CollectPageLineage { ... }

// Line 60-297: CollectStates impl - 15 methods managing state

// Line 782-872: Kani harness embedded inline
#[cfg(kani)]
mod kani_collect_verification { ... }

// Line 874-876: Test reference
#[cfg(test)]
#[path = "../collect_tests.rs"]
mod tests;
```

---

## 6. Priority Assessment

| Factor | Score | Notes |
|--------|-------|-------|
| Line limit violation | 10 | 876 vs 300 (critical) |
| DDD cohesion | 8 | Feature envy, mixed concerns |
| Verification isolation | 7 | Kani harness should be separate |
| Refactoring complexity | 6 | Multiple well-defined extraction targets |
| Risk of change | 4 | Stable domain, low churn expected |

**Overall Priority**: CRITICAL

---

## 7. Compliance Status

- [ ] Under 300 lines
- [ ] Single DDD bounded context
- [ ] Verification artifacts isolated
- [ ] No feature envy patterns
- [ ] Test references in dedicated location
