# Architectural Drift Report: `vb_core/src/value_store.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/value_store.rs`  
**Analysis Date**: 2026-05-29  
**Agent**: architectural-drift

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **2552** | 300 | ❌ CRITICAL VIOLATION |

**Excess**: 2252 lines over limit (750% of allowed size)

---

## 2. DDD Cohesion Analysis

| Question | Answer |
|----------|--------|
| Filename reflects single domain concept? | ✅ YES |
| Domain concept | Cold value arenas (symbols, lists, objects, blobs) |
| DDD smell detected? | **NO** |

The filename `value_store.rs` correctly reflects its single domain concept: cold storage/arenas for runtime slot values. The cohesion is intact.

---

## 3. All Violations

### VIOLATION 1: Critical File Oversize (2552 lines)
- **Limit**: 300 lines
- **Actual**: 2552 lines
- **Excess**: 2252 lines (750% of limit)
- **Severity**: CRITICAL
- **Lines**: 1–2552 (entire file)

### VIOLATION 2: Inline Test Module (~2098 lines)
- **Location**: Lines 455–2552 (`#[cfg(test)] mod tests`)
- **Content**: Unit tests, BDD tests, security regression tests, proptest properties
- **Severity**: HIGH
- **Note**: Tests consume ~82% of the file

### VIOLATION 3: Inline Kani Harness Module
- **Location**: Lines 420–449 (`#[cfg(kani)] mod kani_harnesses`)
- **Content**: PO-012 proof harness for capped ValueStore
- **Severity**: MEDIUM
- **Note**: Kani harnesses should be behind a feature gate in `verification/` or `kani/`

### VIOLATION 4: Empty Extended Tests Module Declaration
- **Location**: Line 452 (`#[cfg(test)] mod extended_tests;`)
- **Issue**: Declares an empty module that doesn't exist
- **Severity**: LOW (dead code declaration)

### VIOLATION 5: Missing Module Separation for Helpers
- **Location**: Lines 332–418 (free functions)
- **Content**: `checked_len_to_u64`, `next_*_id`, `validate_*`, `*_index` functions
- **Suggestion**: Could be extracted to `value_store/helpers.rs` or `value_store/private.rs`
- **Severity**: LOW (cosmetic)

---

## 4. Section Breakdown

| Section | Lines | Type |
|---------|-------|------|
| Module doc + imports | 1–12 | Production |
| `ObjectField` struct + impl | 14–41 | Production |
| `ValueStore` struct + impl | 43–330 | Production |
| Private helper functions | 332–418 | Production |
| `#[cfg(kani)]` harness | 420–449 | Verification |
| `#[cfg(test)] mod extended_tests;` | 451–452 | Declaration (empty) |
| `#[cfg(test)] mod tests;` | 454–2552 | Tests |
| **Total Production** | ~330 | |
| **Total Non-Production** | ~2222 | |

---

## 5. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (CRITICAL)** | Split file — move tests to `value_store/tests.rs` or `value_store/test_harness.rs` | High |
| **P1 (HIGH)** | Move `#[cfg(kani)]` to `vb_core/verification/kani/value_store_proofs.rs` | Medium |
| **P2 (MEDIUM)** | Remove empty `mod extended_tests;` declaration | Trivial |
| **P3 (LOW)** | Extract private helpers to `value_store/private.rs` (optional) | Low |

---

## 6. Recommended Split Strategy

```
vb_core/src/value_store.rs     (~330 lines) — Production code only
vb_core/src/value_store/tests.rs  (~2100+ lines) — All inline tests
vb_core/verification/kani/value_store.rs  — Kani harnesses (if not already in workspace_tests)
```

Or alternatively per repository convention for test placement in `crates/workspace_tests/`.

---

## Summary

| Metric | Result |
|--------|--------|
| Lines Count | **2552** (❌ over 300) |
| DDD Cohesion | ✅ PASS |
| Violations | 5 total (1 critical, 1 high, 1 medium, 2 low) |
| **Remediation Priority** | **P0 — CRITICAL — File must be split** |
