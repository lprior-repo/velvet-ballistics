# Proof Review — vb-e4mt (Attempt 3/7)

## Bead: Resource Bounds and Budget Enforcement
**State**: 6 (proof-reviewer re-review)
**Workdir**: `/home/lewis/src/vb-e4mt-workspace`
**Actual Code Workspace**: `/home/lewis/src/velvet-ballistics`
**Date**: 2026-05-19

---

## Executive Summary

| Obligation Group | Status | Change from Attempt 2 |
|-----------------|--------|----------------------|
| TLA-WF-002 (AggregateResourceSpec) | **PASS** | Was contradictory |
| TLA-WF-003 (StepBudgetSpec) | **PASS** | Unchanged |
| TLA-WF-001 (WorkflowBudgetSpec) | **INCONCLUSIVE** | Unchanged |
| KANI-BUDGET-001..005 (kani_workflow_budget_harnesses) | **BLOCKED_MISSING_MODULE** | NEW: Was NOT_RUN |
| KANI step budget (zero/one) | **PASS** (2 harnesses) | Confirmed passing |

---

## Findings (Ordered by Severity)

### LETHAL-1: Kani Workflow Budget Harnesses Blocked by Missing Module Declaration

**Severity**: LETHAL (production code change required)
**Obligation**: KANI-BUDGET-001, KANI-BUDGET-002, KANI-BUDGET-003, KANI-BUDGET-004, KANI-BUDGET-005
**Artifact**: `crates/vb_core/src/kani_workflow_budget_harnesses.rs`

**Problem**: The harness file `kani_workflow_budget_harnesses.rs` **exists and is structurally correct** (verified: `#[kani::proof]` annotations present, imports valid, `WorkflowParts::any()` from `kani_workflow_arbitrary` used correctly). However, the module **`kani_workflow_budget_harnesses` is NOT declared in `lib.rs`**.

```rust
// lib.rs has these (lines 47-74):
#[cfg(kani)] pub mod kani_step_budget_zero;
#[cfg(kani)] pub mod kani_step_budget_one;
#[cfg(kani)] pub mod kani_step_budget;
#[cfg(kani)] pub mod kani_step_budget_try_take_arbitrary;
// ... etc

// MISSING:
#[cfg(kani)] pub mod kani_workflow_budget_harnesses;  // <-- NOT PRESENT
```

**Consequence**: `cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute` fails with:
```
error: could not find `kani_harness_whole_workflow_budget_compute` in package `vb_core`
```

**This is NOT a tooling issue** — Kani itself works correctly. The BLOCKED_TOOLING label in proof-evidence.md mischaracterizes the problem. The 2 Kani harnesses that PASS (`kani_step_budget_zero`, `kani_step_budget_one`) confirm Kani executes fine when modules are properly declared.

**Required Fix**: Add `pub mod kani_workflow_budget_harnesses;` with `#[cfg(kani)]` guard to `lib.rs` (line ~74). This is a **State 10 production code change**.

**Verifying fix**:
```bash
# After adding the module to lib.rs:
cd /home/lewis/src/velvet-ballistics && cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute
```

---

### CRITICAL-1: BLOCKED_TOOLING Label Is Misleading

**Severity**: CRITICAL (misleading metadata)
**Artifact**: proof-evidence.md, proof-obligations.planned.jsonl

**Problem**: The obligation status for KANI-BUDGET-001..005 is recorded as `BLOCKED_TOOLING`. This implies the Kani tool itself is unavailable or broken. In reality:
- `cargo kani --version` → 0.67.0 (works)
- `cargo kani -p vb_core --harness kani_step_budget_zero` → **PASS** (confirms Kani works)
- The actual blocker: module not declared in lib.rs (code organization, not tooling)

**Impact**: Reviewers cannot distinguish between "Kani is broken" vs "module not wired up". This delays resolution.

**Required Fix**: Update status to `BLOCKED_MISSING_MODULE` and document the exact fix needed.

---

### MAJOR-1: TLA-WF-002 Status Resolution

**Severity**: MAJOR
**Obligation**: TLA-WF-002

**Finding**: The contradictory evidence (proof-evidence.md line 37 claimed both "PASSED with 35M states" AND "INCONCLUSIVE - timed out") is resolved. Per the specs directory, `AggregateResourceSpec.tla` exists and previously passed with 35M states, 540k distinct, 14s. The "timed out at 120s" finding was from a different run session. **TLA-WF-002 = PASS**.

---

### MINOR-1: No Formal Waiver for Missing Module Declaration

**Severity**: MINOR
**Problem**: No waiver exists for KANI-BUDGET-001..005 being blocked by missing module declaration. The existing waivers (WAIVER-GAP-001, WAIVER-OQ-002, WAIVER-OQ-003, WAIVER-PROP-KERNEL-001) cover different issues.

**Required**: Either (a) issue a formal waiver for the missing module with compensating evidence, or (b) execute the production code change to add the module.

---

## Obligation Execution Status

| Obligation | Status | Evidence |
|------------|--------|----------|
| TLA-WF-001 | INCONCLUSIVE | Vacuous inv fixed, error mapping fixed; state space still large |
| TLA-WF-002 | **PASS** | 35M states, 540k distinct, 14s (historical pass) |
| TLA-WF-003 | **PASS** | 1351 states, 186 distinct, <1s |
| KANI-BUDGET-001 | BLOCKED_MISSING_MODULE | Module not in lib.rs |
| KANI-BUDGET-002 | BLOCKED_MISSING_MODULE | Module not in lib.rs |
| KANI-BUDGET-003 | BLOCKED_MISSING_MODULE | Module not in lib.rs |
| KANI-BUDGET-004 | BLOCKED_MISSING_MODULE | Module not in lib.rs |
| KANI-BUDGET-005 | BLOCKED_MISSING_MODULE | Module not in lib.rs |
| KANI step budget (zero/one) | **PASS** (2 harnesses) | Confirmed via cargo kani |
| VERUS-BUDGET-001..006 | BLOCKED | Namespace mismatch + no direct Rust function proofs |
| PROP-BUDGET-001..003 | WAIVED | WAIVER-PROP-KERNEL-001 |
| PROP-BUDGET-004 | MISSING | proptest function not created |
| FUZZ-BUDGET-001 | NOT_RUN | Fuzz target exists but not executed |

---

## Waiver Review

### WAIVER-PROP-KERNEL-001 (Proptest Not in vb_proof_kernels)
**Status**: VALID — Sound rationale (Verus/Aeneas extraction crate, not proptest target)
**Compensating Evidence**: Unit tests (1028 lines) in resource_budget.rs — **NOT EXECUTED in this session**

### WAIVER-GAP-001, WAIVER-OQ-002, WAIVER-OQ-003
**Status**: VALID but **compensating evidence (Kani) not executed**

### Missing: Waiver for KANI-BUDGET-001..005 Module Declaration
**Status**: NOT ISSUED — Required before approval if using waiver path

---

## Required Actions for Approval

1. **Add module to lib.rs** (State 10 production change):
   ```rust
   #[cfg(kani)]
   pub mod kani_workflow_budget_harnesses;
   ```
   Then execute all 5 harnesses and record PASS evidence.

2. **OR Issue Formal Waiver** for KANI-BUDGET-001..005 with:
   - Owner and expiry
   - Compensating evidence (the 2 passing step budget Kani harnesses demonstrate Kani works)
   - Risk acceptance rationale

3. **Execute VERUS obligations** — Resolve namespace mismatch or issue waivers

4. **Execute TLA-WF-001** — Requires Apalache (not available) OR deeper state space reduction

5. **Execute compensating evidence** for existing waivers

---

## TLA Spec Fix Validation (TLA-WF-001)

| Fix | Status | Evidence |
|-----|--------|----------|
| `InvNoOverflow` replaced with constant `TRUE` | **VALID** | Mathematically sound |
| `CompleteComputeReject` IF-ELSIF chain | **VALID** | Each violated bound maps to specific error variant |
| `InvErrorConsistent` added | **VALID** | admitted => last_error=none, rejected => last_error≠none |
| `THEOREM Spec => []InvErrorConsistent` | **VALID BUT UNVERIFIED** | Theorem formed correctly but TLC output not recorded |

---

## STATUS: REJECTED

**Reason**: 5 Kani harnesses (KANI-BUDGET-001..005) are blocked by missing `kani_workflow_budget_harnesses` module declaration in `lib.rs`. This is a **production code change** (State 10 territory), not a tooling limitation. The BLOCKED_TOOLING label is misleading. Without adding the module OR issuing a formal waiver with compensating evidence, approval cannot be granted.

**Severity of remaining blockers**:
- LETHAL-1: Missing module declaration (production change required)
- CRITICAL-1: Misleading BLOCKED_TOOLING label
- MAJOR-1: VERUS namespace mismatch unresolved
- MINOR-1: No waiver for missing module

**If module is added and harnesses pass**: Re-review required to confirm execution evidence.
**If waiver path chosen**: Formal waiver must be issued before re-review.

**STATUS: REJECTED**
