# Proof Plan Repair Guide — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10
**Review**: proof-plan-reviewer (`ppr-vb-xi2f10-20260525T034500Z-f3a91`)
**Review Result**: REJECTED
**Date**: 2026-05-25

---

## Repair Required: 3 CRITICAL + 1 HIGH

This guide specifies the exact changes needed to pass re-review. After repairs, re-enter State 4 with the proof-plan-reviewer.

---

## REPAIR 1 (CRITICAL): Fix PO-015 Cross-Crate Harness Architecture

**Problem**: PO-015's Kani harness is placed at `crates/vb_core/kani/kani_error_types_code.rs` with `--crate vb_core`, but it targets `RuntimeError::symbolic_code` (in `vb_runtime`) and `JournalError::symbolic_code` (in `vb_storage`). Neither type is importable from within `vb_core` because both crates depend on `vb_core`, not the reverse.

### Option A (Recommended): Relocate to workspace_tests

1. **Update PO-015 fields** in `proof-obligations.planned.jsonl`:
   - `artifact`: `crates/workspace_tests/kani/kani_error_types_code.rs`
   - `command`: `cargo kani --harness kani_error_types_symbolic_code --crate workspace_tests`
   - The `target` field stays: `CoreError::symbolic_code, RuntimeError::symbolic_code, JournalError::symbolic_code`

2. **Update proof-strategy.md** Section 5 (obligation count table) — no change needed; the obligation count is unchanged.

3. **Update proof-to-implementation-input.md** §1 row "Error type registration": change Rust source target from `crates/vb_core/kani/kani_error_types_code.rs` to `crates/workspace_tests/kani/kani_error_types_code.rs`.

4. **Update verifier-lane-decisions.jsonl** VLD-145: no change needed; it references `["PO-015"]` which is unchanged.

### Option B: Split into per-crate harnesses

1. Split PO-015 into three obligations:
   - PO-015a: `crates/vb_core/kani/kani_core_error_code.rs` (`--crate vb_core`, CoreError only)
   - PO-015b: `crates/vb_runtime/kani/kani_runtime_error_code.rs` (`--crate vb_runtime`, RuntimeError only)
   - PO-015c: `crates/vb_storage/kani/kani_journal_error_code.rs` (`--crate vb_storage`, JournalError only)

2. Update all downstream references (strategy.md, coverage matrix, VLD rows, bridge input).

**Recommended**: Option A (minimal change, single harness in correct crate).

---

## REPAIR 2 (CRITICAL): Fix PO-013 Unwind Bound

**Problem**: PO-013 `unwind=10` is insufficient for `DiagnosticCode::symbolic_code()` which scans `CODE_REGISTRY` linearly (90 entries). Kani would report success without exploring the full loop body.

**Fix**: In `proof-obligations.planned.jsonl` PO-013:

```json
{
  "tool_metadata": {
    "kani_version": "0.58+",
    "unwind": 100,
    "harness_count": 1,
    "feature_flags": ["kani"]
  },
  "model_bounds": [
    "Each HasSymbolicCode implementor: arbitrary instance (kani::any)",
    "Two consecutive calls per instance",
    "For DiagnosticCode::symbolic_code(): 90-entry registry scan (unwind=100 covers with margin)",
    "For ValidationError::code(): O(1) match arm (unwind=1)"
  ]
}
```

Change `"unwind": 10` to `"unwind": 100` and update `model_bounds` to document the registry scan path.

Also update **proof-strategy.md §3.6** if it mentions unwind bounds.

---

## REPAIR 3 (CRITICAL): Add Planner Invocation Ledger Entry

**Problem**: `agent-invocation-ledger.jsonl` has only one entry (femdation state-1 setup). No proof-planner invocation row exists.

**Fix**: Append a new `agent-invocation/v1` row to `agent-invocation-ledger.jsonl`. Fill in actual values where indicated:

```json
{
  "schema_version": "agent-invocation/v1",
  "ledger_sequence": 2,
  "previous_entry_hash": "<SHA-256 of row 1 excluding entry_hash>",
  "entry_hash": "<SHA-256 of this row excluding entry_hash>",
  "host_session_id": "<actual host session ID>",
  "invocation_id": "<unique planner invocation ID — must differ from reviewer ppr-vb-xi2f10-20260525T034500Z-f3a91>",
  "parent_invocation_id": "<femdation invocation ID>",
  "skill": "proof-planner",
  "state": 4,
  "workdir": "/home/lewis/src/vb-workspaces/vb-xi2f.10/.beads/vb-xi2f.10",
  "input_artifacts": [
    "contract.md",
    "type-contracts.md",
    "error-taxonomy.md",
    "domain-model.md",
    "hazard-analysis.md",
    "boundary-map.md",
    "workflow-model.md",
    "codebase-map.md",
    "delivery-scope.jsonl",
    "proof-seeds.jsonl",
    "traceability-matrix.jsonl",
    "STATE.md"
  ],
  "input_artifact_hashes": [
    "<SHA-256 of contract.md>",
    "<SHA-256 of type-contracts.md>",
    "..."
  ],
  "output_artifacts": [
    "proof-strategy.md",
    "verifier-lane-decisions.jsonl",
    "proof-obligations.planned.jsonl",
    "proof-coverage-matrix.md",
    "trusted-base-plan.md",
    "waiver-candidates.jsonl",
    "proof-to-implementation-input.md",
    "proof-plan-review-input.md"
  ],
  "output_artifact_hashes": [
    "<SHA-256 of each output artifact>"
  ],
  "transcript_artifact": "<path to transcript if available>",
  "transcript_hash": "<SHA-256 of transcript>",
  "reviewed_artifacts_existed_before_start": true,
  "started_at": "<ISO-8601 timestamp when planning started>",
  "completed_at": "<ISO-8601 timestamp when planning completed>",
  "status": "completed"
}
```

If the planner's exact timestamps are unknown, use the best available evidence (git log, file mtimes). The key requirement is that `invocation_id` must differ from the reviewer's `ppr-vb-xi2f10-20260525T034500Z-f3a91`.

---

## REPAIR 4 (HIGH): Fix Proof Coverage Matrix Proptest Count

**Problem**: `proof-coverage-matrix.md` §2 line 43 says "PO-016–PO-026 (11)" but actual proptest count is 10.

**Fix**: In `proof-coverage-matrix.md`, change:

```
| **proptest** | PO-016–PO-026 (11) | ...
```

to:

```
| **proptest** | PO-016–PO-021, PO-023–PO-026 (10) | ...
```

And in the Key Target column, add: "(PO-022 is cargo-fuzz, listed separately)".

---

## OPTIONAL REPAIRS (MEDIUM/LOW)

These are not blocking but improve plan quality:

### MEDIUM — Copy Evidence Refs to Chained VLD Rows

For each `not_applicable` VLD row with empty `non_applicability_evidence_refs`, copy the evidence refs from the canonical VLD for that verifier:

| Verifier | Canonical VLD | Evidence Refs |
|----------|--------------|---------------|
| `tla-plus` | VLD-003 | `["boundary-map.md#2.1","workflow-model.md#Section7","hazard-analysis.md#Section5"]` |
| `verus` | VLD-004 | `["type-contracts.md#Section11","proof-strategy.md#Section6"]` |
| `flux-rs` | VLD-005 | `["type-contracts.md#Section1","type-contracts.md#Section3"]` |
| `loom` | VLD-006 | `["hazard-analysis.md#Section5","boundary-map.md#Section2.5"]` |
| `miri` | VLD-007 | `["hazard-analysis.md#Section6","boundary-map.md#Section2.5"]` |

### MEDIUM — Fix Traceability Matrix Schema Tag

Change `schema_version: "proof-seed/v1"` to `schema_version: "traceability/v1"` in `traceability-matrix.jsonl`.

### LOW — Trusted-Base Plan JSONL

No action needed at State 4. Before State 12 closure, convert TBL entries to `trusted-base-ledger/v1` JSONL rows.

---

## Minimum State to Re-Enter

After completing repairs 1–4 above, the plan can re-enter State 4 review with the proof-plan-reviewer. Repairs 1 and 2 modify `proof-obligations.planned.jsonl`; repair 3 modifies `agent-invocation-ledger.jsonl`; repair 4 modifies `proof-coverage-matrix.md`.

The verifier-lane-decisions.jsonl (160 rows) does NOT need modification unless the optional MEDIUM repair is applied.

No proof-writer artifacts should be produced until review passes.

---

## Repair Verification Checklist

Before resubmitting:

- [ ] PO-015 `artifact` points to `crates/workspace_tests/kani/kani_error_types_code.rs`
- [ ] PO-015 `command` uses `--crate workspace_tests`
- [ ] PO-013 `tool_metadata.unwind` is `100` (not `10`)
- [ ] PO-013 `model_bounds` updated with registry scan note
- [ ] `agent-invocation-ledger.jsonl` has row 2 with `skill: "proof-planner"`, `state: 4`, distinct `invocation_id`
- [ ] `proof-coverage-matrix.md` §2 proptest count says `(10)` not `(11)`
