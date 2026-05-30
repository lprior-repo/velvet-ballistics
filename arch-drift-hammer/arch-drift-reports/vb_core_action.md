# Architectural Drift Report: `vb_core/src/action.rs`

**File**: `crates/vb_core/src/action.rs`
**Analysis Date**: 2026-05-29
**Status**: ❌ CRITICAL DRIFT DETECTED

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **2287** | 300 | ❌ 661% OVER |

**Breakdown**:
- Production code (types + functions): ~522 lines (lines 1–522)
- Inline test module: **1765 lines** (lines 569–2287)

---

## 2. DDD Cohesion Analysis

### ✅ Cohesive Elements (Domain Model is Sound)

The file implements a **coherent Action Domain** with proper DDD elements:

| DDD Concept | Types Present | Status |
|-------------|---------------|--------|
| **Value Objects** | `Idempotency`, `SideEffect`, `RetrySafety`, `RetryPolicy`, `ActionFailureCode` | ✅ |
| **Entities** | `ActionContract`, `ActionTicket`, `ActionInput`, `ActionOutput`, `ActionOutputReady`, `ActionFailure` | ✅ |
| **Domain Events** | `ActionJournalEvent` (Suspended/Completed/Failed) | ✅ |
| **Error Types** | `IdempotencyViolation`, `ActionError` | ✅ |
| **Domain Functions** | `propagate_action_taint`, `compute_action_idempotency_key`, `action_ticket_has_valid_key`, `validate_idempotency_key_ingredients`, `verify_idempotency`, `validate_action_dispatch`, `issue_action_ticket`, `validate_action_outcome` | ✅ |

**Verdict**: DDD cohesion is **excellent**. The domain model is well-structured with proper separation of concerns within the action bounded context.

### ⚠️ Architectural Smell: Monolithic Single-File Structure

Despite good DDD internal cohesion, the file violates the **<300 line rule** and **module separation** principles.

---

## 3. Violations

### V-001: FILE SIZE EXCEEDED — CRITICAL
- **Severity**: P0 (Blocking)
- **Location**: Entire file
- **Actual**: 2287 lines
- **Limit**: 300 lines
- **Overflow**: 1987 lines (661% of limit)

### V-002: INLINE TEST MODULE — VIOLATION
- **Severity**: P1 (Must Fix)
- **Location**: Lines 569–2287 (`#[cfg(test)] mod tests`)
- **Issue**: 1765 lines of tests embedded in production source file
- **Remediation**: Extract to `action/tests/action_contract_tests.rs` or similar

### V-003: NO MODULE SEPARATION — VIOLATION
- **Severity**: P1 (Must Fix)
- **Location**: Entire file is one monolithic module
- **Issue**: All types, functions, and tests in single file
- **Expected structure**:
  ```
  src/action/
    mod.rs          # Re-exports and public API
    types.rs        # Value objects and enums
    entities.rs     # ActionContract, ActionTicket, etc.
    errors.rs       # ActionError, IdempotencyViolation
    journal.rs      # ActionJournalEvent
    validation.rs  # Validation functions
    tactics.rs     # Pure domain functions (propagate_taint, etc.)
  ```

### V-004: OVERSIZED HELPER FUNCTIONS (Minor)
- **Location**: `validate_idempotency_key_ingredients` (lines 347–381)
- **Lines**: 35 lines
- **Verdict**: Acceptable; single-pass loop with early returns is idiomatic

---

## 4. Production Code Quality Assessment

| Function | Lines | Assessment |
|----------|-------|------------|
| `propagate_action_taint` | 11 | ✅ Clean, total-functions style |
| `compute_action_idempotency_key` | 12 | ✅ Deterministic, no side effects |
| `action_ticket_has_valid_key` | 4 | ✅ Thin wrapper |
| `validate_idempotency_key_ingredients` | 35 | ✅ Acceptable loop |
| `verify_idempotency` | 19 | ✅ Clear branching |
| `validate_action_dispatch` | 19 | ✅ Guard clauses |
| `issue_action_ticket` | 19 | ✅ Simple construction |
| `validate_action_outcome` | 9 | ✅ Delegates properly |
| Helper validators | 3–9 each | ✅ Small, focused |

**Production code is ~522 lines and well-structured.**

---

## 5. DDD Smell Summary

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive Obsession | ❌ No (uses NewTypes: `ActionId`, `RunId`, etc.) | None |
| Inline Tests | ✅ Yes (1765 lines) | High |
| Anemic Domain Model | ❌ No (rich validation logic) | None |
| Missing Module Separation | ✅ Yes | High |
| Feature Envy | ❌ No | None |
| Shotgun Surgery | ❌ No | None |

**Primary Smell**: **Inline Tests + Monolithic File** — Not a domain model problem, but a file organization problem.

---

## 6. Remediation Priority

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| **P0** | Split file into `action/` module directory | High | Unblocks CI |
| **P1** | Extract tests to `action/tests/integration_tests.rs` | Medium | Reduces file by 1765 lines |
| **P2** | Group types into `types.rs`, `entities.rs`, `errors.rs`, `journal.rs` | Medium | Improves navigability |

### Recommended Module Structure

```
crates/vb_core/src/action/
├── mod.rs           # 50 lines: public re-exports
├── types.rs         # 150 lines: Idempotency, SideEffect, RetrySafety, RetryPolicy, ActionFailureCode
├── entities.rs      # 150 lines: ActionContract, ActionTicket, ActionInput, ActionOutput
├── errors.rs        # 100 lines: ActionError, IdempotencyViolation
├── journal.rs       # 60 lines: ActionJournalEvent
├── validation.rs    # 150 lines: All validate_* functions
├── tactics.rs       # 100 lines: propagate_action_taint, compute_action_idempotency_key
└── tests/           # 1765 lines: moved to action/tests/
    ├── taint_prop.rs
    ├── idempotency.rs
    ├── dispatch.rs
    ├── ticket.rs
    └── journal.rs
```

---

## 7. Summary

| Metric | Result |
|--------|--------|
| **Line Count** | 2287 (❌ Exceeds 300 by 1987 lines) |
| **DDD Cohesion** | ✅ Excellent — coherent domain model |
| **DDD Smell** | ⚠️ Inline tests + monolithic structure |
| **Oversized Functions** | ✅ None (production functions are all <40 lines) |
| **Missing Module Separation** | ❌ Yes — needs module directory |
| **Remediation Priority** | **P0** — Must split file before merge |

**Recommendation**: Refactor `action.rs` into `action/` module directory. Production code is high quality; structural reorganization only.
