# Proof Plan Repair Guide: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Review:** proof-plan-reviewer (State 5)
**Status:** REJECTED — schema drift only, no substantive re-planning needed
**Minimum re-run state:** State 4 (proof-planner) — re-serialize with corrected fields

---

## Repair Summary

All eight findings are mechanical. No proof strategy changes, no new obligations, no lane reassignments needed. The substantive plan is approved in principle. Only the JSONL serialization format must be repaired to match `verifier-lane-decision/v1`, `proof-obligation/v1`, and `agent-invocation/v1` schemas.

---

## Repair 1: proof-obligations.planned.jsonl

### Required changes per row (all 16 rows: PO-001 through PO-016):

#### 1a. Add `schema_version`
```
"schema_version": "proof-obligation/v1"
```

#### 1b. Rename `bounds` → `model_bounds`
```diff
- "bounds": {"max_string_len": 16, "alphabet": "a-zA-Z0-9_"}
+ "model_bounds": {"max_string_len": 16, "alphabet": "a-zA-Z0-9_"}
```

For PO-008 (`"bounds":"existing default"`): convert to null or proper object.
For PO-014 (`"bounds":null`): rename to `"model_bounds": null`.

#### 1c. Add missing required fields (values per row — see table below)

| Field | Source | Example |
|-------|--------|---------|
| `domain_claim` | proof-seeds.jsonl → `domain_claim` | `"Different wait event values produce different WorkflowDigest values"` |
| `risk_tags` | proof-seeds.jsonl → `risk_tags` array | `["digest_collision","semantic_integrity","behavior_affecting"]` |
| `target` | Rust function path | `"vb_compile::digest_step_primitive"` |
| `tool_metadata` | null or {} | `null` |
| `trusted_base_refs` | TB IDs from trusted-base-plan.md | `["TB-001","TB-002"]` |
| `behavior_affecting` | proof-seeds.jsonl → `behavior_affecting` boolean | `true` |

#### 1d. Row-level mapping of missing fields

| PO | Seed | domain_claim (hint) | behavior_affecting | trusted_base_refs |
|----|------|---------------------|--------------------|--------------------|
| PO-001 | ps-wait-008 | "The new Wait match arm in digest_step_primitive is panic-free" | false | ["TB-001","TB-002","TB-003"] |
| PO-002 | ps-wait-001 | "Different wait event values produce different WorkflowDigest values" | true | ["TB-001","TB-005"] |
| PO-003 | ps-wait-001 | "Different wait event values produce different WorkflowDigest values" | true | ["TB-001","TB-005"] |
| PO-004 | ps-wait-002 | "WaitUntil and WaitEvent produce different WorkflowDigest values" | true | ["TB-001","TB-005"] |
| PO-005 | ps-wait-002 | "WaitUntil and WaitEvent produce different WorkflowDigest values" | true | ["TB-001"] |
| PO-006 | ps-wait-003 | "WaitEvent with timeout=None != timeout=Some(\"none\")" | true | ["TB-001","TB-005"] |
| PO-007 | ps-wait-003 | "WaitEvent with timeout=None != timeout=Some(\"none\")" | true | ["TB-001","TB-005"] |
| PO-008 | ps-wait-004 | "canonical_digest produces same output for identical inputs" | false | ["TB-001","TB-004"] |
| PO-009 | ps-wait-005 | "Cold-path and warm-path produce identical WorkflowDigest" | true | ["TB-001","TB-004","TB-005"] |
| PO-010 | ps-wait-005 | "Cold-path and warm-path produce identical WorkflowDigest" | true | ["TB-001","TB-004"] |
| PO-011 | ps-wait-006 | "All 3 legal Wait configurations produce pairwise-distinct digests" | true | ["TB-001","TB-005"] |
| PO-012 | ps-wait-006 | "All 3 legal Wait configurations produce pairwise-distinct digests" | true | ["TB-001","TB-005"] |
| PO-013 | ps-wait-006 | "All 3 legal Wait configurations produce pairwise-distinct digests" | true | ["TB-001"] |
| PO-014 | ps-wait-007 | "All existing tests verify digest determinism continue to pass" | false | ["TB-001","TB-004"] |
| PO-015 | ps-wait-008 | "Both copies of digest_step_primitive are panic-free for Wait" | false | ["TB-001","TB-002","TB-003"] |
| PO-016 | ps-wait-009 | "Both copies remain structurally identical after fix" | true | ["TB-001","TB-004"] |

---

## Repair 2: verifier-lane-decisions.jsonl

### Required changes per row (all 72 rows):

#### 2a. Rename `decision` → `applicability`
```diff
- "decision": "required"
+ "applicability": "required"
- "decision": "not_applicable"
+ "applicability": "not_applicable"
```

#### 2b. Rename `rationale` → `decision_reason`
```diff
- "rationale": "Bounded panic-freedom and collision detection..."
+ "decision_reason": "Bounded panic-freedom and collision detection..."
```

#### 2c. Rename + pluralize `obligation_id` → `required_obligation_ids` (required rows only)
```diff
- "obligation_id": "PO-001"
+ "required_obligation_ids": ["PO-001"]
```

This affects 16 rows: vld-001, vld-002, vld-003, vld-009, vld-010, vld-017, vld-018, vld-025, vld-033, vld-034, vld-041, vld-042, vld-043, vld-049, vld-057, vld-065.

#### 2d. Add five missing fields

```json
"risk_tags": ["digest_collision", "semantic_integrity"],
"non_applicability_evidence_refs": null,
"limitation_kind": null,
"owner_state": 4,
"status": "planned"
```

**For `not_applicable` rows**, populate `non_applicability_evidence_refs` with the concrete evidence document refs already cited in the `decision_reason`:

| Verifier | non_applicability_evidence_refs value | limitation_kind |
|----------|--------------------------------------|-----------------|
| tla-plus | `["boundary-map.md#section-5","boundary-map.md#section-7","hazard-analysis.md#CH-1","hazard-analysis.md#CH-2"]` | `"scope_not_applicable"` |
| verus | `["hazard-analysis.md#UPH-1","boundary-map.md#section-3"]` | `"scope_priority"` |
| flux | `["type-contracts.md#section-7","hazard-analysis.md#RH-1"]` | `"scope_not_applicable"` |
| loom | `["hazard-analysis.md#CH-1","hazard-analysis.md#CH-2","workflow-model.md#section-5"]` | `"scope_not_applicable"` |
| miri | `["hazard-analysis.md#UPH-1","boundary-map.md#section-4"]` | `"scope_not_applicable"` |

---

## Repair 3: agent-invocation-ledger.jsonl

### Add a proof-planner invocation row (append after the existing femdation row):

```json
{
  "schema_version": "agent-invocation/v1",
  "ledger_sequence": 2,
  "previous_entry_hash": "<hash of entry 1>",
  "entry_hash": "<sha256 of this row excluding entry_hash>",
  "host_session_id": "<session-id>",
  "invocation_id": "proof-planner-vb-xi2f.32-001",
  "parent_invocation_id": "<femdation invocation id>",
  "skill": "proof-planner",
  "state": 4,
  "workdir": "/home/lewis/src/vb-workspaces/vb-xi2f.32/.beads/vb-xi2f.32",
  "input_artifacts": ["contract.md", "proof-seeds.jsonl"],
  "input_artifact_hashes": {"contract.md": "<sha256>", "proof-seeds.jsonl": "<sha256>"},
  "output_artifacts": [
    "proof-strategy.md",
    "verifier-lane-decisions.jsonl",
    "proof-obligations.planned.jsonl",
    "trusted-base-plan.md",
    "waiver-candidates.jsonl",
    "waiver-candidates.md",
    "traceability-matrix.jsonl"
  ],
  "output_artifact_hashes": {},
  "transcript_artifact": null,
  "transcript_hash": null,
  "reviewed_artifacts_existed_before_start": true,
  "started_at": "<ISO-8601>",
  "completed_at": "<ISO-8601>",
  "status": "completed"
}
```

---

## Repair 4: waiver-candidates.jsonl

### Required changes per row (all 5 rows: WC-001 through WC-005):

#### 4a. Rename `status` → `review_status`
```diff
- "status": "candidate"
+ "review_status": "candidate"
```

#### 4b. Add separate `requirement_id` and `contract_clause` fields

Split the combined `clause` field. The current values are comma-separated clause IDs. For WC-001 through WC-005:

| WC | `clause` (current) | `requirement_id` (new) | `contract_clause` (new) |
|----|-------------------|----------------------|------------------------|
| WC-001 | `"C1,C2,C3"` | `"C1"` | `"digest_step_primitive must hash Wait{ event, timeout } fields"` |
| WC-002 | `"C4,C5"` | `"C4"` | `"canonical_digest remains deterministic after fix"` |
| WC-003 | `"C1,C2,C3,C4,C5,C6"` | `"C1"` | `"digest_step_primitive must hash Wait{ event, timeout } fields"` |
| WC-004 | `"C1,C2,C3,C4,C5,C6"` | `"C1"` | `"digest_step_primitive must hash Wait{ event, timeout } fields"` |
| WC-005 | `"C1"` | `"C1"` | `"digest_step_primitive must hash Wait{ event, timeout } fields"` |

Remove or migrate the `clause` field after adding the separate fields.

---

## Verification After Repair

After applying all repairs, re-run the review by:
1. Re-invoke `proof-plan-reviewer` at State 5 with the repaired artifacts.
2. All 72 `verifier-lane-review/v1` rows should receive `reviewer_disposition: accepted` (assuming only schema repairs are applied).

---

## Quick Repair Commands

If using a scripted repair (untested, for reference only):

```bash
# Repair proof-obligations:
# - Add schema_version to each row
# - Rename bounds → model_bounds
# - Add missing fields from proof-seeds cross-reference

# Repair lane decisions:
# - Rename decision → applicability
# - Rename rationale → decision_reason
# - Rename obligation_id → required_obligation_ids (with array wrapping)
# - Add risk_tags, non_applicability_evidence_refs, limitation_kind, owner_state, status

# Repair invocation ledger:
# - Append agent-invocation/v1 row for proof-planner
```
