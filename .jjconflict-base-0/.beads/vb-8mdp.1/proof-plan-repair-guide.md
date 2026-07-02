# Proof Plan Repair Guide — vb-8mdp.1

## Status: REJECTED — 3 blocking findings require repair

---

## Smallest State to Rerun

The proof planner should resume from the following state with minimal rework:

**Resume from**: After `verifier-lane-decisions.jsonl` has been updated with the missing lane decisions.

**Do NOT rerun**: The entire proof-planner from scratch. Only the following files need changes:

1. `artifacts/verifier-lane-decisions.jsonl` — add 2 rows (VB-IPC-FRAME-003, VB-IPC-RESPONSE-001)
2. `artifacts/proof-plan-review.md` — update scope section to explicitly list all 28 seeds

---

## Required Changes

### 1. Add VB-IPC-FRAME-003 Lane Decision (BLOCKER)

**File**: `artifacts/verifier-lane-decisions.jsonl`

Add a new row:

```json
{"verifier":"kani","proof_seed_id":"VB-IPC-FRAME-003","requirement_id":"VB-IPC-REQ-???","lane_decision":"required","artifacts":["crates/vb_ipc/src/frame.rs or relevant file"],"new_harnesses":["kani_harness_validate_frame_magic_zero_alloc"],"command":"cd /home/lewis/src/velvet-ballistics && cargo kani -p vb_ipc --crate-type=lib 2>&1 | tee artifacts/kani-validate-magic.log","expected_evidence":"kani proves validate_frame_magic returns HeaderDecodeFailed/InvalidMagic without any Vec allocation; no panics for all byte sequences","bounds":"kani::any() on byte sequences of length 0..23","assumptions":["byteorder reads never panic on fixed-size slices","no Vec allocation in validate_frame_magic"],"status":"new"}
```

OR, if Kani is not appropriate for this claim, add:

```json
{"verifier":"code-review","proof_seed_id":"VB-IPC-FRAME-003","requirement_id":"VB-IPC-REQ-???","lane_decision":"required","artifacts":["crates/vb_ipc/src/frame.rs"],"new_harnesses":[],"command":"N/A — code review artifact","expected_evidence":"Code review of validate_frame_magic proves no Vec allocation for any input","bounds":"N/A","assumptions":["validate_frame_magic is a pure read-only function","no side effects"],"status":"existing"}
```

**Requirement ID**: The traceability-matrix.jsonl maps VB-IPC-REQ-020 to VB-IPC-FRAME-003... wait, VB-IPC-REQ-020 maps to VB-IPC-DECODE-001. Check the traceability matrix for the correct REQ ID for validate_frame_magic.

Actually, looking at the traceability-matrix.jsonl, I don't see a REQ that maps to VB-IPC-FRAME-003 explicitly. The proof seed exists but may have been added after the traceability matrix was last updated. The planner should identify the correct requirement ID from the contract or create a new VB-IPC-REQ for this seed.

### 2. Add VB-IPC-RESPONSE-001 Lane Decision (BLOCKER)

**File**: `artifacts/verifier-lane-decisions.jsonl`

Add a new row:

```json
{"verifier":"code-review","proof_seed_id":"VB-IPC-RESPONSE-001","requirement_id":"VB-IPC-REQ-???","lane_decision":"required","artifacts":["crates/vb_ipc/src/server/*.rs (frame_error_response)"],"new_harnesses":[],"command":"N/A — code review artifact","expected_evidence":"Code review confirms frame_error_response creates response with command=Health, correlation=0, payload_len=0","bounds":"N/A","assumptions":["frame_error_response is the only error response constructor"],"status":"existing"}
```

OR, if a test is more appropriate:

```json
{"verifier":"proptest","proof_seed_id":"VB-IPC-RESPONSE-001","requirement_id":"VB-IPC-REQ-???","lane_decision":"required","artifacts":["crates/vb_ipc/src/tests.rs or impl_tests.rs"],"new_harnesses":["proptest_error_response_uses_health_command"],"command":"cd /home/lewis/src/velvet-ballistics && cargo test -p vb_ipc --release -- error_response_health_command 2>&1 | tee artifacts/test-error-response.log","expected_evidence":"test passes: all error responses have command=Health, correlation=0, payload_len=0","bounds":"all IpcError variants","assumptions":["frame_error_response is tested for all error paths"],"status":"new"}
```

**Requirement ID**: The traceability-matrix.jsonl maps requirements to seeds. The planner should identify the correct VB-IPC-REQ for VB-IPC-RESPONSE-001 from the contract or create one.

### 3. Add Scope Coverage Table (MAJOR)

**File**: `artifacts/proof-plan-review.md`

Replace the "Reviewer Checklist" and "Blockers" section with an explicit coverage table for all 28 proof seeds:

```
## Coverage of All 28 Proof Seeds

| Proof Seed | Coverage Status | Artifact |
|------------|----------------|----------|
| VB-IPC-DECODE-001 | new (Kani+Verus+Proptest) | proof-obligations.planned.jsonl |
| VB-IPC-DECODE-002 | existing | [existing artifact] |
| VB-IPC-DECODE-003 | new (Kani+Verus) | proof-obligations.planned.jsonl |
| VB-IPC-DECODE-004 | new (Kani+Verus) | proof-obligations.planned.jsonl |
| VB-IPC-DECODE-005 | existing | [existing artifact] |
| VB-IPC-DECODE-006 | existing | [existing artifact] |
| VB-IPC-DECODE-007 | existing | [existing artifact] |
| VB-IPC-POSTCARD-001 | existing | [existing artifact] |
| VB-IPC-POSTCARD-002 | existing | [existing artifact] |
| VB-IPC-BOUNDED-001 | existing | [existing artifact] |
| VB-IPC-BOUNDED-002 | existing | [existing artifact] |
| VB-IPC-MAGIC-001 | existing | [existing artifact] |
| VB-IPC-MAGIC-002 | existing | [existing artifact] |
| VB-IPC-MAGIC-003 | existing | [existing artifact] |
| VB-IPC-VERSION-001 | existing | [existing artifact] |
| VB-IPC-COMMAND-001 | existing | [existing artifact] |
| VB-IPC-COMMAND-002 | existing | [existing artifact] |
| VB-IPC-FRAME-001 | existing | [existing artifact] |
| VB-IPC-FRAME-002 | existing | [existing artifact] |
| VB-IPC-FRAME-003 | **NEW OBLIGATION REQUIRED** | tbd |
| VB-IPC-PAYLOAD-001 | existing | [existing artifact] |
| VB-IPC-RESPONSE-001 | **NEW OBLIGATION REQUIRED** | tbd |
| VB-IPC-SERVER-001 | existing | [existing artifact] |
| VB-IPC-SERVER-002 | new (TLA+) | proof-obligations.planned.jsonl |
| VB-IPC-SERVER-003 | new (Kani+TLA+) | proof-obligations.planned.jsonl |
| VB-IPC-SERVER-004 | new (TLA+) | proof-obligations.planned.jsonl |
| VB-IPC-FRAGMENT-001 | new (TLA++Proptest) | proof-obligations.planned.jsonl |
| VB-IPC-FRAGMENT-002 | new (TLA++Proptest) | proof-obligations.planned.jsonl |
```

Each "[existing artifact]" placeholder must be replaced with the actual file path and obligation_id from the existing proof artifacts.

---

## Verification After Repair

After making these changes, re-run the proof-plan-reviewer on the updated artifacts.

The plan will be APPROVED if:
1. VB-IPC-FRAME-003 has a lane decision (either Kani harness or documented code-review sufficiency)
2. VB-IPC-RESPONSE-001 has a lane decision (either test/proof or documented code-review sufficiency)
3. The coverage table shows all 28 seeds are accounted for

---

## What NOT to Change

- The 15 existing obligations in proof-obligations.planned.jsonl are sound — do not modify them
- The verifier-lane-review.jsonl correctly identifies the strengths and gaps — reviewer rows are accurate
- The trusted-base-plan.md is solid — no changes needed
- The proof-to-implementation-input.md bridge is correct — no changes needed
- The non-applicable lane justifications (loom, miri, flux, cargo-fuzz) are correct — no changes needed
