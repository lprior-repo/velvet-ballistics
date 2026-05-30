# Architectural Drift Report: `vb_runtime/src/action.rs`

## File: `crates/vb_runtime/src/action.rs`

---

## 1. Line Count Analysis

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | **904** | 300 | **CRITICAL VIOLATION** |
| Production code | ~137 lines (lines 1–166) | — | OK |
| Test code | ~736 lines (lines 168–904) | — | VIOLATION |

**Verdict**: Line count exceeds threshold by **201%**. File MUST be split.

---

## 2. DDD Cohesion Analysis

### Domain Elements Identified

| Element | Type | Location | Assessment |
|---------|------|----------|------------|
| `ActionRegistry` | Domain Service | Lines 17–122 | ✅ Cohesive |
| `ActionSlot` | Internal Enum | Lines 21–25 | ✅ Cohesive |
| `dispatch_generic` | Domain Function | Lines 140–152 | ⚠️ Placeholder |
| `validate_input_bytes` | Domain Function | Lines 155–166 | ⚠️ Placeholder |

### DDD Smells

1. **Inline Test Pollution**: 736 lines of tests embedded in production module
   - Tests belong in `crates/vb_runtime/src/action/tests.rs` or `crates/vb_runtime/tests/action_tests.rs`
   - Violates: One responsibility / separation of concerns

2. **Cross-Module Test Leakage**: `IdempotencyTracker` tests (lines 811–903) live in `action.rs` but `IdempotencyTracker` is `pub use` from `idempotency.rs`
   - Violates: A module should not contain tests for another module it imports

3. **Placeholder Implementation**:
   - `validate_input_bytes` (lines 155–166): "This is a structural check placeholder; actual byte counting happens at the IPC boundary"
   - `dispatch_generic` (lines 140–152): Comment states "In generated mode, this becomes a match on ActionId"
   - Violates: `Parse, don't validate` principle — actual validation is deferred elsewhere

4. **Primitive Obsession (Minor)**: `MAX_REGISTERED_ACTIONS` is `usize` literal `65_535` — could be a named constant type

---

## 3. Violations Summary

| ID | Severity | Rule | Description |
|----|----------|------|-------------|
| V1 | **CRITICAL** | File size | 904 lines exceeds 300-line maximum |
| V2 | HIGH | DDD cohesion | Inline tests (736 lines) pollute production module |
| V3 | HIGH | DDD cohesion | `IdempotencyTracker` tests in wrong module |
| V4 | MEDIUM | Completeness | `validate_input_bytes` is placeholder — no actual byte validation |
| V5 | MEDIUM | Completeness | `dispatch_generic` admits generated code will replace it |
| V6 | LOW | Type safety | `MAX_REGISTERED_ACTIONS` should be typed `u16::MAX` |

---

## 4. Recommended Refactoring

### Step 1: Extract Tests
```
crates/vb_runtime/src/action.rs         → date: ~137 lines (production only)
crates/vb_runtime/src/action/tests.rs  → ~736 lines (all inline tests)
```

### Step 2: Fix `validate_input_bytes`
Implement actual byte validation or mark as `unimplemented!()` until IPC boundary is defined.

### Step 3: Address `IdempotencyTracker` Test Location
Move `IdempotencyTracker` tests (lines 811–903) to `idempotency.rs` or `idempotency/tests.rs`.

---

## 5. Priority & Effort

| Priority | Level |
|----------|-------|
| **Priority** | **P0 — CRITICAL** |
| Reason | Hard architectural rule violated (904 >> 300 lines) |
| Estimated refactor effort | Low (file split only; no logic changes) |

---

## 6. Status

```
STATUS: REFACTOR REQUIRED
```

File exceeds architectural limits. Splitting tests is the minimum required action.
