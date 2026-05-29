# State 4 Replan Required Report — vb-t6hx

owner_state: State 4 proof-planner  
rerun_from: State 4  
written_by: proof-writer  
scope: State 5 repair triage only; no production source, verifier harness, proof obligation, or lane-decision edits made.

## Decision

`E_LANE_OBLIGATION_MISMATCH` is present in both State 5 and State 6 validators. The mismatches are between State 4-owned `verifier-lane-decisions.jsonl` records and State 4-owned `proof-obligations.planned.jsonl` records. Repairing this requires re-planning or canonical contract-clause normalization by the proof-planner/reviewer lane, not proof-writer artifact repair.

Per dispatch instruction, I did not patch around the mismatch and did not edit lane decisions or planned obligations.

## Commands Run

1. `python /home/lewis/.opencode/skill/go-skill/tools/go-skill-v9-validate --workspace /home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx --bead vb-t6hx --state 5 --source-checkout /home/lewis/src/velvet-ballistics --format json`
   - Exit status: 1
   - Result: FAIL

2. `python /home/lewis/.opencode/skill/go-skill/tools/go-skill-v9-validate --workspace /home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx --bead vb-t6hx --state 6 --source-checkout /home/lewis/src/velvet-ballistics --format json`
   - Exit status: 1
   - Result: FAIL

3. `python - <<'PY' ... lane/obligation mismatch summarizer ... PY`
   - Exit status: 0
   - Result: 37 obligation-clause mismatches across 36 lane-decision lines.

4. `python - <<'PY' ... run State 5 and State 6 validators and write raw outputs ... PY`
   - Exit status: 0
   - Result: wrote exact raw validator stdout and exit status files:
     - `.beads/vb-t6hx/state5-validator-current-raw.json`
     - `.beads/vb-t6hx/state5-validator-current-exit.txt`
     - `.beads/vb-t6hx/state6-validator-current-raw.json`
     - `.beads/vb-t6hx/state6-validator-current-exit.txt`

## Exact Mismatch Summary

Validator reports `E_LANE_OBLIGATION_MISMATCH` for every required planned obligation `PO-vb-t6hx-001` through `PO-vb-t6hx-037`. The lane decisions carry expanded/capitalized contract clauses, while the planned obligations carry shorter clauses.

| Requirement | Affected obligations | Lane decision contract clause | Planned obligation contract clause |
| --- | --- | --- | --- |
| REQ-08 / Functional Contract 4 | PO-vb-t6hx-001..006 | `Functional Contract 4: Storage scan/get uses a read-only capability and must not append events, write tests, create synthetic run IDs, delete keys, compact, or migrate records.` | `Functional Contract 4: read-only storage scan/get must not mutate records or keys.` |
| REQ-02 / Functional Contract 5 | PO-vb-t6hx-007..012 | `Functional Contract 5: Scan emits at most the requested ScanLimit rows.` | `Functional Contract 5: scan emits at most requested ScanLimit rows.` |
| REQ-04 / Functional Contract 3 | PO-vb-t6hx-013..017 | `Functional Contract 3: Invalid keyspace, invalid hex key, invalid numeric limit/filter, and conflicting flags fail before opening storage.` | `Functional Contract 3: invalid hex key fails before opening storage.` |
| REQ-09 / Functional Contract 10 | PO-vb-t6hx-018..025 | `Functional Contract 10: Envelope decode validates header length, magic, schema, record kind family, payload length bound, header CRC, payload availability, and payload digest before Postcard decode.` | `Functional Contract 10: envelope decode validates length/integrity before Postcard decode.` |
| REQ-01 / Functional Contract 9 | PO-vb-t6hx-026..031 | `Functional Contract 9: Projection scan defaults to skip-decode and must not Postcard-decode every value.` | `Functional Contract 9: projection scan defaults to skip-decode.` |
| REQ-06 / Functional Contract 7 | PO-vb-t6hx-032..036 | `Functional Contract 7: Large values render as bounded previews with explicit truncation metadata and a hint to use raw get or larger bounded preview.` | `Functional Contract 7: large values render as bounded previews with truncation metadata and hint.` |
| REQ-10 / Non-functional | PO-vb-t6hx-037 | `Non-Functional Contract: Cold CLI formatting may use diagnostic serialization, but vb_core, vb_runtime, vb_storage runtime hot paths, and vb_ipc must not gain JSON/YAML/HTTP behavior.` | `Non-Functional Contract: doctor formatting stays outside runtime core/hot paths and no JSON/YAML/HTTP behavior is added to runtime core.` |

## Additional Validator Blockers Observed

These are not safe proof-writer repairs while the State 4 mismatch exists, and some belong to State 6 review provenance rather than State 5 proof artifacts:

- `E_INVOCATION_LEDGER_FORGED` on `agent-invocation-ledger.jsonl` lines 9, 10, 14, and 16: `transcript-state6-proof-reviewer.md` transcript/artifact hash mismatch.
- `E_REVIEW_PROVENANCE_MISSING` on active `proof-review.md`: missing `reviewer_skill` or `reviewer_invocation_id` header.
- `E_STATUS_NOT_APPROVED` on active `proof-review.md`: status token is `REJECTED`.

I did not manufacture approval, alter reviewer provenance, or normalize the invocation ledger around a rejected State 6 review.

## Raw Validator Output Summary

Exact raw validator stdout files from this proof-writer triage are attached as:

- `.beads/vb-t6hx/state5-validator-current-raw.json` with exit status file `.beads/vb-t6hx/state5-validator-current-exit.txt` (`1`).
- `.beads/vb-t6hx/state6-validator-current-raw.json` with exit status file `.beads/vb-t6hx/state6-validator-current-exit.txt` (`1`).

State 5 raw validator status:

```json
{"bead":"vb-t6hx","state":5,"status":"FAIL","finding_codes":["E_INVOCATION_LEDGER_FORGED","E_REVIEW_PROVENANCE_MISSING","E_STATUS_NOT_APPROVED","E_LANE_OBLIGATION_MISMATCH"],"lane_mismatch_obligations":"PO-vb-t6hx-001..PO-vb-t6hx-037","additional_paths":["agent-invocation-ledger.jsonl","proof-review.md"]}
```

State 6 raw validator status:

```json
{"bead":"vb-t6hx","state":6,"status":"FAIL","finding_codes":["E_INVOCATION_LEDGER_FORGED","E_REVIEW_PROVENANCE_MISSING","E_STATUS_NOT_APPROVED","E_LANE_OBLIGATION_MISMATCH"],"lane_mismatch_obligations":"PO-vb-t6hx-001..PO-vb-t6hx-037","additional_paths":["agent-invocation-ledger.jsonl","proof-review.md"]}
```

Exact validator finding messages observed for the lane mismatch are of the form:

```text
E_LANE_OBLIGATION_MISMATCH verifier-lane-decisions.jsonl:<line> obligation PO-vb-t6hx-<id> contract_clause mismatch
```

The State 5 and State 6 validators emitted this for decision lines 1, 2, 3, 4, 5, 7, 9, 10, 11, 12, 15, 16, 18, 19, 20, 23, 24, 25, 26, 27, 28, 30, 31, 32 twice, 33, 34, 35, 36, 39, 40, 42, 43, 44, 47, 48, and 50.

## Required Next Dispatch

Dispatch State 4 proof-planner/proof-plan-reviewer to reconcile `verifier-lane-decisions.jsonl` and `proof-obligations.planned.jsonl` contract clauses and obligation mappings. After State 4 validates cleanly, rerun State 5 proof-writer only if proof-owned evidence/report artifacts still fail validation.
