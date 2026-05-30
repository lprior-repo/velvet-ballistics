# Architectural Drift Report: vb_runtime::admission

**File:** `crates/vb_runtime/src/admission.rs`  
**Analyzed:** 2026-05-29  
**Priority:** CRITICAL

---

## Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | **1970** | < 300 | ❌ FAIL (656% over) |
| DDD cohesion | **LOW** | High | ❌ FAIL |
| File size violations | 1 | 0 | ❌ FAIL |
| DDD smell score | **SEVERE** | None | ❌ FAIL |

---

## 1. Line Count Analysis

```
Actual:   1970 lines
Limit:     300 lines
Over by:  1670 lines (656% of threshold)
```

**Verdict:** ❌ **CRITICAL VIOLATION** — File is 6.5x the size limit.

---

## 2. DDD Cohesion Analysis

### Domain Concepts Identified (13 distinct concepts)

| Concept | Type | DDD Role | File Location |
|---------|------|----------|---------------|
| `RunAdmission` | Entity | Admission Record | Lines 80-95 |
| `AdmissionBudgetRequest` | Value Object | Budget Request | Lines 97-106 |
| `ArtifactEnvelopeError` | Error/Enum | Artifact Validation | Lines 22-78 |
| `AdmissionError` | Error/Enum | Admission Rejection | Lines 199-314 |
| `ArtifactStore` | Trait | Storage Abstraction | Lines 316-323 |
| `AcceptedArtifactStore` | Trait | Storage Abstraction | Lines 361-374 |
| `AlwaysPresentArtifactStore` | Impl | Test Stub | Lines 331-383 |
| `MissingAcceptedArtifactStore` | Impl | Test Stub | Lines 336-434 |
| `StorageArtifactStore` | Impl | Production Storage | Lines 436-497 |
| `admit_run` | Function | Admission Service | Lines 594-622 |
| `admit_artifact_run` | Function | Admission Service | Lines 638-653 |
| `admit_artifact_run_with_certificate_floor` | Function | Admission Service | Lines 655-729 |
| `admit_run_with_budget` | Function | Budget Admission | Lines 732-753 |
| `admit_run_with_budget_policy` | Function | Budget Admission | Lines 755-793 |

### Cohesion Assessment: **LOW**

**Reason:** Single file contains:
- 2 error enums (should be separate files)
- 2 traits (should be separate files)
- 3 store implementations (should be separate files)
- 5 admission functions (should be in a service module)
- 1 entity + 1 value object
- 300+ lines of inline tests
- 1 `include!` macro pulling in external test file

**Scott Wlaschin Violation:** "Gather together things that change for the same reason." This file changes for many reasons (storage backend changes, error taxonomy changes, admission policy changes, budget logic changes).

---

## 3. Violations

### ❌ CRITICAL: File Size Exceeded (1970 > 300)

```
VIOLATION: File exceeds 300 line maximum
SEVERITY:  CRITICAL
EFFORT:    High (requires module decomposition)
```

### ❌ CRITICAL: DDD Boundary Violations

```
VIOLATION: Multiple aggregates in single file
SEVERITY:  CRITICAL
ENTITIES:  RunAdmission
VALUE OBJECTS: AdmissionBudgetRequest
ERRORS:    ArtifactEnvelopeError, AdmissionError
TRAITS:    ArtifactStore, AcceptedArtifactStore
IMPLS:     AlwaysPresentArtifactStore, MissingAcceptedArtifactStore, StorageArtifactStore
```

### ❌ HIGH: Inline Test Code (300+ lines)

```
VIOLATION: Tests embedded in production module
SEVERITY:  HIGH
LOCATION:  Lines 891-1963
NOTE:      Tests should be in tests/ directory or behind feature gate
```

### ❌ HIGH: `include!` Macro for Tests

```
VIOLATION: include!("admission/artifact_envelope_tests.rs")
SEVERITY:  HIGH
LOCATION:  Line 1969
NOTE:      Cross-module coupling via include! breaks compilation isolation
```

### ⚠️ MEDIUM: God Object Tendency

```
OBSERVATION: Single file handles:
  - Artifact validation
  - Capability checking
  - Budget admission
  - Storage abstraction
  - Error mapping
  - Test stubs
SEVERITY:  MEDIUM
RISK:      Change amplification, merge conflicts, test paralysis
```

---

## 4. DDD Smell Classification

| Smell | Severity | Description |
|-------|----------|-------------|
| **Giant Module** | CRITICAL | 1970 lines in single file |
| **Shared Chaos** | HIGH | 13 domain concepts crammed together |
| **Hidden Dependencies** | HIGH | `include!` pulls in external test file |
| **Test Blob** | HIGH | 300+ lines of inline tests |
| **Feature Envy** | MEDIUM | StorageArtifactStore has deep vb_storage coupling |

---

## 5. Recommended Refactoring

### Phase 1: Error Type Extraction
```
errors/admission.rs         → ArtifactEnvelopeError + AdmissionError
errors/mod.rs               → pub mod admission;
```

### Phase 2: Trait + Impl Extraction
```
stores/artifact_store.rs    → ArtifactStore trait
stores/accepted_store.rs    → AcceptedArtifactStore trait  
stores/always_present.rs    → AlwaysPresentArtifactStore
stores/missing.rs           → MissingAcceptedArtifactStore
stores/fjall_journal.rs     → StorageArtifactStore
```

### Phase 3: Service Extraction
```
services/admission.rs        → admit_run, admit_artifact_run, admit_run_with_budget
services/mod.rs              → pub mod services;
```

### Phase 4: Entity + Value Object
```
domain/admission.rs          → RunAdmission, AdmissionBudgetRequest
domain/mod.rs                → pub mod domain;
```

### Phase 5: Test Extraction
```
tests/admission_artifact_envelope.rs  → artifact_envelope_tests module
tests/admission_run.rs                → run admission tests
```

---

## 6. Priority Assessment

| Priority | Item | Effort |
|----------|------|--------|
| **P0** | File size reduction (1970 → <300) | High |
| **P0** | Error enum extraction | Medium |
| **P1** | Store trait/impl separation | Medium |
| **P1** | Inline test extraction | Low |
| **P2** | `include!` macro removal | Low |

---

## Conclusion

**Status:** ❌ **ARCHITECTURAL DRIFT DETECTED**

This file is a prime example of "growing organically without boundaries." It violates:
1. Hard limit: 1970 lines vs 300 line max
2. DDD cohesion: 13 concepts where 1-3 per file is ideal
3. Test isolation: Inline tests + `include!` macro

**Immediate action required** before any new features can be safely added to this module.

---

*Report generated by architectural-drift agent*
