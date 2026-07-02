# Proof Plan Repair Guide: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Review State

State 5 (proof-plan-reviewer) — REJECTED

## Reviewer

- **Skill**: proof-plan-reviewer
- **Invocation ID**: `1ba170d2-14a5-43a5-bf1f-3e444e8ac456`
- **Planner Invocation ID**: `proof-planner-vb-xi2f.35-20260525T041200Z`

## Rejection Reason

**CRITICAL: PS-012 proptest lane is vacuous (E_LANE_DECISION_WEAK)** — The proptest lane for PS-012 (validation coverage, R12/C5) is marked `decision: required` with `obligation_ids: ["PO-K11"]`. PO-K11 is a Kani obligation (`verifier: kani`, `mode: verify-proof`). No distinct proptest obligation exists for validation coverage. The proptest lane's `evidence_requirement` is null. This creates a false positive in the coverage matrix: validation is counted as having proptest coverage when none is planned.

## Required Repairs

### Repair 1 (REQUIRED): Fix PS-012 proptest lane — choose Option A or B

#### Option A: Add proptest obligation PO-P08

Add a new proptest obligation for validation coverage:

```
{"schema_version":"proof-obligation/v1","id":"PO-P08","requirement_id":"vb-xi2f.35-R12",
"contract_clause":"C5","risk":"regression: random contracts not validated against system limits",
"verifier":"proptest","artifact":"crates/vb_core/tests/proptest_validation_coverage.rs",
"command":"cargo test proptest_validation_all_17_fields -- --nocapture",
"workdir":"crates/vb_core",
"expected_evidence":"cargo test reports PASS. For each of 17 fields: generate random values including boundary-violating and valid values, assert validate_resource_contract returns correct error for violations and Ok for valid values. Minimum 100 cases per field.",
"assumptions":["validate_resource_contract is deterministic","HARD_MAX_TRANSITIONS_PER_TICK is stable"],
"bounds":{"cases_per_field":"≥ 100","fields":"all 17 with random values spanning [0, HARD_MAX*2]"},
"required":true,"mode":"run-tests","owner_state":6,"rerun_from":6,"status":"planned","waiver":null}
```

Then update PS-012 proptest lane in `verifier-lane-decisions.jsonl`:
- `obligation_ids: ["PO-P08"]`
- `evidence_requirement: "cargo test proptest_validation_all_17_fields passes with ≥ 100 cases per field"`

#### Option B (RECOMMENDED): Change proptest lane to not_applicable

Change PS-012 proptest lane to `decision: not_applicable`:

```
Rationale: Kani PO-K11 already exhaustively covers all validation boundary values
(max_transitions_per_tick: {0, 1, HARD_MAX, HARD_MAX+1}; allows_secret_results: {true, false}).
Proptest random sampling adds no incremental coverage — the boundary space is fully enumerated by Kani.
Proptest cannot discover validation gaps that Kani's exhaustive boundary check misses.
```

Update in `verifier-lane-decisions.jsonl`:
- `decision: "not_applicable"`
- `obligation_ids: null`
- `evidence_requirement: null`
- `rationale: "Kani PO-K11 exhaustively covers all validation boundary values. Proptest random sampling is non-incremental for deterministic boundary checks."`

And update `proof-coverage-matrix.md`:
- PS-012 proptest cell: change from "P" (PO-K11) to "— (not applicable)"
- Update summary statistics (proptest: 7→6 required, 7→8 not_applicable)

### Repair 2 (RECOMMENDED): Add `id` field to all verifier-lane-decision rows

Add an `id` field to each of 136 rows using format `LD-{seed}-{verifier}` (e.g., `LD-PS001-kani`, `LD-PS012-proptest`). This enables clean `lane_decision_id` references in the verifier-lane-review rows without composite-key workarounds.

### Repair 3 (RECOMMENDED): Add missing canonical fields to proof-obligation rows

Per `proof-obligation/v1` schema, add to each of 26 obligation rows:
- `domain_claim` — copy from corresponding proof seed
- `behavior_affecting` — `true` for all non-waived; `false` for PO-F01 (waived)
- `target` — canonical path to the Rust function under test
- `model_bounds` — rename from `bounds`; include loop-structure justification
- `tool_metadata` — document whether encoding uses loop or macro; justify unwind depth
- `trusted_base_refs` — reference trusted-base-plan.md anchors per assumption
- `risk_tags` — copy from proof seed

### Repair 4 (RECOMMENDED): Emit trusted-base-ledger.jsonl

Create `trusted-base-ledger.jsonl` with one `trusted-base-ledger/v1` row per trust anchor from `trusted-base-plan.md` (28 rows: 5 T0 + 3 T1 + 7 T2 + 11 T3 + 6 T4 + 1 T5). Each row must include `trusted_kind`, `reason`, `scope`, `impact`, `behavior_affecting`, `compensating_evidence`, `owner`, `expiry`, `reviewer_disposition`, `status`.

## What Does Not Need Repair

The following are NOT rejection reasons and do NOT block re-approval:
- **Kani unwind bounds**: `--unwind 2` or `--unwind 3` may be sufficient if the encoding function uses macro-unrolled field hashing (no loop). The proof-writer will determine actual unwind depth. Not a plan defect.
- **Verus command format**: `verus --crate-type=lib` is the proof-writer's responsibility to validate. Not a plan defect.
- **PS-009 TLA+ exclusion**: Recorded as LOW observation (FIND-005). Runtime TLA+ is out of scope for this digest bead.
- **Waiver WC-001**: Valid non-behavior-affecting P2 scope exclusion. APPROVED as-is.

## Minimum Rerun State

After repairs, the proof-planner should:
1. Apply Repair 1 (fix PS-012 proptest lane)
2. Update `proof-coverage-matrix.md` to reflect the fix
3. Update `STATE.md` summary to reflect new lane count
4. Re-run proof-plan-reviewer

Repairs 2-4 are recommended but not required for approval. The CRITICAL finding must be resolved.

## Verification

After repairs, confirm:
```bash
# PS-012 proptest lane is consistent
python3 -c "
import json
with open('.beads/vb-xi2f.35/verifier-lane-decisions.jsonl') as f:
    for line in f:
        if not line.strip(): continue
        d = json.loads(line)
        if d['proof_seed_id']=='PS-012' and d['verifier']=='proptest':
            print(json.dumps(d, indent=2))" | head -10
```
