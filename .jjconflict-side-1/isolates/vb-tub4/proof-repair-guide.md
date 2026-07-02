# Proof Repair Guide: vb-tub4

## Purpose

This guide documents required repairs to advance vb-tub4 from State 6 rejection to approval. All items below are blockers that must be resolved before the proof can be approved.

---

## Repair 1: Create `proof-obligations.planned.jsonl` with Schema-Compliant Obligations

**Current State**: The obligations file does not exist. The proof-plan-review.md (State 4) was REJECTED with 3 critical blockers.

**Required State**: A schema-compliant `proof-obligations.planned.jsonl` exists with all required fields.

**Steps**:
1. Create `proof-obligations.planned.jsonl` based on the 29 obligations referenced in proof-writer-report.md
2. Add `"schema_version": "proof-obligation/v1"` to every entry
3. Add `"workdir": "/home/lewis/src/velvet-ballistics"` to every entry
4. Rename any `"bound"` field to `"model_bounds"`
5. Add `"required": true` and `"behavior_affecting": false` to each obligation
6. Add `"artifact"` field pointing to the harness source file
7. Run `proof-plan-reviewer` to validate schema compliance
8. Advance to State 5 only after approval

---

## Repair 2: Provide `agent-invocation-ledger.jsonl`

**Current State**: No provenance ledger exists to verify State 4→5 advancement was approved.

**Required State**: A ledger recording invocation IDs, approval decisions, and timestamps for all reviewer agents.

**Steps**:
1. Create `agent-invocation-ledger.jsonl` with entries for each reviewer invocation
2. Each entry must include: `invocation_id`, `reviewer_skill`, `review_state`, `decision`, `timestamp`
3. Ensure chain of custody from State 4 rejection through State 5 re-approval is documented

---

## Repair 3: Resolve State Machine Discrepancy (Succeeded→Pending)

**Current State**: vb_proof_kernels/step_state.rs line 48 allows `(StepState::Succeeded, StepState::Pending)`. The harness `validate_transition_terminal_blocks_all` expects terminal states to block all non-self transitions.

**Required State**: Either the kernel or the harness must be corrected to eliminate the inconsistency.

**Option A (Recommended)**: Create a separate bead (e.g., vb-tub5) to:
1. Review whether Succeeded→Pending transition is intentional design or bug
2. If bug: Remove `(StepState::Succeeded, StepState::Pending)` from vb_proof_kernels/src/step_state.rs line 48
3. If intentional: Update K-F5 harness to assert the actual machine behavior

**Option B**: Update K-F5 harness to match current machine behavior:
```rust
// Change assertion at frame.rs:1929
// FROM:
kani::assert(!result, "terminal->other blocked");
// TO:
if terminal == StepState::Succeeded && target == StepState::Pending {
    // Succeeded->Pending is allowed by proof kernel
    kani::assert(result, "Succeeded->Pending is allowed");
} else if terminal != target {
    kani::assert(!result, "terminal->other blocked");
}
```

**Status**: This is a product/design decision requiring bead owner input. Cannot resolve unilaterally.

---

## Repair 4: Resolve Timeout Blockers for K-S1 and K-S2

**Current State**: `read_slot_no_panic` and `write_slot_no_panic` timeout with symbolic u16 slot_count bound 1..=16.

**Required State**: Evidence of non-panic behavior for slot operations.

**Option A**: Use concrete slot_count values:
```rust
// Replace symbolic slot_count with concrete 4
let slot_count: u16 = 4;  // concrete value
kani::assume(slot_count >= 1);  // still need lower bound for constructor
let slot_raw: u16 = kani::any();
kani::assume(slot_raw < slot_count);  // 0..=3
```

**Option B**: File formal waiver with compensating evidence:
```json
{
  "harness": "read_slot_no_panic",
  "status": "WAIVED_TIMEOUT",
  "waiver_reason": "symbolic u16 state space too large for Kani",
  "compensating_evidence": [
    "unit tests for SlotIdx bounds checking",
    "integration tests for slot read/write operations"
  ],
  "owner": "Lewis",
  "expiry": "before claiming slot_count > 16 coverage"
}
```

---

## Repair 5: Link Pre-existing Failure to Resolution Bead

**Current State**: `validate_transition_exhaustive_64` failure is documented as pre-existing but has no formal waiver.

**Required State**: Either file a waiver in `proof-obligations.planned.jsonl` or link to a resolution bead.

**Steps**:
1. Add obligation entry for `validate_transition_exhaustive_64`
2. Mark status as `BLOCKED_SPEC_DISCREPANCY`
3. Link to resolution bead (same as Repair 3)
4. Provide compensating evidence citation

---

## Execution Order

1. **First**: Complete Repair 1 (create obligations file) — required before any reviewer can evaluate
2. **Second**: Complete Repair 2 (provenance ledger) — required for approval chain
3. **Third**: Address Repair 3 (state machine) — requires product decision
4. **Fourth**: Complete Repair 4 (timeout) or provide waivers
5. **Fifth**: Complete Repair 5 (link pre-existing failure)
6. **Sixth**: Re-run proof-plan-reviewer (State 4 equivalent)
7. **Seventh**: Re-run proof-reviewer (State 6)

---

## Non-Blocking Items

The following items were correctly handled and do NOT require repair:
- ✅ K-B1 (add_dim_no_panic): Uses `kani::any()` with proper bounds, passes verification
- ✅ K-B2 (sub_dim_no_panic): Uses `kani::any()` with proper bounds, passes verification
- ✅ K-F4 (validate_transition_running_to_all_valid_targets): Uses `kani::any()` for target, passes verification
- ✅ KANI-RUNTIME-004, KANI-RUNTIME-005: Placeholder harnesses correctly deleted
- ✅ No `by(compute)` usage — correct tool (Kani) used throughout
- ✅ Trust markers documented in `trusted-base-ledger.jsonl`
