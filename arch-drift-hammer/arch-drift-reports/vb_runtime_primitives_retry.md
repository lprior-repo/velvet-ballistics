# Architectural Drift Report: vb_runtime_primitives_retry

**File**: `crates/vb_runtime/src/primitives/retry.rs`
**Date**: 2026-05-29
**Analyst**: architectural-drift agent

---

## Executive Summary

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **1712** | 300 | 🔴 FAIL (471% over) |
| Production Code | ~437 | 300 | 🔴 FAIL (46% over) |
| Test Code | ~1275 | N/A | 🔴 SMELL (74% of file) |
| DDD Cohesion | LOW | HIGH | 🔴 FAIL |

---

## 1. Line Count Violations

### Total: 1712 lines (LIMIT: 300)

| Section | Lines | % of Limit | Status |
|---------|-------|------------|--------|
| Production code (1-437) | 437 | 146% | 🔴 FAIL |
| Test code (438-1712) | 1275 | 425% | 🔴 SMELL |
| **Total** | **1712** | **571%** | **🔴 FAIL** |

The file is **5.7x the maximum allowed size**.

---

## 2. DDD Cohesion Analysis

### Single File Contains Multiple Bounded Contexts

| DDD Concept | Type | Responsibility | Smell |
|-------------|------|-----------------|-------|
| `DelayStrategy` | Value Object | Delay enumeration | ✓ Cohesive |
| `RetryPolicy` | Value Object | Configuration | ✓ Cohesive |
| `RetryPolicyError` | Error Type | Domain errors | ✓ Cohesive |
| `RetryState` | Entity | State machine | ✓ Cohesive |
| `RetryDecision` | Enum | Outcome | ✓ Cohesive |
| `is_failure_retriable` | Domain Function | Policy evaluation | ✓ Cohesive |
| `evaluate_retry` | Domain Function | State transitions | ✓ Cohesive |
| `compute_delay` | Domain Function | Delay calculation | ✓ Cohesive |
| `exhaustion_error` | Domain Function | Error mapping | ✓ Cohesive |
| `retry_start` | Application Service | Frame slot init | 🔴 INFRASTRUCTURE |
| `retry_on_failure` | Application Service | Frame slot ops | 🔴 INFRASTRUCTURE |
| `RetryState::write_to_slot` | Infrastructure | Serialization | 🔴 INFRASTRUCTURE |
| `RetryState::read_from_slot` | Infrastructure | Deserialization | 🔴 INFRASTRUCTURE |
| **Tests (438-1712)** | **Test Module** | **~1275 lines** | 🔴 MIXED |

### Cohesion Verdict: **LOW**

The file violates:
- **Single Responsibility Principle**: Infrastructure concerns (RunFrame slot I/O) mixed with domain logic
- **Package Cohesion**: 5+ distinct concepts crammed into one file
- **Separation of Concerns**: Tests consume 74% of the file

---

## 3. All Violations

### Critical (MUST FIX)

| # | Violation | Rule | Location |
|---|-----------|------|----------|
| 1 | **Line count 1712 > 300** | File size limit | Entire file |
| 2 | **Production code exceeds 300 lines** | File size limit | Lines 1-437 |
| 3 | **Inline tests 1275 lines** | Test separation | Lines 438-1712 |

### Major (SHOULD FIX)

| # | Violation | Rule | Location |
|---|-----------|------|----------|
| 4 | **Infrastructure in domain file** | Hexagonal boundary | `write_to_slot`, `read_from_slot` at lines 254-277 |
| 5 | **RunFrame dependency** | Dependency inversion | Lines 254-277, 410-436 |

### Minor (NICE TO FIX)

| # | Violation | Guideline |
|---|-----------|-----------|
| 6 | `#[allow(unreachable_code)]` at line 313 | Future-proofing catch-all is questionable |

---

## 4. DDD Smell Assessment

```
SMELL: God File Pattern
```

The file exhibits the **"God File" anti-pattern**:
- 1712 lines of tightly coupled concepts
- 14 public types/functions in a single file
- 3 distinct architectural layers (domain, application, infrastructure) in one file
- Tests are 3.4x larger than production code

**Scott Wlaschin DDD Violations**:
1. ❌ One file contains multiple aggregates (`RetryPolicy`, `RetryState`)
2. ❌ Value objects and entities not separated
3. ❌ Infrastructure polluting domain layer
4. ❌ Application services (`retry_start`, `retry_on_failure`) in same file as domain

---

## 5. Remediation Priority

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| **P0 - CRITICAL** | Extract tests to `retry/tests.rs` | Medium | Reduces file to 437 lines |
| **P0 - CRITICAL** | Split domain from infrastructure | High | Reduces production to ~300 lines |
| **P1 - MAJOR** | Create `retry/policy.rs` for RetryPolicy, DelayStrategy, RetryPolicyError | Medium | Improves cohesion |
| **P1 - MAJOR** | Create `retry/state.rs` for RetryState entity | Medium | Isolates state machine |
| **P2 - MINOR** | Create `retry/decision.rs` for RetryDecision and evaluation | Low | Completes split |

---

## 6. Recommended File Structure

```
crates/vb_runtime/src/primitives/retry/
├── mod.rs           # Re-exports, line ~50
├── policy.rs        # RetryPolicy, DelayStrategy, RetryPolicyError (~150 lines)
├── state.rs         # RetryState entity (~200 lines)
├── decision.rs      # RetryDecision, evaluate_retry, compute_delay (~150 lines)
└── tests.rs         # All tests (~1275 lines) or move to workspace_tests/
```

---

## 7. Verification Commands

```bash
# Count lines
wc -l crates/vb_runtime/src/primitives/retry.rs
# Expected: < 300

# Check for infrastructure leaks
grep -n "write_to_slot\|read_from_slot\|RunFrame" crates/vb_runtime/src/primitives/retry.rs
# Expected: Only in infrastructure layer after split
```

---

## Conclusion

**Status**: 🔴 **ARCHITECTURAL DRIFT DETECTED**

This file is in **serious violation** of the architectural guidelines:
- 5.7x over the line count limit
- Infrastructure and domain mixed together
- Tests consume 74% of the file

**Immediate action required** to restore architectural integrity.
