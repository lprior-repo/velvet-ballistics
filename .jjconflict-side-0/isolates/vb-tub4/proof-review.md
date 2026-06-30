# Proof Review: vb-tub4

## Reviewer Information
- **reviewer_skill**: proof-reviewer
- **review_state**: 6
- **reviewed_artifacts**: proof-writer-report.md, trusted-base-ledger.jsonl, verifier-lane-review.jsonl, proof-plan-findings.jsonl, proof-plan-review.md, proof-plan-repair-guide.md, modified source files (budget.rs, frame.rs, kani_idempotency_gates.rs)

## STATUS: REJECTED

---

## Summary

The proof-writer executed 17/29 obligations (2 deletions, 6 structural fixes, 3 confirmed passes). However, **this review cannot approve because critical prerequisite artifacts are missing and blocking issues remain unresolved**. The proof plan was REJECTED at State 4 with 3 critical blockers that were never resolved before advancing to State 6.

---

## Findings

### CRITICAL — Blocker 1: `proof-obligations.planned.jsonl` Is Entirely Absent

- **Artifact**: `proof-obligations.planned.jsonl` (MISSING)
- **Finding Code**: `E_ARTIFACT_MISSING`
- **Severity**: blocker
- **Message**: The canonical obligations file does not exist in the isolate. The proof-plan-review.md (State 4) was REJECTED with 3 critical blockers: missing `schema_version`, missing `workdir`, and wrong field name `bound` instead of `model_bounds`. The proof-plan-repair-guide.md specifies how to fix these issues, but the file was never created/fixed. Execution proceeded to State 6 without an approved obligations file.
- **Evidence**: `rtk find /home/lewis/src/velvet-ballistics/isolates/vb-tub4 -name "*obligations*" -type f` returns no matches. proof-plan-review.md line 8 shows `STATUS: REJECTED`.
- **Required Fix**: Create `proof-obligations.planned.jsonl` with schema-compliant obligations per the repair guide, then re-run proof-plan-reviewer to advance to State 5.

### CRITICAL — Blocker 2: No Provenance Ledger (`agent-invocation-ledger.jsonl`)

- **Artifact**: `agent-invocation-ledger.jsonl` (MISSING)
- **Finding Code**: `E_NO_PROVENANCE`
- **Severity**: blocker
- **Message**: Cannot verify reviewer/approval provenance. No ledger records which agent approved advancement from State 4 to State 5.
- **Evidence**: Not found in isolate directory.
- **Required Fix**: Provide `agent-invocation-ledger.jsonl` or equivalent provenance record before this review can approve.

### CRITICAL — Blocker 3: State Machine Discrepancy — Unresolved

- **Artifact**: `crates/vb_proof_kernels/src/step_state.rs` line 48 vs. reference model
- **Finding Code**: `E_STATE_MACHINE_MISMATCH`
- **Severity**: blocker
- **Harnesses Affected**: `validate_transition_terminal_blocks_all` (K-F5), `validate_transition_exhaustive_64`
- **Message**: The vb_proof_kernels state machine explicitly allows `(StepState::Succeeded, StepState::Pending)` transition (line 48). The reference model does NOT include this transition. The harnesses assert terminal states block all non-self transitions. This is not a tooling issue — it is a spec discrepancy requiring a product decision.
- **Evidence**:
  - `cargo kani -p vb_core --harness validate_transition_terminal_blocks_all` → FAILED: "terminal->other blocked" assertion at frame.rs:1929
  - `cargo kani -p vb_core --harness validate_transition_exhaustive_64` → FAILED: "X->P!" assertion at frame.rs:1478
  - vb_proof_kernels/step_state.rs line 48: `(StepState::Succeeded, StepState::Pending)` exists
  - reference/src/step_state_model.rs lines 58-61: does NOT have Succeeded->Pending transition
- **Required Fix**: Either (a) update the harness assertions to match the proof kernel's Succeeded->Pending allowance, OR (b) file a separate bead to change the proof kernel's VALID_TRANSITIONS to remove Succeeded->Pending. The current state is unsatisfiable: both harness and implementation cannot be simultaneously correct.

### CRITICAL — Blocker 4: Timeout Blockers Lack Waiver or Resolution Path

- **Artifact**: `crates/vb_core/src/frame.rs` harnesses K-S1, K-S2
- **Finding Code**: `E_TIMEOUT_BLOCKER`
- **Severity**: blocker
- **Harnesses Affected**: `read_slot_no_panic`, `write_slot_no_panic`
- **Message**: Symbolic u16 state space with bound 1..=16 times slot_raw<slot_count creates 256 combinations that Kani cannot explore within 180s timeout. Proof-writer report recommends reducing to concrete values but no waiver or formal resolution path exists.
- **Evidence**: `cargo kani -p vb_core --harness read_slot_no_panic` → TIMEOUT (>180s); `cargo kani -p vb_core --harness write_slot_no_panic` → TIMEOUT (>180s)
- **Required Fix**: Either (a) use concrete slot_count=4 values as the proof strategy with explicit justification that this is sufficient coverage, OR (b) file a formal waiver with compensating evidence (e.g., unit tests, integration tests) for the unbounded symbolic case.

### MINOR — Advisory 1: Pre-existing Failure Not Linked to Downstream Waiver

- **Artifact**: `validate_transition_exhaustive_64` (pre-existing failure)
- **Finding Code**: `E_PREEXISTING_NOT_WAIVED`
- **Severity**: advisory
- **Message**: The pre-existing failure at `validate_transition_exhaustive_64` is the same state machine discrepancy as K-F5. No formal waiver exists in `proof-obligations.planned.jsonl` linking this to a resolution bead.
- **Required Fix**: File formal waiver or link to resolution bead in obligations ledger.

---

## GOD RULE Compliance Check

### ✅ KANI-001 (No hardcoded harness inputs): FIXED for 17/29 obligations
- K-B1 (`add_dim_no_panic`): Uses `kani::any()` with `kani::assume(current <= u64::MAX/2 && requested <= u64::MAX/2)`. VERIFICATION:- SUCCESSFUL confirmed.
- K-B2 (`sub_dim_no_panic`): Uses `kani::any()` with `kani::assume(requested <= current)`. VERIFICATION:- SUCCESSFUL confirmed.
- K-F4 (`validate_transition_running_to_all_valid_targets`): Uses `kani::any()` for target state. VERIFICATION:- SUCCESSFUL confirmed.
- KANI-RUNTIME-004, KANI-RUNTIME-005: Placeholder harnesses DELETED confirmed.

### ✅ No `by(compute)` found (correct tool usage — Kani, not Verus)
- Confirmed via grep across isolate and vb_core src directories.

### ✅ Trust markers recorded in `trusted-base-ledger.jsonl`
- 12 trust marker entries present documenting assumption bounds.

### ❌ Blockers lack formal waiver/disposition
- State machine discrepancy, timeout blockers, and pre-existing failure are documented in proof-writer-report.md but NOT formally waived in `proof-obligations.planned.jsonl` (which doesn't exist).

---

## Traceability

| Obligation | Status | Evidence |
|------------|--------|----------|
| K-B1 (add_dim_no_panic) | PASS | cargo kani exit 0, 0 of 14 failed |
| K-B2 (sub_dim_no_panic) | PASS | cargo kani exit 0, 0 of 10 failed |
| K-F4 (validate_transition_running_to_all_valid_targets) | PASS | cargo kani exit 0, 0 of 99 failed |
| K-F5 (validate_transition_terminal_blocks_all) | FAIL | State machine discrepancy (Succeeded->Pending allowed) |
| K-S1 (read_slot_no_panic) | TIMEOUT | Symbolic u16 state space too large |
| K-S2 (write_slot_no_panic) | TIMEOUT | Symbolic u16 state space too large |
| validate_transition_exhaustive_64 | FAIL (pre-existing) | Same state machine discrepancy |
| KANI-RUNTIME-004 | DELETED | Confirmed removed |
| KANI-RUNTIME-005 | DELETED | Confirmed removed |

---

## Verdict

**REJECTED** — Cannot approve because:

1. **`proof-obligations.planned.jsonl` does not exist** — The proof-plan was rejected at State 4 and the required fixes were never applied before advancing to State 6. Without an approved obligations file, there is no machine-readable record of what was supposed to be executed or its current disposition.

2. **No provenance ledger** — Cannot verify that State 4→5 advancement was properly approved.

3. **State machine discrepancy is a hard blocker** — vb_proof_kernels allows Succeeded→Pending but harnesses expect it blocked. This is not a tooling issue; it is a spec conflict that requires a product decision and either harness repair or kernel repair.

4. **Timeout blockers lack resolution path** — read_slot_no_panic and write_slot_no_panic cannot be verified with current symbolic bounds. A concrete fallback strategy or formal waiver is required.

**Required State**: Return to State 4 with `proof-obligations.planned.jsonl` properly schema-compliant and re-reviewed. The 17/29 obligations that were correctly fixed may be re-presented in a schema-compliant obligations file.

---

## Recommendations for Advancement

1. **For state machine discrepancy**: Create a separate bead to resolve Succeeded→Pending spec conflict. Meanwhile, update K-F5 harness to assert the actual machine behavior, OR remove Succeeded→Pending from vb_proof_kernels VALID_TRANSITIONS if it is indeed a bug.

2. **For timeout blockers**: Use concrete slot_count=4 with explicit justification that this covers the boundary conditions, or provide unit test evidence as compensating verification.

3. **For proof-obligations.planned.jsonl**: Apply all fixes from proof-plan-repair-guide.md, ensure file exists, and re-run proof-plan-reviewer before advancing to State 5.
