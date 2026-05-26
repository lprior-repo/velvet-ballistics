# Proof Plan Repair Guide: vb-xi2f.9

**Reviewer:** proof-plan-reviewer (ppr-vb-xi2f.9-001)  
**Plan State:** REJECTED  
**Required State:** Return to State 4 with all repairs applied

---

## Repair Overview

All 6 blocking findings (F-001 through F-006) and 5 non-blocking findings (F-007 advisory) require repair. The proof strategy and reasoning are sound — repair is **purely schema compliance and provenance**. Expected time: ~60 minutes of careful field renaming and addition.

---

## Repair 1: `proof-obligations.planned.jsonl` — Schema Compliance (F-001, F-002, F-003)

**Current State:** All 21 rows missing 8 required fields, 2 alias-field violations.

**Required State:** All 21 rows conform to `proof-obligation/v1` schema with these fields:

### Required Fields to Add

| Field | Type | Value Guidance |
|---|---|---|
| `schema_version` | string | `"proof-obligation/v1"` for all rows |
| `domain_claim` | string | Extract from corresponding proof-seed's `domain_claim` field |
| `risk_tags` | array[string] | Convert existing `risk` string to single-element array (e.g., `"invariant"` → `["invariant"]`) |
| `target` | string | Use `artifact` value or derive from verifier (e.g., `"kani harness: span_paired_invariant_proof"`) |
| `model_bounds` | object | Rename existing `bounds` field to `model_bounds`; keep content unchanged |
| `tool_metadata` | object\|null | `null` if no tool-specific metadata; otherwise `{"kani_version": "latest", "unwind": N}` |
| `trusted_base_refs` | array[string] | Reference the trusted-base-ledger IDs that back this obligation (use `["TB-001", ...]` after creating trusted-base-ledger) |
| `behavior_affecting` | boolean | `true` for proofs, `false` for CI/grep gates; match proof-seed's `behavior_affecting` |

### Field Renames

| Old Name | New Name |
|---|---|
| `bounds` | `model_bounds` |
| `risk` | `risk_tags` (convert string to array) |

### Example repair for PO-K01

```json
{
  "schema_version": "proof-obligation/v1",
  "id": "PO-K01",
  "requirement_id": "C1.1-C1.3",
  "contract_clause": "SPAN-ENRICH",
  "domain_claim": "Enriched Span with optional line/column fields maintains backward compatibility. Span paired invariant holds for all public constructors.",
  "risk": "invariant",
  "risk_tags": ["public API", "invariant"],
  "verifier": "kani",
  "artifact": "crates/vb_core/proofs/span_kani.rs",
  "target": "kani harness: span_paired_invariant_proof",
  "command": "cargo kani --proof span_with_location_produces_paired_invariant --unwind 3 --harness span_paired_invariant_proof",
  "workdir": "/home/lewis/src/vb-workspaces/vb-xi2f.9",
  "expected_evidence": "Kani reports VERIFICATION SUCCESSFUL for all harnesses.",
  "assumptions": ["Span line and column fields are Option<u32>", "u32 values bounded to [0, u32::MAX]"],
  "model_bounds": {"u32_values": "kani::any()", "max_unwind": 3},
  "tool_metadata": {"kani_unwind": 3},
  "trusted_base_refs": ["TB-TRUSTED-STDLIB", "TB-KANI-ARBITRARY-SPAN"],
  "required": true,
  "behavior_affecting": true,
  "mode": "verify-proof",
  "owner_state": 6,
  "rerun_from": 6,
  "status": "planned",
  "waiver": null
}
```

---

## Repair 2: `verifier-lane-decisions.jsonl` — Schema Compliance (F-004, F-005)

**Current State:** All 96 rows have wrong field names and missing required fields.

**Required State:** All 96 rows conform to `verifier-lane-decision/v1` with these fields:

### Required Fields to Add

| Field | Type | Value Guidance |
|---|---|---|
| `id` | string | `"vld-001"` through `"vld-096"` (sequential per row) |
| `risk_tags` | array[string] | From corresponding proof-seed's `risk_tags` |
| `non_applicability_evidence_refs` | array[string] | For `not_applicable` rows: extract references from existing `evidence` field (e.g., `["HA-06", "workflow-model.md §5"]`). For `required` rows: `[]` |
| `limitation_kind` | string | `"none"` for `required`, `"not_triggered"` for `not_applicable` |
| `owner_state` | integer | `4` (proof-planning state) |
| `status` | string | `"planned"` |

### Field Renames

| Old Name | New Name | Value Transformation |
|---|---|---|
| `decision` | `applicability` | Keep values: `"required"` or `"not_applicable"` |
| `evidence` | `decision_reason` | Keep text content unchanged |
| `obligation_id` | `required_obligation_ids` | Convert single string/null to array (e.g., `"PO-K01"` → `["PO-K01"]`, `null` → `[]`) |

### Example repair for PS-001/Kani (row 3)

```json
{
  "schema_version": "verifier-lane-decision/v1",
  "id": "vld-003",
  "requirement_id": "C1.1-C1.3",
  "contract_clause": "SPAN-ENRICH",
  "proof_seed_id": "PS-001",
  "verifier": "kani",
  "risk_tags": ["public API", "invariant"],
  "applicability": "required",
  "decision_reason": "Bounded invariant (Span paired line/column) is well-suited to Kani bounded model checking. Verification of public constructors with finite (u32) domain.",
  "required_obligation_ids": ["PO-K01"],
  "non_applicability_evidence_refs": [],
  "limitation_kind": "none",
  "owner_state": 4,
  "status": "planned"
}
```

### Example repair for PS-001/TLA+ (row 1)

```json
{
  "schema_version": "verifier-lane-decision/v1",
  "id": "vld-001",
  "requirement_id": "C1.1-C1.3",
  "contract_clause": "SPAN-ENRICH",
  "proof_seed_id": "PS-001",
  "verifier": "tla-plus",
  "risk_tags": ["public API", "invariant"],
  "applicability": "not_applicable",
  "decision_reason": "No temporal workflows, retries, leases, queues, or distributed protocols. Pipeline is single-threaded, deterministic.",
  "required_obligation_ids": [],
  "non_applicability_evidence_refs": ["HA-06", "workflow-model.md §5", "workflow-model.md §6"],
  "limitation_kind": "not_triggered",
  "owner_state": 4,
  "status": "planned"
}
```

---

## Repair 3: `agent-invocation-ledger.jsonl` — Add Proof-Planner Entry (F-006)

**Current State:** Only femdation setup entry.

**Required State:** Append a proof-planner invocation entry.

**Example entry to append:**
```json
{
  "schema_version": "agent-invocation/v1",
  "ledger_sequence": 2,
  "invocation_id": "planner-vb-xi2f.9-001",
  "parent_invocation_id": "femdation-vb-xi2f.9-001",
  "skill": "proof-planner",
  "state": 4,
  "workdir": "/home/lewis/src/vb-workspaces/vb-xi2f.9",
  "input_artifacts": [".beads/vb-xi2f.9/contract.md", ".beads/vb-xi2f.9/proof-seeds.jsonl", ".beads/vb-xi2f.9/traceability-matrix.jsonl"],
  "output_artifacts": [".beads/vb-xi2f.9/proof-strategy.md", ".beads/vb-xi2f.9/verifier-lane-decisions.jsonl", ".beads/vb-xi2f.9/proof-obligations.planned.jsonl", ".beads/vb-xi2f.9/trusted-base-plan.md", ".beads/vb-xi2f.9/waiver-candidates.jsonl", ".beads/vb-xi2f.9/proof-coverage-matrix.md"],
  "started_at": "2026-05-24T00:00:00Z",
  "completed_at": "2026-05-24T00:00:00Z",
  "status": "completed"
}
```

> **Note:** Also add `schema_version` to the existing femdation entry and populate hashes if available. The femdation entry currently lacks the `agent-invocation/v1` schema marker.

---

## Repair 4: `waiver-candidates.jsonl` — Fix Duplicate ID + Missing Fields (F-007, F-008)

**Current State:** Two rows share `id: "WC-03"`. Missing `boundary_proof`. Wrong field name `reviewer_status`.

**Required State:**

1. **Re-ID the duplicate:** Change the second `WC-03` (for PS-010/UNIFY-DIAG) to `"WC-04"`. Change the current `WC-04` (for PS-011/SEM-MAP-MSG) to `"WC-05"`.

2. **Add `boundary_proof`** field to all 5 rows. Value: the boundary being waived. Examples:
   - WC-01: `"Kani PO-K02 + proptest PO-P02 cover all invariants; Flux refinement on generic Vec wrapper adds complexity without benefit"`
   - WC-02: `"usize→usize conversion (same-width, no risk); risky usize→u32 path covered by PO-M01"`
   - WC-03: `"Dead code removal — no runtime behavior; grep + cargo-check is the complete verification"`
   - WC-04: `"Refactoring, behavior unchanged; grep + cargo-test is the complete verification"`
   - WC-05: `"Kani cannot model string formatting (heap strings); proptest PO-P07 + unit tests provide complete coverage"`

3. **Rename `reviewer_status`** to `review_status` on all rows.

---

## Repair 5: Create `trusted-base-ledger.jsonl` (F-009)

**Current State:** Prose-only `trusted-base-plan.md`.

**Required State:** Create `trusted-base-ledger.jsonl` with one `trusted-base-ledger/v1` row per trusted assumption, stub, model reduction, and trusted operation.

**Estimated rows:** ~25-30 (one per entry in sections 1-6 of trusted-base-plan.md).

**Example entries:**
```json
{"schema_version":"trusted-base-ledger/v1","id":"TB-001","obligation_id":"ALL","artifact":"crates/vb_core/src/span.rs","location":"Span::ZERO const","marker":"trusted","trusted_kind":"constant","reason":"Hardcoded const; never changes; verified by compile-time constant propagation.","scope":"all Span proofs (PO-K01, PO-F01, PO-P01)","impact":"low","behavior_affecting":false,"compensating_evidence":"PO-K01, PO-P01","owner":"proof-planner","expiry":"bead-landing","reviewer_disposition":"pending","status":"planned"}
{"schema_version":"trusted-base-ledger/v1","id":"TB-002","obligation_id":"PO-K01,PO-K03,PO-K06","artifact":"kani stubs","location":"kani::Arbitrary for Span","marker":"stub","trusted_kind":"model_reduction","reason":"Kani harness generates arbitrary Span values via kani::Arbitrary. Bounded to honest limits (u32).","scope":"PO-K01, PO-K03, PO-K06","impact":"medium","behavior_affecting":false,"compensating_evidence":"proptest PO-P01 covers broad input space beyond Kani bounds","owner":"proof-planner","expiry":"bead-landing","reviewer_disposition":"pending","status":"planned"}
```

---

## Repair 6: Add Non-Vacuity Section to `proof-strategy.md` (F-010)

Add a section titled `## Non-Vacuity Plan` after the `## Strategy Pillars` section but before `## Non-Applicable Lanes`. The section should enumerate:

1. Every Kani `assume()` statement and why it is non-vacuous (referencing concrete harness evidence)
2. Every proptest `Arbitrary` strategy and its edge-case coverage
3. Every stub/model reduction from trusted-base-plan §4-§5 and how correctness is independently validated
4. Confirmation that `kani::proof` harnesses exercise the actual production implementation, not simplified stubs (except where explicitly documented and trusted)

**Example content:**
```markdown
## Non-Vacuity Plan

### Kani Assumption Audit

| Obligation | Assumption | Non-Vacuity Check |
|---|---|---|
| PO-K01 | Span line/col are Option<u32> | True by type system; Kani proves no panic for all u32 values |
| PO-K03 | String allocation succeeds | Kani uses abstract string representation; string content does not affect invariant; proptest covers string content |

### Proptest Strategy Audit

| Obligation | Strategy | Edge-Case Coverage |
|---|---|---|
| PO-P01 | u32::ANY + start<=end filter | Covers u32::MAX, 0, 1, and all intermediate values |
| PO-P07 | YAML with known paths + intentional errors | Covers deep nesting (1..4), duplicate keys, unknown fields |
```

---

## Repair 7 (Advisory): Resolve PO-G04 Redundancy (F-011)

Either:
- **Option A:** Remove PO-G04 entirely (moon ci already runs cargo test --workspace)
- **Option B:** Change PO-G04 `mode` to `"verify-by-build (po-g03-sub-check)"` and skip it in proof-writer execution (reference PO-G03 as parent)

---

## Repair Execution Order

1. **Repair 2 first**: Fix `verifier-lane-decisions.jsonl` (96 rows, field renames + additions)
2. **Repair 1 second**: Fix `proof-obligations.planned.jsonl` (21 rows, field additions + renames)
3. **Repair 4**: Fix `waiver-candidates.jsonl` (5 rows, re-ID + field additions)
4. **Repair 5**: Create `trusted-base-ledger.jsonl` (~25-30 rows)
5. **Repair 6**: Add non-vacuity section to `proof-strategy.md`
6. **Repair 3**: Add proof-planner entry to `agent-invocation-ledger.jsonl`
7. **Repair 7**: Resolve PO-G04 redundancy
8. **Re-run proof-plan-reviewer** after all repairs are applied.

---

## Verification Before Re-Review

Run these checks before submitting for re-review:

```bash
# Schema compliance
jq -e '.schema_version == "proof-obligation/v1"' .beads/vb-xi2f.9/proof-obligations.planned.jsonl > /dev/null && echo "PASS: obligations schema" || echo "FAIL: obligations schema"
jq -e '.schema_version == "verifier-lane-decision/v1"' .beads/vb-xi2f.9/verifier-lane-decisions.jsonl > /dev/null && echo "PASS: lane decisions schema" || echo "FAIL: lane decisions schema"
jq -e '.schema_version == "waiver-candidate/v1"' .beads/vb-xi2f.9/waiver-candidates.jsonl > /dev/null && echo "PASS: waiver schema" || echo "FAIL: waiver schema"

# Invocation ledger
grep -c '"skill":"proof-planner"' .beads/vb-xi2f.9/agent-invocation-ledger.jsonl && echo "PASS: planner entry" || echo "FAIL: no planner entry"

# Unique waiver IDs
jq -r '.id' .beads/vb-xi2f.9/waiver-candidates.jsonl | sort | uniq -d | grep . && echo "FAIL: duplicate waiver IDs" || echo "PASS: unique waiver IDs"

# Lane decision count
jq -s 'length' .beads/vb-xi2f.9/verifier-lane-decisions.jsonl | grep -q '^96$' && echo "PASS: 96 lane decisions" || echo "FAIL: wrong lane decision count"

# Obligation count
jq -s 'length' .beads/vb-xi2f.9/proof-obligations.planned.jsonl | grep -q '^21$' && echo "PASS: 21 obligations" || echo "FAIL: wrong obligation count"

# Trusted-base ledger exists
test -s .beads/vb-xi2f.9/trusted-base-ledger.jsonl && echo "PASS: trusted-base-ledger exists" || echo "FAIL: trusted-base-ledger missing"
```

---

**Rerun Gate:** `proof-plan-reviewer` with all repairs applied.
