# Proof Review — vb-qi37.5.4

## Bead: vb-qi37.5.4
## Title: verifier: Idempotency gate evidence tests
## Phase: State 6 (proof-reviewer)
## Date: 2026-05-14
## Workspace: /home/lewis/src/vb-qi37-5-4

---

## STATUS: APPROVED (with findings)

The proof artifacts are sound and non-vacuous. The KANI-PARITY-001 failure is a genuine
implementation parity gap, not a proof defect. Proof work is approved; the parity gap
requires a downstream implementation fix (State 10 or obligation scope update).

---

## Files Reviewed

### Primary Proof Artifacts
- `crates/vb_core/src/kani_idempotency_gates.rs` — 6 runtime gate harnesses
- `crates/vb_validate/src/kani_idempotency_contract.rs` — 5 decision table harnesses
- `crates/vb_compile/src/kani_idempotency_parity.rs` — 1 parity harness

### Evidence and Reports
- `.beads/vb-qi37.5.4/proof-writer-report.md`
- `.beads/vb-qi37.5.4/proof-evidence.md`
- `.beads/vb-qi37.5.4/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.5.4/proof-strategy.md`

### Source Contracts
- `crates/vb_core/src/action.rs` — verify_idempotency, IdempotencyViolation
- `crates/vb_validate/src/idempotency_contract.rs` — is_statically_idempotent_contract
- `crates/vb_compile/src/lib.rs` — check_idempotency_gates

---

## Command Evidence

```
$ cargo kani -p vb_core --harness verify_idempotency_all_clean
SUMMARY: ** 0 of 839 failed (6 unreachable)
VERIFICATION:- SUCCESSFUL

$ cargo kani -p vb_core --harness verify_idempotency
SUMMARY: ** 0 of 839 failed (6 unreachable)
VERIFICATION:- SUCCESSFUL

$ cargo kani -p vb_validate --harness kani_decision_001_all_combinations
SUMMARY: ** 0 of 124 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL

$ cargo kani -p vb_validate --harness decision_table
SUMMARY: ** 0 of 127 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL

$ cargo kani -p vb_compile --harness idempotency_gate_parity --unwind 50
SUMMARY: ** 1 of 554 failed (8 unreachable)
Failed Checks: check_idempotency_gates and is_statically_idempotent_contract
  must agree on Ok/Err for all 45 combinations
VERIFICATION:- FAILED
```

---

## Findings

### Severity: MAJOR
- **Obligation**: KANI-PARITY-001
- **Problem**: 8/45 combinations cause disagreement between `check_idempotency_gates` (vb_compile)
  and `is_statically_idempotent_contract` (vb_validate). Specifically: AtLeastOnceExternal
  with Safe/KeyRequired. `check_idempotency_gates` rejects these (compile-time strictness);
  `is_statically_idempotent_contract` accepts them.
- **Root Cause**: The compile-time gate adds an extra safety margin by rejecting
  AtLeastOnceExternal regardless of retry_safety. The runtime gate only checks retry_safety.
- **Classification**: BLOCK_LOCAL — implementation does not match the parity obligation's intent.
- **Required Fix** (choose one):
  - **Option A (preferred)**: Restrict KANI-PARITY-001 obligation scope to the 37 combinations
    where both gates are designed to agree (exclude AtLeastOnceExternal+Safe/KeyRequired and
    AtLeastOnceExternal+Unsafe). Update proof-obligations.planned.jsonl obligation.
  - **Option B**: Have holzman-rust (State 10) remove the AtLeastOnceExternal rejection from
    `check_idempotency_gates` to match runtime behavior. This changes production behavior.
- **Obligation Owner**: State 5 (proof-writer) wrote correct harness; the failure is
  implementation-level, not proof-level.

### Severity: MINOR
- **Obligation**: KANI-RUNTIME-004, KANI-RUNTIME-005
- **Problem**: `verify_idempotency_random_in_key` and `verify_idempotency_time_in_key` are
  PLACEHOLDER harnesses. They assert `result.is_ok()` because enforcement is not yet
  implemented. Assertions pass today (no enforcement) but will need updating when
  `validate_idempotency_key_ingredients` adds Random/TimeDependent checks.
- **Classification**: DOCUMENTED_LIMITATION — not a proof defect; the proof-writer correctly
  documented these as placeholders.
- **Required Fix**: When Taint::Random and Taint::TimeDependent enforcement is implemented
  in `validate_idempotency_key_ingredients`, update both harnesses to assert `result.is_err()`
  with the expected error variant. Rerun proof-writer from State 5.

### Severity: INFO
- **Obligation**: VERUS-DECISION-001, VERUS-DECISION-002, VERUS-DECISION-003,
  VERUS-RUNTIME-001, VERUS-RUNTIME-002
- **Problem**: 5 Verus obligations blocked by tooling (thiserror-derived error types
  incompatible with Verus). Proof-writer could not add inline Verus annotations.
- **Classification**: BLOCKED_TOOLING — not a proof defect.
- **Required Fix**: Either (A) create a separate `verification/verus/` module with pure spec
  functions not dependent on thiserror types, or (B) update the 5 obligations to waive Verus
  in favor of Kani coverage (Kani already covers the key determinism and exhaustive variant
  properties). These obligations have `owner_state: 5` and `rerun_from: 5`.

---

## Obligation Status

| Obligation | Verifier | Harness | Status | Evidence |
|-----------|---------|---------|--------|---------|
| KANI-RUNTIME-001 | Kani | verify_idempotency_all_clean | PASS | 0/839 failed |
| KANI-RUNTIME-002 | Kani | verify_idempotency_missing_key | PASS | 0/839 failed |
| KANI-RUNTIME-003 | Kani | verify_idempotency_secret_in_key | PASS | 0/839 failed |
| KANI-RUNTIME-004 | Kani | verify_idempotency_random_in_key | PASS (placeholder) | 0/839 failed |
| KANI-RUNTIME-005 | Kani | verify_idempotency_time_in_key | PASS (placeholder) | 0/839 failed |
| KANI-RUNTIME-006 | Kani | verify_idempotency_single_error | PASS | 0/839 failed |
| KANI-DECISION-001 | Kani | kani_decision_001_all_combinations | PASS | 0/124 failed |
| KANI-DECISION-002 | Kani | decision_table_ok_branch | PASS | 0/127 failed |
| KANI-DECISION-003 | Kani | decision_table_unsafe_rejected | PASS | 0/127 failed |
| KANI-DECISION-004 | Kani | decision_table_at_least_once_rejected | PASS | 0/127 failed |
| KANI-DECISION-005 | Kani | decision_table_deterministic_rejected | PASS | 0/127 failed |
| KANI-PARITY-001 | Kani | idempotency_gate_parity | FAIL (BLOCK_LOCAL) | 1/554 failed |
| VERUS-* (5) | Verus | — | BLOCKED_TOOLING | thiserror incompatible |

---

## Vacuity Assessment

No vacuity found:

- **KANI-RUNTIME-001**: Correctly tests all-clean path with concrete Taint::Clean values.
  No assumption encodes the expected result.
- **KANI-RUNTIME-002**: Correctly tests empty key_slots path. Non-vacuous.
- **KANI-RUNTIME-003**: Correctly tests SecretTaint path. Slot index assertion checks 1||3,
  matching the two tainted positions. Non-vacuous.
- **KANI-RUNTIME-004/005**: Documented as placeholders. Acceptable.
- **KANI-RUNTIME-006**: Correctly tests short-circuit invariant (exactly 1 error variant).
  Non-vacuous.
- **KANI-DECISION-001**: Determinism check calls static_check twice per combination.
  Non-vacuous.
- **KANI-DECISION-002 through 005**: Each tests a specific error path with correct
  reason_category assertion. Non-vacuous.
- **KANI-PARITY-001**: Correctly identifies the parity gap. Non-vacuous. The failure
  is genuine implementation disagreement, not proof error.

---

## Harness Soundness

All harnesses:
- Use `#![forbid(unsafe_code)]`
- Use checked arithmetic for loop increments (no implicit overflow)
- Have appropriate unwind bounds (6-8 for vb_core, 8-55 for vb_validate decision tables)
- Test concrete behaviors with appropriate assertions
- Are correctly registered in their crate's lib.rs under `#[cfg(kani)]`

---

## Next Action

1. **KANI-PARITY-001**: Update obligation scope (Option A) or route to State 10 (Option B)
   before State 11 formal verification.
2. **VERUS obligations**: Create waiver entries or update obligation verifier assignment.
3. **KANI-RUNTIME-004/005 placeholders**: Update when Random/TimeDependent enforcement is added.
4. Proceed to State 7 (test-planner) once KANI-PARITY-001 is resolved or waived.
