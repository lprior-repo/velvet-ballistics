# Architectural Drift Report: `vb_core/src/errors.rs`

**File**: `crates/vb_core/src/errors.rs`  
**Analysis Date**: 2026-05-29  
**Status**: CRITICAL DRIFT DETECTED

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **2055** | 300 | ❌ **685% of limit** |

---

## 2. DDD Cohesion Analysis

**Filename concept**: `errors` (errors/error handling)  
**Verdict**: ❌ **NO** — The filename reflects a single domain concept (errors), but the file violates single-responsibility by bundling:

### Mixed Concepts in One File

| Concept | Lines | Domain |
|---------|-------|--------|
| `CoreError` enum (50+ variants) | 163–498 | Execution errors |
| `CollectPageOrderViolationKind` enum | 21–30 | Collection lifecycle |
| `CollectExtraHydrationFailureKind` enum | 33–61 | Collection lifecycle |
| `CollectEvidenceCapacityExceeded` struct | 63–76 | Collection lifecycle |
| `LifecycleStorageUnavailable` struct | 78–89 | Lifecycle |
| `LifecycleDuplicateRequest` struct | 92–104 | Lifecycle |
| `LifecycleStaleRequest` struct | 106–119 | Lifecycle |
| `LifecycleInvalidTransition` struct | 121–134 | Lifecycle |
| `JournalWriteFailure` struct | 136–147 | Lifecycle |
| `ReplayCorruption` struct | 149–160 | Lifecycle |
| `impl CoreError` methods | 500–734 | Error impl |
| `HasSymbolicCode` impl | 719–734 | Trait impl |
| **`#[cfg(test)]` module** | 736–2055 | Tests only |

**DDD Smell**: ✅ **YES** — Feature envy / mixed responsibilities

---

## 3. Violations

### V1: File Size (CRITICAL)
- **Location**: Entire file
- **Severity**: CRITICAL
- **Description**: File is 2055 lines, 685% of the 300-line maximum
- **Impact**: Unmaintainable, violates architectural contract
- **Remediation**: Split into `errors/*.rs` module

### V2: Inline Tests (HIGH)
- **Location**: Lines 736–2055 (1319 lines of tests in-file!)
- **Severity**: HIGH
- **Description**: 60+ test functions inline in production source file
- **Impact**: Violates test separation; should be in `tests/` or behind feature gate
- **Remediation**: Move to `vb_core/tests/` or `tests/errors_tests.rs`

### V3: Mixed Domain Concepts (HIGH)
- **Location**: Lines 21–160 (error structs) + 163–498 (`CoreError`)
- **Severity**: HIGH
- **Description**: Collection lifecycle errors (`CollectPageOrderViolationKind`, etc.) mixed with execution errors (`CoreError`)
- **Impact**: Violates DDD bounded context separation
- **Remediation**: Extract collection errors to `errors/collection.rs`, lifecycle errors to `errors/lifecycle.rs`

### V4: Standalone Error Structs Should Be Newtypes (MEDIUM)
- **Location**: Lines 63–160
- **Description**: `CollectEvidenceCapacityExceeded`, `Lifecycle*`, `JournalWriteFailure`, `ReplayCorruption` are standalone structs that duplicate the pattern of `CoreError` variants with same fields
- **Impact**: Code duplication; these fields are also in `CoreError` variants
- **Remediation**: Remove standalone structs if redundant, or refactor to proper newtypes

### V5: Constants Bloating (MEDIUM)
- **Location**: Lines 500–627 (98 diagnostic code constants)
- **Severity**: MEDIUM
- **Description**: 98 `pub const` diagnostic codes defined inline in `impl CoreError`
- **Impact**: Constants should be in a separate diagnostic registry module
- **Remediation**: Extract to `diagnostic/codes.rs`

---

## 4. Specific Line Counts

| Section | Lines | Content |
|---------|-------|---------|
| Module doc + imports | 1–12 | Header |
| `CollectPageOrderViolationKind` | 21–30 | 10 lines |
| `CollectExtraHydrationFailureKind` | 33–61 | 29 lines |
| Standalone error structs | 63–160 | 98 lines |
| `CoreError` enum | 163–498 | 336 lines |
| `impl CoreError` constants | 500–627 | 128 lines |
| `diagnostic_code()` impl | 629–684 | 56 lines |
| `runtime_code()` impl | 686–716 | 31 lines |
| `HasSymbolicCode` impl | 719–734 | 16 lines |
| **Inline tests** | 736–2055 | **1319 lines** |

---

## 5. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (Critical)** | Split file into `errors/` module directory | High |
| **P0 (Critical)** | Move inline tests to `tests/errors_tests.rs` | Medium |
| **P1 (High)** | Extract collection errors → `errors/collection.rs` | Medium |
| **P1 (High)** | Extract lifecycle errors → `errors/lifecycle.rs` | Medium |
| **P2 (Medium)** | Extract diagnostic constants to `diagnostic/codes.rs` | Low |
| **P3 (Low)** | Audit standalone error structs for redundancy | Low |

---

## 6. Proposed Module Structure

```
crates/vb_core/src/
├── errors/
│   ├── mod.rs          (reexports, CoreError, type aliases)
│   ├── collection.rs   (CollectPageOrderViolationKind, CollectExtraHydrationFailureKind, collection variants)
│   ├── lifecycle.rs     (Lifecycle* structs, lifecycle variants)
│   └── codes.rs        (Diagnostic code constants)
└── tests/
    └── errors_tests.rs  (moved inline tests)
```

---

## Summary

- **Lines**: 2055 (must be ≤300) ❌
- **DDD Smell**: YES ❌  
- **Violations**: 5 total (1 critical, 2 high, 2 medium)
- **Remediation Priority**: P0 — Immediate refactor required
