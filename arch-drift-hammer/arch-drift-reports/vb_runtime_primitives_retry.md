# Architectural Drift Report: `vb_runtime::primitives::retry`

**File**: `crates/vb_runtime/src/primitives/retry.rs`  
**Analysis Date**: 2026-05-29  
**Status**: `VIOLATION_DETECTED`

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **1712** | 300 | ❌ EXCEEDED |
| Production code | **437** | 300 | ❌ EXCEEDED |
| Test code | **1275** | N/A | Informational |

**Verdict**: File is **5.7x over the 300-line limit**. Production code alone is **1.5x over limit**.

---

## 2. DDD Cohesion Analysis

### Domain Concepts Identified (5 types)

| Type | Responsibility | Cohesion |
|------|-----------------|----------|
| `DelayStrategy` | Delay enumeration | ✅ Cohesive |
| `RetryPolicy` | Retry configuration VO | ✅ Cohesive |
| `RetryState` | Retry state machine state | ⚠️ Mixed with infrastructure |
| `RetryPolicyError` | Error enumeration | ✅ Cohesive |
| `RetryDecision` | Outcome enumeration | ✅ Cohesive |

### DDD Smells

#### SMELL 1: Infrastructure Intrusion (HIGH SEVERITY)
- **Location**: `RetryState::encode()`, `RetryState::decode()`, `RetryState::write_to_slot()`, `RetryState::read_from_slot()`
- **Problem**: The `RetryState` domain type directly encodes/decodes to `i64` and manipulates `RunFrame` slots. This is persistence/infrastructure concern bleeding into the domain primitive.
- **Violation**: DDD "Pure Domain" principle — domain types should not know about storage representation.

#### SMELL 2: Workflow Orchestration Mixed with Domain Logic (HIGH SEVERITY)
- **Location**: `retry_start()` and `retry_on_failure()` functions
- **Problem**: These are workflow-level orchestration functions that operate on `RunFrame`. They mix imperative shell concerns with the domain primitive module.
- **Violation**: These should be in a separate orchestration or workflow module.

#### SMELL 3: Test Code Bloat (MEDIUM SEVERITY)
- **Problem**: 1275 lines of tests (74% of file) obscures the production code structure.
- **Violation**: File is difficult to navigate; tests should be in a separate `tests/` or `tests/retry.rs` integration module.

---

## 3. Violations Summary

| ID | Violation | Severity | Category |
|----|-----------|----------|----------|
| V1 | Total file exceeds 300 lines (1712) | CRITICAL | Size |
| V2 | Production code exceeds 300 lines (437) | CRITICAL | Size |
| V3 | `RetryState` encodes to `i64` — infrastructure leak | HIGH | DDD |
| V4 | `RetryState::write_to_slot/read_from_slot` — frame manipulation | HIGH | DDD |
| V5 | `retry_start`/`retry_on_failure` — orchestration in domain module | HIGH | DDD |
| V6 | Test code mixed with production code | MEDIUM | Structure |

---

## 4. Remediation Priority

### P0 — Immediate (File exceeds hard limit)
**Action**: Mandatory split into multiple files.

**Proposed Structure**:
```
primitives/
├── mod.rs           # Re-exports
├── delay.rs         # DelayStrategy + compute_delay() (≈80 lines)
├── policy.rs        # RetryPolicy + RetryPolicyError (≈150 lines)
├── state.rs         # RetryState + RetryDecision + encode/decode (≈200 lines)
├── decision.rs      # RetryDecision + is_failure_retriable (≈50 lines)
└── orchestration.rs # retry_start, retry_on_failure, exhaustion_error (≈80 lines)
```

**Tests**: Move to `crates/vb_runtime/tests/retry_tests.rs` or `retry.rs` under `tests/`.

### P1 — Short Term (DDD Hygiene)
1. Extract slot encoding from `RetryState` into a separate `RetryStateCodec` or `RetryStateSlot` adapter.
2. Move `retry_start`/`retry_on_failure` to a workflow/handler module.
3. Add proper module boundaries with `#[cfg(test)]` isolation.

### P2 — Medium Term (Cohesion Polish)
1. Ensure each module has a single responsibility.
2. Consider `SlotIdx` newtype wrapper to make slot operations more explicit.
3. Review `compute_delay` loop — the manual exponentiation loop (lines 386-395) could use `u32::pow()`.

---

## 5. Proof Binding Review

✅ **Loop bounds**: `compute_delay` while-loop bounded by `exponent ≤ u16::MAX`  
✅ **Arithmetic safety**: Uses `checked_mul`, `saturating_add`, `checked_shl`  
✅ **No unsafe code**: `#![forbid(unsafe_code)]` present  
✅ **No panic paths**: All public functions return `Result` or `RetryDecision`  
✅ **Determinism**: State machine is pure function of inputs  

---

## 6. Conclusion

**STATUS**: `REFACTOR_REQUIRED`

This file demonstrates strong domain modeling discipline (proof invariants, arithmetic safety, clear state machine semantics) but **violates the architectural size constraint by 5.7x**. The DDD smells are moderate — the infrastructure intrusion into `RetryState` is the most concerning pattern.

**Priority**: Split immediately. The file cannot land in this state per architectural rules.
