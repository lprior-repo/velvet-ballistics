# Proof Plan Repair Guide: vb-0jll

## State to Rerun From

State 4 (proof-planning). Return all artifacts to planner for revision.

## Critical Blockers to Resolve

### F-001: proof-obligations.planned.jsonl Schema Non-Compliance

**Problem**: All 6 obligations missing `schema_version: "proof-obligation/v1"` and 8 required fields.

**Required Fix**:

Add the following fields to each of the 6 obligations:

```json
{
  "schema_version": "proof-obligation/v1",
  "domain_claim": "<string describing the domain-level claim>",
  "risk_tags": ["<risk category>"],
  "target": "<exact function or harness name>",
  "workdir": "/home/lewis/src/velvet-ballistics",
  "model_bounds": "<kani-specific bounds, e.g. unwind(4) or concretized size>",
  "tool_metadata": {"kani_version": "0.42+", "unwind": <N>},
  "trusted_base_refs": ["<external surface references>"],
  "behavior_affecting": <false for delete/replace actions; determine for add actions>
}
```

For **PO-001** (DELETE): Change `behavior_affecting` to `false` (file deletion is not behavior-affecting for runtime).

For **PO-002 through PO-006** (verify-proof): Set `behavior_affecting` appropriately — adding Ok-path assertions is behavior-affecting if the assertions constrain behavior, which they do.

### F-002: verifier-lane-decisions.jsonl Schema Non-Compliance

**Problem**: All 48 rows missing `schema_version` and use wrong field names.

**Required Fix**:

Each row must include:
```json
{
  "schema_version": "verifier-lane-decision/v1",
  "applicability": "<required | not_applicable | blocked_tooling>",
  "decision_reason": "<reason, same as current rationale>",
  "required_obligation_ids": ["<PO-XXX>"],
  "non_applicability_evidence_refs": ["<if not_applicable, cite evidence>"],
  "limitation_kind": "<if not_applicable, the kind of limitation>",
  "owner_state": 5,
  "status": "accepted"
}
```

For **kani lanes** (vld-001, vld-009, vld-017, vld-025, vld-033, vld-041):
- `applicability: "required"`
- `required_obligation_ids: ["<corresponding PO-XXX>"]`

For **non-kani lanes**:
- `applicability: "not_applicable"`
- `non_applicability_evidence_refs: ["#![forbid(unsafe_code)] in <file>"]`
- `limitation_kind: "no_unsafe_code" | "no_concurrency" | "no_verus_specs" | etc.`

### F-003: Behavior-Affecting Waivers

**Problem**: WC-001 and WC-002 are invalid because they are behavior-affecting.

**Required Fix**: DELETE waiver-candidates.jsonl entirely and restructure the plan:

**Option A (Full Scope)**: Write actual Ok-path Kani harnesses for seeds 004-006 without waivers. The proof-writer must implement and verify:
- `submit_artifact_ok_path`: proves Ok result has non-None artifact_id and correct workflow_digest
- `admit_compiled_artifact_ok_path`: proves Ok result digest matches workflow.digest()
- `hydrate_run_frame_ok_path`: proves Ok RunFrame matches expected snapshot+tail merge semantics

**Option B (Reduced Scope)**: If Ok-path proofs cannot be written, REMOVE seeds 004-006 from the plan entirely. The plan should only cover:
- PO-001: DELETE 6 tautological flag proofs
- PO-002: REPLACE verification_proof_digest_binding with meaningful digest proof
- PO-003: REPLACE recover_runtime_summary_precond_basic with meaningful recovery proof

Do NOT use waivers to defer Ok-path proof obligations.

### F-004: Command Masking with `|| true`

**Problem**: `|| true` silently ignores Kani failures.

**Required Fix**: Remove `|| true` from PO-002 through PO-006 commands:

```bash
# Before (WRONG)
cargo kani --no-remove-typestate -p vb_storage --harness verification_proof_digest_binding_meaningful 2>&1 || true

# After (CORRECT)
cargo kani --no-remove-typestate -p vb_storage --harness verification_proof_digest_binding_meaningful 2>&1
```

If the proof is expected to fail during development, document this as a separate "expected FAIL" phase with explicit `expected_evidence: "Kani assertion failure at <location>"` — not masked with `|| true`.

### F-005: Missing Explicit Unwind Bounds in Commands

**Problem**: proof-strategy.md specifies unwind(4) and unwind(5) but commands don't include `--unwind`.

**Required Fix**: Add explicit `--unwind` to each obligation command:

- **PO-002** (verification_proof_digest_binding): `cargo kani ... --harness verification_proof_digest_binding_meaningful --unwind 4`
- **PO-003** (recover_runtime_summary): `cargo kani ... --harness recover_runtime_summary_meaningful --unwind 5`
- **PO-004** (submit_artifact_ok_path): `cargo kani ... --harness submit_artifact_ok_path --unwind 4`
- **PO-005** (admit_compiled_artifact_ok_path): `cargo kani ... --harness admit_compiled_artifact_ok_path --unwind 4`
- **PO-006** (hydrate_run_frame_ok_path): `cargo kani ... --harness hydrate_run_frame_ok_path --unwind 5`

## Additional Recommendations

1. **Add invocation ledger**: proof-obligations.planned.jsonl should reference an `agent-invocation-ledger.jsonl` proving the planner ran independently from the reviewer.

2. **Clarify PO-001 command**: The DELETE obligation uses `rm` directly. Consider using a two-phase approach:
   - Phase 1: `cargo kani` confirms the file is unreachable
   - Phase 2: `rm` removes the file

3. **Add bridge references**: The plan should include `rust-refinement-obligation/v1` entries mapping each Kani proof claim to specific Rust source refs (file:line) for the proof-to-implementation phase.

## Minimal Rerun State

Return to State 4 (proof-planning). Required inputs:
- Revised `proof-obligations.planned.jsonl` with all schema fields
- Revised `verifier-lane-decisions.jsonl` with all schema fields
- Evidence that waiver-candidates.jsonl has been emptied or removed (no behavior-affecting waivers)
- Revised commands without `|| true` and with explicit `--unwind` flags
