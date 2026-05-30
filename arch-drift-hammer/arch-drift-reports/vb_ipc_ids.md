# Architectural Drift Report: `vb_ipc/src/ids.rs`

**File**: `crates/vb_ipc/src/ids.rs`
**Date**: 2026-05-29
**Status**: DRIFT DETECTED

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **453** | 300 | **VIOLATION** |

---

## 2. DDD Cohesion Analysis

### Domain Role
This file implements **wire-format IPC identifiers** at the boundary layer. It provides two NewType wrappers:
- `AskTicketId` — identifier for suspended ask responses
- `ActionTicketId` — identifier for external action completion/failure

### Cohesion Assessment: **GOOD**

| Aspect | Status | Notes |
|--------|--------|-------|
| NewType pattern | ✅ | Properly wraps `u64` to prevent primitive obsession |
| `#[repr(transparent)]` | ✅ | Correct representation for zero-cost wrapping |
| Parse don't validate | ✅ | `from_wire()` accepts any `u64`; encoding invariant is self-enforcing |
| `#[must_use]` | ✅ | All public constructors/accessors marked |
| No `unsafe` | ✅ | File-level `#![forbid(unsafe_code)]` |
| Serde derives | ✅ | Proper Serialize/Deserialize for IPC transport |
| Documentation | ✅ | Clear doc comments explaining wire encoding |

### Observed Code Duplication (TEST MODULE)

The test module contains **massive duplication** — identical test functions appear twice:

| Test Function | Appears At | Duplicated At |
|---------------|------------|---------------|
| `ask_ticket_id_ordering_by_wire_value` | L169 | L300 |
| `action_ticket_id_ordering_by_wire_value` | L176 | L308 |
| `ask_ticket_id_step_idx_masks_upper_bits` | L185 | L338 |
| `action_ticket_id_step_idx_masks_upper_bits` | L193 | L346 |
| `ask_ticket_id_serde_roundtrip` | L201 | L358 |
| `action_ticket_id_serde_roundtrip` | L211 | L369 |
| `ask_ticket_id_serde_roundtrip_boundary` | L222 | L380 |
| `action_ticket_id_serde_roundtrip_boundary` | L235 | L393 |
| `ask_ticket_id_hash_consistency` | L248 | L410 |
| `action_ticket_id_hash_consistency` | L258 | L420 |

This duplication accounts for ~200+ wasted lines.

---

## 3. Violations

### Hard Violations (Must Fix)

| ID | Violation | Severity | Rule |
|----|-----------|----------|------|
| V1 | **File exceeds 300 lines** (453 total) | CRITICAL | Architectural drift rule |
| V2 | **Test duplication** — 10 test functions duplicated verbatim | HIGH | DDD cohesion / test hygiene |

### Violation Details

**V1: Line Count**
- **Current**: 453 lines
- **Limit**: 300 lines
- **Overflow**: 153 lines (51% over limit)
- **Root cause**: Test duplication + inline tests in same file

**V2: Test Duplication**
- ~200 lines of tests are exact duplicates
- Reducing duplication would bring file to ~253 lines (under limit)

---

## 4. Recommendations

### Immediate (Required)

1. **Move tests to separate file** `ids_tests.rs` or `ids/`
   - Production code in `ids.rs` (42 lines actual logic)
   - Tests in `ids.rs` under `#[cfg(test)]` but deduplicated
   
2. **Deduplicate tests** before moving
   - Collapse duplicate test functions into parameterized tests
   - Use `proptest` for boundary value testing instead of manual repetition

### Refactoring Path

```
ids.rs (42 lines production + 200 lines unique tests)
  ↓ dedup + restructure
ids.rs (42 lines) + ids_test.rs (200 lines deduplicated)
  ↓ if still >300, split further
ids.rs (boundary types) + ticket_ids.rs (AskTicketId + ActionTicketId)
```

---

## 5. DDD Smell Assessment

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive obsession | No | — |
| Anemic domain types | No | — |
| Type distraction | No | — |
| Hidden invalid states | No | — |
| Test duplication smell | **YES** | MEDIUM |

---

## 6. Priority & Effort

| Priority | Effort | Description |
|----------|--------|-------------|
| **P1** | **Low** | Remove duplicate tests (~30 min) |

The production code is well-structured. Only test deduplication and file split required.

---

**Verdict**: `STATUS: DRIFT_DETECTED`

The file violates the 300-line hard limit and contains significant test duplication. Production code quality is high. Fix requires test refactoring only.
