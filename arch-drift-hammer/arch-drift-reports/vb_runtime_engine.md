# Architectural Drift Report: vb_runtime::engine

**Module**: `crates/vb_runtime/src/engine/`
**Date**: 2026-05-29
**Status**: DRIFT DETECTED

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 8,388 |
| **Total Files** | 10 |
| **Violations (>300 lines)** | 5 files |
| **DDD Cohesion** | GOOD (bounded contexts are correct) |
| **Priority** | HIGH (massive test modules mask implementation bloat) |

---

## File-by-File Line Count

| File | Lines | Status |
|------|-------|--------|
| tests.rs | 2,555 | **VIOLATION** |
| execute.rs | 1,910 | **VIOLATION** |
| drive.rs | 1,383 | **VIOLATION** |
| types.rs | 1,180 | **VIOLATION** |
| action.rs | 674 | **VIOLATION** |
| property_tests.rs | 250 | OK |
| retry_math.rs | 207 | OK |
| signal.rs | 167 | OK |
| mod.rs | 39 | OK |
| helpers.rs | 23 | OK |

---

## Violations

### V1: tests.rs — 2,555 lines (CRITICAL)
**Problem**: Massive test file contains both integration tests and black-hat security review tests.

**Recommendation**: Split into:
- `tests_integration.rs` — BDD-style drive loop tests (~800 lines)
- `tests_blackhat.rs` — Security findings (~900 lines)
- `tests_proptests.rs` — Property-based tests (~200 lines)

**Root Cause**: Tests were not split when module grew.

---

### V2: execute.rs — 1,910 lines (CRITICAL)
**Problem**: Single monolithic `execute_node_full` function with giant match block (lines 55-370) covering ~25 node kinds, plus 1,540 lines of inline tests.

**Recommendation**: Split by node kind categories:
- `execute_primitives.rs` — ForEach, Reduce, Repeat iteration primitives
- `execute_primitives_collect.rs` — CollectStart/Next/Page/Finish
- `execute_primitives_together.rs` — TogetherStart/Branch/Join
- `execute_primitives_wait.rs` — WaitUntil, WaitEvent, Ask, AskResume
- `execute_action.rs` — Do node handling
- `execute_misc.rs` — RetryCheck, ErrorHandler, fallback to step_once

Move tests to `tests_execute_*.rs`.

---

### V3: drive.rs — 1,383 lines (CRITICAL)
**Problem**: Drive loop implementation (~165 lines) plus comprehensive test suite (~1,218 lines).

**Recommendation**:
- Extract helper functions (`cn`, `fin`, `nop`, `setc`, `cpy`, `don`, `collect_start`, etc.) to `drive_helpers.rs`
- Move test helpers to `tests_drive_helpers.rs`
- Move tests to `tests_drive.rs`

---

### V4: types.rs — 1,180 lines (HIGH)
**Problem**: Type definitions (~295 lines production, ~885 lines tests). EvidenceCollector has significant inline test coverage.

**Recommendation**:
- Move `#[cfg(test)]` module to `tests/types_tests.rs`
- Consider extracting `EvidenceCollector` to its own file if it grows further

---

### V5: action.rs — 674 lines (MEDIUM)
**Problem**: Action execution helpers with inline tests (~450 lines production, ~224 tests).

**Recommendation**:
- Production code is well-structured, just extract tests to `tests/action_tests.rs`

---

## DDD Cohesion Analysis

**GOOD**: The module correctly follows Scott Wlaschin DDD principles:

| File | DDD Role | Cohesion |
|------|----------|----------|
| `types.rs` | Value Objects & Error Types | HIGH — single bounded context |
| `action.rs` | Domain Services | HIGH — action execution logic |
| `execute.rs` | Workflow Engine | HIGH — node dispatch |
| `drive.rs` | Orchestration | HIGH — drive loop |
| `signal.rs` | Signal Translation | HIGH — thin adapter |
| `helpers.rs` | Utilities | MEDIUM — thin but appropriate |
| `retry_math.rs` | Pure Calculations | HIGH — deterministic retry logic |

**DDD Smell**: `types.rs` contains `EvidenceCollector` which is a COLLECTION VALUE OBJECT with capacity-bounded behavior. This is appropriate but borders on SERVICE. Consider if `EvidenceCollector` should live in a `evidence` submodule.

---

## Primitive Obsession Violations

| Location | Issue | Remediation |
|----------|-------|-------------|
| `types.rs:53` | `DEFAULT_EVIDENCE_CAPACITY: usize = 3 * 1024` | Wrap in `NonZeroUsize` or typed `Capacity` |
| `types.rs:65-67` | `capacity: usize`, `dropped: usize` | Consider `Count` newtypes |
| `retry_math.rs:26-29` | `RetryPolicyLimits { max_attempts: u16, max_interval_ms: u64 }` | These are fine — already bounded |
| `action.rs:31` | `let action_index = usize::from(action.get())` | Acceptable — index into vector |

---

## Test Module Organization (Structural Violation)

The 300-line rule is being violated by **test code inflation**, not implementation complexity. This is a structural smell:

```
engine/
├── mod.rs           (39 lines)  ✓
├── types.rs         (1180 lines) ← 295 prod + 885 tests
├── action.rs        (674 lines)  ← 450 prod + 224 tests  
├── drive.rs         (1383 lines) ← 165 prod + 1218 tests
├── execute.rs       (1910 lines) ← 370 prod + 1540 tests
├── helpers.rs       (23 lines)   ✓
├── signal.rs       (167 lines)  ← 33 prod + 134 tests
├── retry_math.rs   (207 lines)  ✓ (pure code, minimal tests)
├── property_tests.rs (250 lines) ✓
└── tests.rs        (2555 lines) ← integration + blackhat
```

**Actual Production Code**: ~1,400 lines across 7 production files
**Actual Test Code**: ~6,988 lines across 4 test-heavy files

---

## Recommendations (Priority Order)

1. **P0**: Split `tests.rs` (2555 lines) into `tests_integration.rs`, `tests_blackhat.rs`, `tests_proptests.rs`
2. **P0**: Split `execute.rs` (1910 lines) — extract node-kind-specific handlers
3. **P1**: Split `drive.rs` (1383 lines) — extract test helpers
4. **P1**: Extract tests from `types.rs` (1180 lines)  
5. **P2**: Extract tests from `action.rs` (674 lines)
6. **P3**: Create `NonZeroUsize` wrapper for evidence capacity constant

---

## Compliance Status

| Rule | Status |
|------|--------|
| File < 300 lines | ❌ FAIL (5 violations) |
| DDD Cohesion | ✅ PASS |
| No primitive obsession in core types | ⚠️ MINOR (capacity usize) |
| Test extraction | ❌ FAIL (tests inline in production files) |

---

**CONCLUSION**: The module has sound DDD structure but severe file-size drift caused by test accumulation. Priority is to extract tests into dedicated test files before the module becomes unmaintainable.
