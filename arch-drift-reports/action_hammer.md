# Architectural Drift Report: `action.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/action.rs`  
**Status:** CATASTROPHIC VIOLATION  
**Line Count:** 2287 lines (**7.6x over the 300-line limit**)

---

## Executive Summary

The `action.rs` file is a **2287-line monolithic blob** that violates:
1. **<300 line rule** (hard architectural constraint)
2. **Single Responsibility Principle** (DDD)
3. **Scott Wlaschin DDD: "Type-Driven Design"** — primitive obsession violations
4. **Cohesion** — domain types, validation, errors, journal, and tests all混在一起

---

## Violation Breakdown

### 1. LINE COUNT VIOLATION (CRITICAL)

| Section | Lines | Limit | Violation |
|---------|-------|-------|-----------|
| Domain types + impls | ~307 | 300 | SLIGHT |
| Standalone functions | ~365 | 300 | OVER |
| Journal events | ~45 | 300 | OK |
| **Tests only** | **~1718** | **N/A** | **75% of file!** |
| **TOTAL** | **2287** | **300** | **+1987 lines** |

**The tests alone are 1718 lines** — this is 5.7x the entire 300-line budget.

---

## Domain Map

### Current Structure (Single File)

```
action.rs (2287 lines)
├── Type Definitions (lines 1-307)
│   ├── Idempotency (enum)
│   ├── SideEffect (enum)
│   ├── RetrySafety (enum)
│   ├── RetryPolicy (enum)
│   ├── IdempotencyViolation (enum)
│   ├── ActionContract (struct)
│   ├── ActionInput (struct)
│   ├── ActionOutput (struct)
│   ├── ActionTicket (struct)
│   ├── ActionOutputReady (struct)
│   ├── ActionFailure (struct)
│   ├── ActionFailureCode (enum)
│   ├── ActionError (enum + impl)
│   └── ActionOutcome (enum)
├── Standalone Functions (lines 156-521)
│   ├── compute_action_idempotency_key()
│   ├── action_ticket_has_valid_key()
│   ├── propagate_action_taint()
│   ├── validate_idempotency_key_ingredients()
│   ├── verify_idempotency()
│   ├── validate_action_dispatch()
│   ├── issue_action_ticket()
│   ├── validate_action_outcome()
│   └── helper fns (validate_ready_outcome, check_output_*, etc.)
├── Journal Events (lines 527-567)
│   └── ActionJournalEvent (enum)
└── Tests (lines 569-2287)
    ├── Phase 2 adversarial BDD tests
    ├── Phase 38 tests
    ├── Phase 18-19 tests
    ├── Edge-case tests
    ├── vb-8mdp.6 proptest tests
    └── Property tests (deterministic_rand, canonical key validation)
```

---

## Scott Wlaschin DDD Violations

### A. Primitive Obsession (Type-Driven Design Failures)

| Primitive Type | Usage | Should Be |
|---------------|-------|-----------|
| `u16` | `ActionTicket.attempt` | `AttemptCount(u16)` |
| `u16` | `ActionTicket.capacity` | `RetryCapacity(u16)` |
| `u128` | `ActionTicket.idempotency_key` | `IdempotencyKey(u128)` |
| `u64` | `ActionContract.timeout_ms` | `TimeoutMs(u64)` |
| `u32` | `IdempotencyViolation::SecretInKey(u32)` | `SlotIdx` |
| `u32` | `IdempotencyViolation::RandomInKey(u32)` | `SlotIdx` |
| `u32` | `IdempotencyViolation::TimeInKey(u32)` | `SlotIdx` |

**Impact:** These raw types allow invalid values to be constructed. For example:
- `ActionTicket { attempt: 0, ... }` — attempt is 1-indexed but nothing enforces it
- `timeout_ms: u64::MAX` — no bounds check at type level
- Slot indices use raw `u32` instead of the existing `SlotIdx` newtype

### B. Mixing Concerns — "Bunched" Domain Model

Per Scott Wlaschin, a **module should represent ONE concept**. This file crams:

| Concern | What It Does | Should Be |
|---------|--------------|-----------|
| **ABI Contract** | `ActionContract` describes dispatch metadata | `action/contracts.rs` |
| **Ticket/Tracking** | `ActionTicket`, `ActionInput`, `ActionOutput` | `action/ticket.rs` |
| **Taint Propagation** | `propagate_action_taint()` | `action/taint.rs` or `value/taint.rs` |
| **Validation** | `verify_idempotency`, `validate_action_dispatch` | `action/validation.rs` |
| **Error Modeling** | `ActionError`, `ActionFailureCode` | `action/error.rs` |
| **Journal** | `ActionJournalEvent` | `action/journal.rs` |
| **Tests** | 1718 lines of test cases | `workspace_tests/action_tests.rs` |

### C. Validation Logic Entanglement

`validate_idempotency_key_ingredients()` and `verify_idempotency()` are **validation functions** that know about `RunFrame` internals. This couples the action domain to the frame domain.

```rust
// VIOLATION: Validation in domain module knows about RunFrame internals
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,  // <-- Domain boundary leak
) -> Result<(), IdempotencyViolation>
```

---

## Specific Findings

### Finding 1: ATTEMPT IS 1-INDEXED BUT UNENFORCED

```rust
pub struct ActionTicket {
    pub attempt: u16,  // Should be AttemptCount(u16) with invariants
    pub capacity: u16,  // Should be RetryCapacity(u16)
}
```

Test shows `attempt: 1` is valid but nothing prevents `attempt: 0`.

### Finding 2: IDEMPOTENCY KEY IS RAW u128

```rust
pub idempotency_key: u128,  // Should be IdempotencyKey(u128)
```

### Finding 3: TIMEOUT IS RAW u64

```rust
pub timeout_ms: u64,  // Should be TimeoutMs(u64)
```

### Finding 4: TESTS ARE 75% OF FILE

The test module (lines 569-2287) contains **1718 lines of tests**. This is:
- 5.7x the entire 300-line limit
- Larger than the entire production code + domain types combined
- Should be in `crates/workspace_tests/` per repository structure

### Finding 5: VALIDATION FUNCTIONS COUPLE TO RunFrame

Functions like `validate_idempotency_key_ingredients` and `verify_idempotency` take `&RunFrame` as a parameter, creating hidden coupling between the action module and the frame module.

### Finding 6: join_taint IS UNNECESSARY

```rust
const fn join_taint(input: Taint) -> Taint {
    input
}
```

This is a no-op function that adds complexity without value.

---

## Recommended Refactoring

### Proposed Module Structure

```
vb_core/src/action/
├── mod.rs           # Re-exports
├── types.rs         # Domain types only (enums, structs) ~300 lines
├── ticket.rs        # ActionTicket, ActionInput, ActionOutput ~150 lines
├── contract.rs      # ActionContract ~100 lines
├── error.rs         # ActionError, ActionFailureCode, ActionFailure ~150 lines
├── taint.rs         # propagate_action_taint, join_taint ~50 lines
├── validation.rs    # verify_idempotency, validate_* ~200 lines
├── journal.rs       # ActionJournalEvent ~50 lines
└── ids.rs           # Newtype wrappers: AttemptCount, IdempotencyKey, TimeoutMs ~100 lines
```

### Newtype Wrappers Required

```rust
// In action/ids.rs
pub struct AttemptCount(u16);
pub struct RetryCapacity(u16);
pub struct IdempotencyKey(u128);
pub struct TimeoutMs(u64);
```

### Test Extraction

```
crates/workspace_tests/
└── vb_core/
    └── action_tests.rs  # Extracted from action.rs tests
```

---

## Risk Assessment

| Risk | Severity | Likelihood | Notes |
|------|----------|------------|-------|
| Unenforced invariants (attempt=0) | HIGH | MEDIUM | Runtime bug, hard to trace |
| Primitive obsession | MEDIUM | HIGH | Type safety lost |
| Frame coupling | MEDIUM | HIGH | Validation leaks implementation detail |
| Test maintenance | MEDIUM | HIGH | 1718-line test block is unmaintainable |
| Line count | **CRITICAL** | **CERTAIN** | 2287 lines vs 300 limit |

---

## Verdict

**STATUS: MUST REFACTOR**

This file is a **textbook example of domain model bunching** — all concepts related to "actions" dumped into a single file regardless of concern separation. The 2287-line size is a structural hazard that makes the code:
- Hard to navigate
- Hard to test
- Hard to reason about
- Prone to primitive obsession bugs

**Required Actions:**
1. Extract all newtypes (AttemptCount, IdempotencyKey, TimeoutMs)
2. Split into domain-specific modules per DDD
3. Move tests to `workspace_tests/`
4. Remove `join_taint` no-op
5. Decouple validation from RunFrame

---

*Report generated by architectural-drift agent*
