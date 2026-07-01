# Proof Plan Repair Guide — vb-09aaz

This guide explains the minimum changes required to bring the proof plan from
`STATUS: REJECTED` to a re-reviewable state. The plan is substantively sound;
the rejection is mechanical.

## Blockers (must fix before re-review)

### B1 — proof-obligations.planned.jsonl schema drift (E_SCHEMA_MISSING_FIELD + E_SCHEMA_ALIAS_FIELD)

All 5 obligation rows (PO-09aaz-001..PO-09aaz-005) must be re-emitted with the
following canonical fields added:

| Field | Required by schema | Notes |
| --- | --- | --- |
| `schema_version` | proof-obligation/v1 | Always `"proof-obligation/v1"` |
| `target` | proof-obligation/v1 | Canonical verifier target (replaces legacy aliases `layer`, `checker`); differs from `artifact` |
| `workdir` | proof-obligation/v1 | Absolute path to repo root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz` |
| `model_bounds` | proof-obligation/v1 | Array of bounded-state assumptions; cite from trusted-base-plan.md sections 1-2 |
| `tool_metadata` | proof-obligation/v1 | Object: verifier version pin + command flag list |
| `trusted_base_refs` | proof-obligation/v1 | Array of trusted-base-plan.md section IDs (1..7) |
| `risk_tags` | proof-obligation/v1 | Structured array; replaces comma-separated `risk` string |
| `domain_claim` | proof-obligation/v1 | Rename `claim` -> `domain_claim` |
| `proof_seed_id` | proof-obligation/v1 | Singular; replace `proof_seed_ids` array with primary seed per row |

#### Per-row field values

##### PO-09aaz-001 (verus, behavior-affecting)

```json
{
  "schema_version": "proof-obligation/v1",
  "id": "PO-09aaz-001",
  "requirement_id": "C1,C2,C4,C7",
  "contract_clause": "contract.md#C1; contract.md#C2; contract.md#C4; contract.md#C7",
  "proof_seed_id": "vb-09aaz-PS-001",
  "domain_claim": "Every fallible step in JournalWriteBatch::append_event (including G8 IndexKeyConstruction) sets self.aborted = true before propagating Err(KeyCapacity). The Verus spec mirror at PS-008/PS-009 is extended to model G8 with a new index_key_ok:bool exec arg, a new Err(KeyCapacity) match arm in assume_specification requiring spec_state_preserved_except_aborted with witness !index_key_ok, and a new exec wrapper wrapper_append_event_index_key_error. The 8-guard order G1..G8 is verified by lemma_guard_order_is_valid.",
  "risk": "persistence,public-api,verifier-binding,production-binding",
  "risk_tags": ["persistence","public-api","verifier-binding","production-binding"],
  "verifier": "verus",
  "artifact": "verification/verus/vb-vzcuf-PS-008.rs; verification/verus/vb-vzcuf-PS-009.rs; verification/verus/production_inner/vb_vzcuf_PS_008_production.rs; verification/verus/production_inner/vb_vzcuf_PS_009_production.rs",
  "target": "verification/verus/vb-vzcuf-PS-008.rs",
  "command": "bash scripts/verify-verus.sh && bash scripts/check-verus-production-binding.sh && bash scripts/check-production-inner-drift.sh",
  "workdir": "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz",
  "expected_evidence": "all 3 scripts exit 0; production-binding gate shows STRONG/WEAK_EXTERN matches for PS-008 and PS-009; drift gate shows zero drift between production and mirror",
  "assumptions": ["the production fix is committed before this verification runs","the drift-gate header at PS-008 L5-14 and PS-009 L5-32 has been honored (regeneration completed)","the new index_key_ok:bool arg has been added to the SpecJournalWriteBatch::append_event signature in both production mirrors"],
  "model_bounds": ["MAX_BATCH_COUNT=10000","INDEX_ACTION_KEY_BYTES=13","JOURNAL_KEY_BYTES=17","byte_limit=Some(1048576)","guard_order_G1_lt_G2_lt_G3_lt_G4_lt_G5_lt_G6_lt_G7_lt_G8"],
  "tool_metadata": {"verifier_version": "<pin from rust-toolchain.toml>","flags": ["--crate-type=lib","--edition 2021"]},
  "trusted_base_refs": [1, 3, 4, 6, 7],
  "production_binding": { ... existing fields ... },
  "required": true,
  "behavior_affecting": true,
  "mode": "verify-standard",
  "owner_state": 5,
  "rerun_from": 5,
  "status": "planned",
  "waiver": null
}
```

##### PO-09aaz-002 (rust-local, behavior-affecting)

Same schema additions; key fields:

- `proof_seed_id`: "vb-09aaz-PS-002" (primary; PS-007 secondary documented in assumptions)
- `target`: "crates/vb_storage/src/batch/append_event.rs"
- `workdir`: "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz"
- `model_bounds`: ["MAX_BATCH_COUNT=10000","INDEX_ACTION_KEY_BYTES=13","single-batch singleton"]
- `tool_metadata`: {"verifier_version": "<cargo pin>", "flags": ["--test"]}
- `trusted_base_refs`: [1, 3, 6]
- `risk_tags`: ["persistence","public-api"]
- `domain_claim`: rename from `claim`

##### PO-09aaz-003 (proptest, behavior-affecting)

- `proof_seed_id`: "vb-09aaz-PS-005"
- `target`: "crates/vb_storage/src/batch/append_event.rs"
- `model_bounds`: ["ActionId u16","RunId u64","StepIdx u16","INDEX_ACTION_KEY_BYTES=13"]
- `trusted_base_refs`: [1, 3, 6]
- `risk_tags`: ["persistence","arithmetic","public-api"]

##### PO-09aaz-004 (persistence, behavior-affecting)

- `proof_seed_id`: "vb-09aaz-PS-006"
- `target`: "crates/vb_storage/src/batch/append_event.rs"
- `model_bounds`: ["MAX_BATCH_COUNT=10000","OwnedWriteBatch atomicity","fjall substrate durability"]
- `trusted_base_refs`: [1, 3, 4]
- `risk_tags`: ["persistence","master-contract"]

##### PO-09aaz-005 (rust-local, NON-behavior-affecting)

- `proof_seed_id`: "vb-09aaz-PS-007" (primary; PS-004 secondary)
- `target`: "crates/vb_storage/src/batch/append_event.rs"
- `model_bounds`: ["signature unchanged","error variant unchanged","accessor surface unchanged"]
- `trusted_base_refs`: [3]
- `risk_tags`: ["public-api","migration"]
- `production_binding.production_lines`: tighten to `[18,19,20,21,22,23,24,25,26,33,34,35,36,37,38,39,40,41]` (doc-comment lines only) — addresses F-09aaz-05

### B2 — agent-invocation-ledger.jsonl missing state-3 and state-4 rows (E_INVOCATION_LEDGER_MISSING)

The femdation controller must append:

1. **state-3 (rust-contract)** row:
   - `ledger_sequence: 3`
   - `previous_entry_hash`: `<current entry_hash for ledger_sequence=2>` (i.e., `edcdfb974ef698de19c20b9bd08eaf11b191ba7bd4af6fbc179af7941768dfcd`)
   - `entry_hash`: new SHA-256 of the canonical row excluding entry_hash
   - `skill: "rust-contract"`
   - `state: 3`
   - `parent_invocation_id: "go-skill-vb-09aaz-state1"`
   - `output_artifacts`: ["contract.md", "type-contracts.md", "domain-model.md", "workflow-model.md", "error-taxonomy.md", "boundary-map.md", "hazard-analysis.md", "proof-seeds.jsonl", "traceability-matrix.jsonl"]
   - `output_artifact_hashes`: SHA-256 of each
   - `transcript_artifact: "transcript-state3.txt"` (must be created)

2. **state-4 (proof-planner)** row:
   - `ledger_sequence: 4`
   - `previous_entry_hash`: `<entry_hash for ledger_sequence=3>`
   - `entry_hash`: new SHA-256
   - `skill: "proof-planner"`
   - `state: 4`
   - `parent_invocation_id: "<state-3 invocation_id>"`
   - `output_artifacts`: ["proof-strategy.md", "verifier-lane-decisions.jsonl", "proof-obligations.planned.jsonl", "trusted-base-plan.md", "waiver-candidates.jsonl", "proof-coverage-matrix.md", "verifier-lane-matrix.md"]
   - `output_artifact_hashes`: SHA-256 of each
   - `transcript_artifact: "transcript-state4.txt"` (must be created)

The entry_hash field per agent-invocation/v1 must equal the SHA-256 of the
canonical row content excluding entry_hash. See `.agents/skills/go-skill/SKILL.md`
for the hash chain algorithm.

## Non-Blockers (record for backfill)

### NB1 — Transcript gaps (E_TRANSCRIPT_MISSING, owner_approved_debt)

- `transcript-state3.txt` and `transcript-state4.txt` should be created and
  populated with timestamps + output artifact SHA-256 hashes + invocation IDs.
- Disposition: `owner_approved_debt`; debt_ref: `DEBT-09aaz-TRANSCRIPT`.
- This is non-blocking; backfill can occur in parallel with proof-writer work.

### NB2 — PO-09aaz-005 production_lines over-reporting (E_BRIDGE_PLAN_PARTIAL, owner_approved_no_action)

- Tighten `production_lines` to doc-comment lines only:
  `[18,19,20,21,22,23,24,25,26,33,34,35,36,37,38,39,40,41]`.
- Disposition: `owner_approved_no_action`; this is conservative over-reporting
  that does not weaken the proof.
- This is non-blocking; can be fixed during repair of B1.

## Smallest State to Rerun

After B1 and B2 are complete, this reviewer (proof-plan-reviewer) should re-run
on the same bead (vb-09aaz, workdir=/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz).

Re-run expected outcome:
- 16 verifier-lane-review/v1 rows re-issued with:
  - `planner_invocation_id`: actual ledger value (not the placeholder)
  - `reviewer_disposition: "accepted"` for all 16 rows
  - `finding_refs`: empty (or only F-09aaz-04 + F-09aaz-05 if still pending)
- proof-plan-review.md status flipped to `STATUS: APPROVED`.
- state-4 row appended to ledger (this is the reviewer's row).
- proof-writer can then begin State 5.

## Re-Review Checklist for proof-planner

- [ ] All 5 obligation rows have `schema_version: "proof-obligation/v1"`
- [ ] All 5 obligation rows have `target`, `workdir`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `risk_tags`
- [ ] All 5 obligation rows use `domain_claim` (not `claim`)
- [ ] All 5 obligation rows use `proof_seed_id` (not `proof_seed_ids` array)
- [ ] PO-09aaz-005 `production_lines` is tightened to doc-comment lines
- [ ] state-3 + state-4 ledger rows appended with full hash chain
- [ ] transcript-state3.txt and transcript-state4.txt created
- [ ] All SHA-256 hashes in ledger rows match the actual artifacts on disk

After these fixes, the plan should re-evaluate cleanly to `STATUS: APPROVED`.