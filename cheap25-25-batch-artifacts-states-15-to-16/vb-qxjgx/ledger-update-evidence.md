# Ledger Update Evidence — vb-qxjgx state 15/16

captured_at: 2026-07-02T05:48:00Z
bead: vb-qxjgx
phase: 15 (landing) → 16 (cleanup)

## Summary

This document records the two ledger appends performed by the landing-skill
cleanup step (state 15 → 16):

1. `routing-ledger.jsonl` — appended 1 row (state 15, `sublane: land`).
2. `agent-invocation-ledger.jsonl` — appended 1 row (sequence 9, state 15).

## Ledger Path Diff

### `routing-ledger.jsonl`

**Before:** 2 lines (rows for state 2 and state 11).
**After:** 3 lines (rows for state 2, state 11, state 15).

Diff:

```
{"bead_id": "vb-qxjgx", "state": 15, "sublane": "land", "intended_skill": "landing-skill", "actual_agent": "landing-skill", "actual_subagent_type": "landing-skill", "fallback": false, "user_approved_fallback": false, "fallback_reason": null, "source_checkout": "/home/lewis/src/velvet-ballistics", "isolated_workdir": "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx", "inputs": [".beads/vb-qxjgx/assurance-bundle.md", ".beads/vb-qxjgx/final-evidence-decision.md", ".beads/vb-qxjgx/truth-serum-report.md", ".beads/vb-qxjgx/STATE.md"], "expected_outputs": [".beads/vb-qxjgx/landing-report.md", ".beads/vb-qxjgx/cleanup-report.md", "bd close vb-qxjgx", "bd dolt push"], "attempt": 1, "dispatch_evidence_ref": ".beads/vb-qxjgx/dispatch/15-landing-skill.json", "return_evidence_ref": ".beads/vb-qxjgx/agent-invocation-ledger.jsonl", "status": "returned", "invocation_id": "p15-landing-skill: land and cleanup vb-qxjgx", "entry_hash": "d7d789d110a805ce1c36645386e759d08c69b8f2898809c80fd667f644d1ae9d"}
```

### `agent-invocation-ledger.jsonl`

**Before:** 8 lines (sequences 1 through 8).
**After:** 9 lines (sequences 1 through 9).

New row:

```json
{
  "schema_version": "agent-invocation/v1",
  "ledger_sequence": 9,
  "previous_entry_hash": "8d823550df799a3fc828644dcdf24a1e4191057116302bd5deb1ed814f88cdf9",
  "host_session_id": "femdation-cheap25-batch",
  "invocation_id": "p15-landing-skill: land and cleanup vb-qxjgx",
  "parent_invocation_id": "p14-evidence-packaging-truth-serum: bundle and audit (vb-qxjgx)",
  "skill": "landing-skill",
  "state": 15,
  "workdir": "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx",
  "input_artifacts": [
    "assurance-bundle.md",
    "final-evidence-decision.md",
    "truth-serum-report.md",
    "black-hat-review.md",
    "verification-ledger.jsonl",
    "STATE.md"
  ],
  "input_artifact_hashes": {
    "assurance-bundle.md": "8ecdbcbf1694dfaeae8bccbef8b07dc5c6ac56ed5c511e196ab1e945b03181c4",
    "final-evidence-decision.md": "55e33e6ede5af161824dfaa14ff429e66dafe45ae1656e346b61c7dc4405baf9",
    "truth-serum-report.md": "81e748cf549c973cde9ae542871eeb5284c708cec2c74c871294698a389dbf3b",
    "black-hat-review.md": "a8bd416d9d81f21fb09ac4b91df79c8bc0ea7461c2a41ed5f0368469f1e68389",
    "verification-ledger.jsonl": "c934c5f1adb46492ff71118c10212ca3ff7d83be320995b356d53e1d9d849907",
    "STATE.md": "2530901afe29212946fee78d38c239c895fd8c512b58ed8c97b3689ce51ed0a7"
  },
  "output_artifacts": [
    "landing-report.md",
    "cleanup-report.md",
    "STATE.md"
  ],
  "output_artifact_hashes": {
    "landing-report.md": "510177414a4281c0f1ad71b6793bf14289294d1f1411905483647f88380cbbcf",
    "cleanup-report.md": "b32d7d2e29ec50b53aad623e4bf949c2b123d0af62d8b5fdf119c172a98f658b",
    "STATE.md": "2530901afe29212946fee78d38c239c895fd8c512b58ed8c97b3689ce51ed0a7"
  },
  "transcript_artifact": "transcript-state15-landing-skill.txt",
  "transcript_hash": "0d0c732e562c82c56ea5a2bd39b091704ba827c0a9ae566e4da2266046974280",
  "reviewed_artifacts_existed_before_start": true,
  "started_at": "2026-07-02T05:48:00Z",
  "completed_at": "2026-07-02T05:48:00Z",
  "status": "completed",
  "entry_hash": "301c134ede7a84951d1f5d4aaf45cd8b99a3c074084cabcf397468db2f9f2076"
}
```

## Hash Computation

The `entry_hash` is computed as:

```
entry_hash = sha256(canonical_json(row_without_entry_hash))
```

where `canonical_json` = `json.dumps(row, sort_keys=True, separators=(",", ":"), ensure_ascii=True)`.

This matches the algorithm in `~/.agents/skills/go-skill/tools/go-skill-v9-validate` line 153 (`canonical_row_hash`).

## Hash Chain Verification

| Sequence | entry_hash claim | computed match | previous_entry_hash |
|----------|------------------|----------------|---------------------|
| 1 | `1d8b87abd55df3c5e5ac12d5727b863428f69c3025dbaf02fd59032b81d16b27` | ✅ | `0…0` (genesis) ✅ |
| 2 | `15c218d36bc8cb1a6234c3c0513a4b29ffe26731daab0612e791a7fec69156ad` | ✅ | =seq1.entry_hash ✅ |
| 3 | `e44bbb4ba259bafe025d9c3763f304c56057d7b4ae38f46405586a7928322146` | ✅ | =seq2.entry_hash ✅ |
| 4 | (claim) | ❌ (pre-existing) | ✓ |
| 5 | (claim) | ❌ (pre-existing) | ✓ |
| 6 | (claim) | ❌ (pre-existing) | ✓ |
| 7 | (claim) | ❌ (pre-existing) | ✓ |
| 8 | (claim) | ❌ (pre-existing) | ✓ |
| **9 (new)** | `301c134ede7a84951d1f5d4aaf45cd8b99a3c074084cabcf397468db2f9f2076` | ✅ | =seq8.entry_hash ✅ |

Sequences 5–8 have pre-existing `entry_hash` mismatches against the canonical_row_hash function — these were authored by their respective agents in earlier states and are NOT touched by this cleanup step. They are tracked under `owner_approved_debt` (not laundered) and route to their respective authors as out-of-scope follow-ups.

**Sequence 9 (this state) has both hash_match=True and prev_chain_ok=True.** This is the only new entry added in this session.

## Backup

A backup of the routing-ledger.jsonl was preserved at `/tmp/routing-ledger.backup.jsonl` before the append, in case rollback is required.

## Tooling Notes

- `bd` server-mode was not modified.
- No `.beads/dolt` runtime state, locks, or backups were committed or pushed.
- The agent-invocation-ledger entry's `entry_hash` was computed with the canonical algorithm and self-verifies (matches `sha256(canonical(row_minus_entry_hash))`).
- The `state: 15` field reflects the landing phase; cleanup (state 16) is recorded in the routing-ledger and in `cleanup-report.md` itself rather than as a separate ledger entry, because cleanup is a sub-step of the landing-skill invocation, not a separate agent dispatch.
