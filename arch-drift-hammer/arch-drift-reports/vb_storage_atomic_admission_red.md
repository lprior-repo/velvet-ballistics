# Architectural Drift Report: `vb_core_atomic_admission_red.rs`

## File Summary
- **Path:** `crates/vb_storage/tests/vb_core_atomic_admission_red.rs`
- **Total Lines:** 1283
- **File Size:** 51.5 KB
- **Test Count:** 26 (`#[test]` functions)

## Drift Violations

### 1. Line Count Violation (CRITICAL)
| Metric | Limit | Actual | Status |
|--------|-------|--------|--------|
| Max lines per file | 300 | 1283 | **VIOLATION (+327%)** |

**Rule Violated:** `<300 line files` per `architectural-drift` skill.

### 2. DDD Cohesion Issues

| Issue | Location | Severity |
|-------|----------|----------|
| Large enum `ContractAdmissionError` (lines 40-95) | Single file | Medium |
| `LegacyJournalObservation` enum (lines 98-112) | Single file | Medium |
| Large helper functions | Lines 120-335 | Medium |
| Proptest strategy `arb_minimal_workflow` | Lines 808-885 (77 lines) | High |

### 3. Test Organization
- 12 standard `#[test]` functions (lines 338-801)
- 14 proptest `#[test]` functions (lines 889-1283)
- All tests disabled via `#![cfg(any())]` (line 2)

## Recommendations

### Priority 1: Split This File
**Rationale:** 1283 lines violates the 300-line hard limit by 4.3x.

**Suggested Split:**
```
vb_storage/tests/
  ├── vb_core_atomic_admission_red.rs          (NEW: 300 lines max)
  │   ├── Helper types (AdmissionBoundary, ObservedAdmissionOutcome)
  │   ├── Helper fns (journal_stage_observation, temp_store_path, etc.)
  │   └── 4-6 core integration tests
  ├── vb_core_atomic_admission_red/           (NEW: module directory)
  │   ├── mod.rs                               (re-exports)
  │   ├── contract_errors.rs                  (ContractAdmissionError, LegacyJournalObservation)
  │   ├── helpers.rs                          (journal helpers, workflow factories)
  │   ├── standard_tests.rs                   (12 standard #[test] functions)
  │   └── proptest_tests.rs                   (14 proptest #[test] functions)
```

### Priority 2: Extract Error Types to Shared Module
Move `ContractAdmissionError` and `LegacyJournalObservation` to `vb_core` or `vb_storage` domain module to enable reuse across test modules.

### Priority 3: Extract Proptest Strategy
Move `arb_minimal_workflow()` to a shared test helper crate/module to avoid duplication across test files.

## Metrics
| Metric | Value |
|--------|-------|
| Lines per test (avg) | 49.3 |
| Helper code ratio | ~40% |
| Test code ratio | ~60% |
| Disabled by cfg | Yes (`#![cfg(any())]`) |

## Verdict
**STATUS: REFACTOR REQUIRED**

This file exceeds the 300-line limit by **983 lines**. It must be split into a module directory before landing any new work. The file's `#![cfg(any())]` also gates all tests—determine if this is intentional or a dead test file.
