# Proof Plan Repair Guide: vb-jpq7.3

## Minimal repair set

1. Repair `proof-obligations.planned.jsonl` to canonical `proof-obligation/v1` fields from `proof-schemas.md`: add `risk`, `verifier` (or rename from `verification_lane` while preserving value), `assumptions`, `mode`, and `rerun_from` for every row; keep exact commands, workdirs, model bounds, raw evidence refs, and limitations.
2. Repair `verifier-lane-decisions.jsonl`: add `risk_tags` to all 72 lane decision rows; keep one row for each core verifier per requirement plus required supplemental cargo-test/static-source-scan/moon-ci rows.
3. Repair `waiver-candidates.jsonl`: either use the canonical singular waiver schema (`requirement_id`, `contract_clause`, `reason`, `boundary_proof`, `compensating_evidence`, `owner`, `expiry`, `review_status`) or provide an explicitly approved schema migration. Behavior-affecting waivers remain forbidden.
4. Repair `verification-ledger.jsonl` to canonical `verification-ledger/v1` fields or provide a schema migration. Latest Moon pass must be `tool_e54cfc867001em3UkY7dnDZZ7z`; older Moon logs must be historical/superseded only.
5. Regenerate `verifier-lane-review.jsonl` after repair with one `verifier-lane-review/v1` row per lane decision and independent planner/reviewer invocation IDs.
6. Update stale prose in `proof-plan-review.md` / lane decision evidence refs that still present `tool_e54ad4ea40019LkG7p2r0N30AH` as latest closure evidence.

## Recheck commands

```bash
python3 - <<'PY'
import json, pathlib
for path in [
    '.beads/vb-jpq7.3/proof-obligations.planned.jsonl',
    '.beads/vb-jpq7.3/verifier-lane-decisions.jsonl',
    '.beads/vb-jpq7.3/waiver-candidates.jsonl',
    '.beads/vb-jpq7.3/verification-ledger.jsonl',
    '.beads/vb-jpq7.3/verifier-lane-review.jsonl',
]:
    for line_no, line in enumerate(pathlib.Path(path).read_text().splitlines(), 1):
        if line.strip():
            json.loads(line)
print('JSONL parse PASS')
PY
```

Then run the canonical required-field schema validator used by the proof-plan reviewer, not only a parse check.
