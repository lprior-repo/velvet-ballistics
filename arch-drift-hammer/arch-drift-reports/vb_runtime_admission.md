# Architectural Drift Report: `vb_runtime/src/admission.rs`

**File:** `crates/vb_runtime/src/admission.rs`
**Date:** 2026-05-29
**Analyzer:** architectural-drift skill

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **1970** | 300 | ❌ FAIL (557% of limit) |
| Production code | ~891 lines | 300 | ❌ FAIL (197% of limit) |
| Inline tests | ~1072 lines | 0 (should be in `tests/`) | ❌ FAIL |
| Test module includes | 7 lines (line 1969) | 0 | ❌ FAIL |

---

## 2. DDD Cohesion Analysis

**Filename:** `admission.rs`
**Domain Concept:** Runtime admission control for workflow runs

### Cohesion Verdict: **PARTIALLY COHESIVE** (smell detected)

The file attempts to capture a single bounded context ("admission"), but it violates single responsibility by mixing:

| Concern | Present | Should Be Separate |
|---------|---------|-------------------|
| Core domain types (`RunAdmission`, errors) | ✅ | Keep in `admission.rs` |
| Artifact envelope validation | ✅ | Move to `admission/artifact_envelope.rs` |
| `ArtifactStore` / `AcceptedArtifactStore` traits | ✅ | Move to `admission/store.rs` |
| Store implementations (`AlwaysPresentArtifactStore`, `MissingAcceptedArtifactStore`, `StorageArtifactStore`) | ✅ | Move to `admission/stores.rs` |
| Budget admission functions | ✅ | Move to `admission/budget.rs` |
| Inline tests | ❌ | Move to `admission/tests/` or `tests/admission/` |

---

## 3. Violations

### V-01: CRITICAL — File Size (1970 lines >> 300 lines)

```
Lines 1-890:    Production code (~45% of file)
Lines 891-1963: Inline tests (~54% of file)  
Lines 1964-1970: Module includes (~0.4% of file)
```

### V-02: CRITICAL — Inline Tests (1072 lines of `#[cfg(test)]`)

**Location:** Lines 891–1963

The module contains a massive `mod tests { ... }` block with:
- 40+ individual test functions
- Duplicated helper struct definitions (`NeverPresentStore`, `AlwaysPresentStore`, `FixedAcceptedStore`, etc.)
- Test-specific factory functions (`accepted_artifact_with_caps`, `test_digest`)

**Existing test infrastructure ignored:**
- `admission/artifact_envelope_tests.rs` (19.2K) — already exists but is `include!`d at EOF
- `admission/admission_test_support.rs` (856B) — already exists but not used in inline tests

### V-03: HIGH — Missing Module Separation (Concerns Not Segregated)

The file implements multiple distinct modules in one file:

| Module | Lines | Proposed Location |
|--------|-------|-------------------|
| `ArtifactEnvelopeError` + validation | 26–78, 499–546 | `admission/artifact_envelope.rs` |
| `ArtifactStore`, `AcceptedArtifactStore` traits | 316–374 | `admission/store.rs` |
| `AlwaysPresentArtifactStore`, `MissingAcceptedArtifactStore` | 331–434 | `admission/stores.rs` |
| `StorageArtifactStore` | 436–497 | `admission/stores.rs` |
| Budget types (`AdmissionBudgetRequest`) | 98–106 | `admission/budget.rs` |
| Budget admission functions | 731–793 | `admission/budget.rs` |
| `RunAdmission` + `AdmissionError` | 80–314 | KEEP: `admission.rs` |
| Core admission functions | 588–729 | KEEP: `admission.rs` |

### V-04: MEDIUM — Oversized Functions

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `admit_artifact_run_with_certificate_floor` | 71 | 30 | ❌ 137% over |
| `validate_accepted_artifact_envelope` | 37 | 30 | ❌ 23% over |
| `map_budget_error` | 62 | 30 | ❌ 107% over |
| `admit_run_with_budget_policy` | 38 | 30 | ❌ 27% over |

### V-05: LOW — Duplicated Test Infrastructure

Helper structs defined MULTIPLE times in inline tests:
- `NeverPresentStore` — defined 4 times (lines ~1274, ~1344, ~1375, ~1396, ~1868)
- `AlwaysPresentStore` — defined 2 times (lines ~1254, ~1717)
- `FixedAcceptedStore` — defined 1 time (line 896)
- `test_digest()` — defined 1 time but should be in test support
- `accepted_artifact_with_caps()` — defined 1 time but should be in test support

### V-06: LOW — Dead Code Path

Line 851: `resource: "unknown_aggregate_budget_error"` comment says "DEAD: #[non_exhaustive] catch-all" — this is a smell indicating the `#[non_exhaustive]` enum handling is inadequate.

---

## 4. DDD Smell Detection

| Smell | Present | Severity |
|-------|---------|----------|
| **Inline tests in production module** | ✅ YES | CRITICAL |
| **Feature Envy** (tests reaching into internals) | ✅ YES | MEDIUM |
| **Duplicate Code** (repeated test helpers) | ✅ YES | LOW |
| **Primitive Obsession** | ❌ NO | — |
| **Invalid Input Sedimentation** | ❌ NO | — |
| **Parasitic Equality** | ❌ NO | — |

---

## 5. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (Critical)** | Extract inline `mod tests` to `admission/tests/admission_tests.rs` | Medium |
| **P0 (Critical)** | Extract inline `mod artifact_envelope_tests` + `include!` to proper module | Medium |
| **P1 (High)** | Split production code into submodules: `artifact_envelope.rs`, `store.rs`, `stores.rs`, `budget.rs` | High |
| **P1 (High)** | Reduce `admit_artifact_run_with_certificate_floor` to ≤30 lines via extraction | Low |
| **P2 (Medium)** | Deduplicate test helper structs using existing `admission_test_support.rs` | Low |
| **P3 (Low)** | Address `#[non_exhaustive]` catch-all dead code path | Low |

---

## 6. Summary

```
Lines:           1970 (LIMIT: 300)         ❌ FAIL
DDD Cohesion:   Mixed concerns             ⚠️  SMELL
Inline Tests:   1072 lines (54% of file)   ❌ FAIL  
Module Sep:     Multiple modules in 1 file ❌ FAIL
Oversized Fns:  4 functions over limit     ⚠️  WARN
Duplicated Code: 6 repeated structs       ⚠️  WARN

OVERALL: ❌ ARCHITECTURAL DRIFT DETECTED
```

---

## 7. Recommended File Layout

```
vb_runtime/src/
├── admission/
│   ├── mod.rs           (re-exports)
│   ├── artifact_envelope.rs   (ArtifactEnvelopeError + validation)
│   ├── store.rs               (traits: ArtifactStore, AcceptedArtifactStore)
│   ├── stores.rs              (implementations)
│   ├── budget.rs              (budget types + functions)
│   └── tests/
│       ├── admission_tests.rs        (from inline mod tests)
│       └── artifact_envelope_tests.rs
└── admission.rs          (RunAdmission, AdmissionError, core admit_* fns)
```
