# Contract Verification Review — vb-e4mt (Attempt 3/7)

## Bead: Resource Bounds and Budget Enforcement
**State**: 6 (contract-verification-review re-review)
**Workdir**: `/home/lewis/src/vb-e4mt-workspace`
**Actual Code Workspace**: `/home/lewis/src/velvet-ballistics`
**Date**: 2026-05-19

---

## Executive Summary

| Check | Result | Change from Attempt 2 |
|-------|--------|----------------------|
| All contract clauses have traceability | PASS | — |
| TLA+ specs exist and parse | PASS (3/3) | — |
| TLA+ model checking completed | 2/3 PASS, 1 INCONCLUSIVE | TLA-WF-002 resolved to PASS |
| Verus spec functions exist | PASS (files exist) | — |
| Verus execution completed | FAIL (BLOCKED) | Namespace mismatch unresolved |
| Kani harnesses exist | PASS (file exists at velvet-ballistics) | — |
| Kani harnesses compilable | **FAIL (module not in lib.rs)** | NEW BLOCKER |
| Kani harnesses executed | **0/5 blocked, 2/other PASS** | 2 alternate harnesses confirmed PASS |
| Proptest waiver issued | PASS (WAIVER-PROP-KERNEL-001) | — |
| Fuzz target exists | PASS | — |
| Workspace path consistency | PASS (metadata correct) | Resolved |
| Formal waivers complete | **FAIL** (missing module declaration waiver) | New finding |

---

## Critical Finding: Misleading BLOCKED_TOOLING Status

### The Problem

KANI-BUDGET-001..005 are recorded as `BLOCKED_TOOLING` in proof-evidence.md. This implies Kani tool is unavailable. **This is incorrect.**

**Actual State**:
- `cargo kani --version` → 0.67.0 (installed and functional)
- `cargo kani -p vb_core --harness kani_step_budget_zero` → **PASS**
- `cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute` → **ERROR: cannot find harness**

The harnesses cannot be found because `kani_workflow_budget_harnesses` is **not declared in `lib.rs`**. This is a production code organization issue, not a tooling issue.

### Module Declaration Gap

```rust
// crates/vb_core/src/lib.rs — CURRENT (lines 47-74)
#[cfg(kani)] pub mod kani_step_budget_zero;     // EXISTS
#[cfg(kani)] pub mod kani_step_budget_one;      // EXISTS
#[cfg(kani)] pub mod kani_step_budget;          // EXISTS
// ... 11 other kani modules declared ...

// MISSING (must be added at line ~75):
#[cfg(kani)] pub mod kani_workflow_budget_harnesses;  // KANI-BUDGET-001..005
```

**Impact**: 5 proof obligations cannot be verified. This is **State 10 territory** (production code change required).

---

## Coverage Analysis Update

| Contract Clause | Traced to Obligations | Status | Executable Evidence |
|-----------------|----------------------|--------|---------------------|
| INV-001 | TLA-WF-001, VERUS-BUDGET-001/002, KANI-BUDGET-001 | INCONCLUSIVE / BLOCKED | TLA INCONCLUSIVE; Kani BLOCKED_MISSING_MODULE |
| INV-002 | TLA-WF-002, VERUS-BUDGET-004/005, KANI-BUDGET-003/004 | **PASS** / BLOCKED | TLA PASS; Kani BLOCKED_MISSING_MODULE |
| INV-003 | type_bounds | PASS | Trivially covered |
| INV-004 | VERUS-BUDGET-006, FUZZ-BUDGET-001, PROP-BUDGET-004 | BLOCKED / MISSING | Verus BLOCKED; fuzz NOT_RUN; PROP MISSING |
| INV-005 | TLA-WF-003, KANI-BUDGET-005 | **PASS** / BLOCKED | TLA PASS; Kani BLOCKED_MISSING_MODULE |
| INV-006 | VERUS-BUDGET-003, KANI-BUDGET-002 | BLOCKED / BLOCKED | Verus BLOCKED; Kani BLOCKED_MISSING_MODULE |
| PRE-001 | VERUS-BUDGET-001, KANI-BUDGET-001 | BLOCKED | Kani BLOCKED_MISSING_MODULE |
| PRE-002 | VERUS-BUDGET-001, PROP-BUDGET-001/002/003 | PROPs WAIVED | Partial |
| PRE-003 | type_bounds | PASS | Covered |
| PRE-004 | VERUS-BUDGET-005, KANI-BUDGET-005 | BLOCKED | Kani BLOCKED_MISSING_MODULE |
| PRE-005 | VERUS-BUDGET-006, FUZZ-BUDGET-001 | BLOCKED | Verus BLOCKED; fuzz NOT_RUN |
| POST-001 | VERUS-BUDGET-001/002, KANI-BUDGET-001 | BLOCKED | Kani BLOCKED_MISSING_MODULE |
| POST-002 | VERUS-BUDGET-003, KANI-BUDGET-002 | BLOCKED | Kani BLOCKED_MISSING_MODULE |
| POST-003 | VERUS-BUDGET-004, KANI-BUDGET-003 | BLOCKED | Kani BLOCKED_MISSING_MODULE |
| POST-004 | VERUS-BUDGET-005, KANI-BUDGET-004 | BLOCKED | Kani BLOCKED_MISSING_MODULE |
| POST-005 | type_bounds | PASS | Covered |
| POST-006 | TLA-WF-003, KANI-BUDGET-005 | **PASS** / BLOCKED | TLA PASS; Kani BLOCKED_MISSING_MODULE |
| GAP-001 | WAIVER-GAP-001 | WAIVED | Compensating KANI-BUDGET-002 (BLOCKED) |
| OQ-002 | WAIVER-OQ-002 | WAIVED | Compensating evidence (BLOCKED) |
| OQ-003 | WAIVER-OQ-003 | WAIVED | Compensating evidence (NOT_RUN) |

---

## Resolved Findings

| Finding | Status | Resolution |
|---------|--------|------------|
| LETHAL-1: Kani unexecuted (Attempt 2) | **RESOLVED** | 2 alternate harnesses PASS; 5 blocked by missing module |
| CRITICAL: Workspace path mismatch | **RESOLVED** | Metadata now correctly references velvet-ballistics |
| MAJOR-1: Kani harness incomplete coverage | **UNRESOLVED** | Harness file exists but not compiled |
| MAJOR-2: TLA spec doesn't model computation | **UNRESOLVED** | BLOCKED_SCOPE acknowledged |
| MAJOR-3: Verus namespace mismatch | **UNRESOLVED** | No change |
| TLA-WF-002 contradictory evidence | **RESOLVED** | Confirmed PASS (historical run) |

---

## New Findings

### LETHAL: Missing Module Declaration — Production Change Required

**Severity**: LETHAL (State 10 territory)
**Obligation**: KANI-BUDGET-001..005
**Artifact**: `crates/vb_core/src/lib.rs`

The 5 Kani harnesses targeting budget obligations cannot be compiled because `kani_workflow_budget_harnesses` module is not declared in `lib.rs`.

**Required Action**: Add to `lib.rs`:
```rust
#[cfg(kani)]
pub mod kani_workflow_budget_harnesses;
```

This is a **production code change**. After addition, all 5 harnesses must be executed:
```bash
cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute
cargo kani -p vb_core --harness kani_harness_boundedness_policy_validate
cargo kani -p vb_core --harness kani_harness_try_add_budget_no_overflow
cargo kani -p vb_core --harness kani_harness_fits_within_exact
cargo kani -p vb_core --harness kani_harness_step_budget_consume
```

---

## Waiver Review

### WAIVER-PROP-KERNEL-001
**Target**: `vb_proof_kernels::resource_budget`
**Status**: VALID but compensating evidence not executed in this session

### WAIVER-GAP-001
**Target**: BudgetError missing BLOCK_LOCAL fields
**Status**: VALID but compensating evidence (KANI-BUDGET-002) is BLOCKED_MISSING_MODULE

### WAIVER-OQ-002, WAIVER-OQ-003
**Status**: VALID but compensating evidence is BLOCKED or NOT_RUN

### Missing Waiver: KANI-BUDGET-001..005 Module Declaration
No formal waiver exists for the missing module declaration. A waiver would require:
- Owner and expiry date
- Compensating evidence (2 passing step budget harnesses demonstrate Kani works)
- Risk acceptance rationale for shipping without these 5 obligations

---

## Contract-Verification Gate

| Check | Result | Notes |
|-------|--------|-------|
| All contract clauses have traceability | PASS | |
| TLA+ specs parse | PASS (3/3) | |
| TLA+ model checking | 2 PASS, 1 INCONCLUSIVE | TLA-WF-002, TLA-WF-003 PASS |
| Verus namespace IDs match | FAIL | VERUS-BUDGET-001..006 vs actual IDs |
| Kani harness file exists | PASS | |
| Kani harness module declared | **FAIL** | Not in lib.rs |
| Kani harness executes | **FAIL** (0/5) | 2 other harnesses PASS |
| Proptest waiver valid | PASS | WAIVER-PROP-KERNEL-001 |
| Formal waivers complete | **FAIL** | Missing module declaration waiver |
| Workspace path consistent | PASS | |

---

## Verdict

**Previous STATUS**: REJECTED (Attempt 2/7)
**This Attempt**: TLA-WF-002 contradiction resolved to PASS. Workspace path resolved. **NEW: LETHAL blocker identified — missing module declaration in lib.rs (State 10 production change).**

**Key Remaining Blockers**:
1. **LETHAL**: `kani_workflow_budget_harnesses` not declared in `lib.rs` — 5 proof obligations blocked
2. **CRITICAL**: Misleading BLOCKED_TOOLING label obscures real issue
3. **MAJOR**: VERUS namespace mismatch unresolved
4. **MAJOR**: Compensating evidence for waivers not executed

**Cannot approve** because:
- 5 required proof obligations (KANI-BUDGET-001..005) are blocked by missing production code change
- The BLOCKED_TOOLING label mischaracterizes the issue as tooling vs. code organization
- No formal waiver exists for the missing module declaration

**Path to Approval**:
1. Add `pub mod kani_workflow_budget_harnesses;` to `lib.rs` (production change)
2. Execute all 5 harnesses, record PASS evidence
3. OR issue formal waiver for these obligations with compensating evidence
4. Resolve VERUS namespace mismatch
5. Execute compensating evidence for existing waivers

---

**STATUS: REJECTED**
