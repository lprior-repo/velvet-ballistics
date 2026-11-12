# Formal Verification Report — TLC Fix Pass for `vb-rpch`

Status: **NOT CLOSED**. The small smoke model passes and the non-vacuity witness fires as intended, but the primary bounded model did **not** finish. No exhaustive full TLC proof may be claimed from this pass.

Date: 2026-05-24
Agent lane: `formal-verifier`
Scope: `.beads/vb-rpch/proof-obligations.tlc-fix.planned.jsonl`

## Commands executed

### TLC-FIX-005 — smoke model

```bash
tlc -config specs/tla/RecoveryReplayFull-smoke.cfg specs/tla/RecoveryReplayFull.tla
```

Workdir: `/home/lewis/src/vb-jpq7-jj-fix`

Result: **PASS**

Evidence:
- TLC reported: `Model checking completed. No error has been found.`
- States generated: `5,883,676`
- Distinct states: `505,140`
- States left on queue: `0`
- Complete graph search depth: `9`

Classification: `PASS`.

### TLC-FIX-007 — non-vacuity witness model

```bash
tlc -config specs/tla/RecoveryReplayFull-nonvacuity.cfg specs/tla/RecoveryReplayFull.tla
```

Workdir: `/home/lewis/src/vb-jpq7-jj-fix`

Result: **PASS for expected counterexample witness**

Evidence:
- TLC reported: `Invariant NotAllNonVacuityWitnessesReached is violated.`
- This is the expected result for the negated witness invariant.
- Witness reaches `RunAccepted`, `ActionCompleted`, `RunFinished`, `snapshot_seq = 0`, and `recovered_runs = {2}`.
- States generated before witness: `477,860`
- Distinct states before witness: `82,285`
- States left on queue at witness: `55,340`
- Search depth at witness: `6`

Classification: `PASS` for reachability-witness obligation, not a full semantic proof.

### TLC-FIX-008 — source/evidence sync

```bash
cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg
```

Workdir: `/home/lewis/src/vb-jpq7-jj-fix`

Result: **PASS**. Command produced no output and exited successfully.

Classification: `PASS`.

### TLC-FIX-001/002/003/004/006 — primary full bounded model

```bash
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla
```

Workdir: `/home/lewis/src/vb-jpq7-jj-fix`

Result: **FAIL_LOCAL / PARTIAL_BFS**

Raw partial output transcribed to:

- `evidence/specs/RecoveryReplayFull.formal-timeout.tlc.log`

Observed before timeout:
- TLC parsed and semantically processed `RecoveryReplayFull.tla` successfully.
- Initial states computed successfully.
- Last progress line before runner timeout:
  - `34,905,320 states generated`
  - `34,899,577 distinct states found`
  - `34,898,477 states left on queue`
- Runner timeout: `180000 ms`.
- No final `Model checking completed` line exists.
- Queue was not drained.

Classification:
- `TLC-FIX-001`: `PASS` for parser/config-loader acceptance only.
- `TLC-FIX-002`: `FAIL_LOCAL` because full invariant execution did not complete.
- `TLC-FIX-003`: `FAIL_LOCAL` because full invariant execution did not complete.
- `TLC-FIX-004`: `FAIL_LOCAL` because full invariant execution did not complete.
- `TLC-FIX-006`: `FAIL_LOCAL` because primary bounded model timed out with a huge queue.

## Waivers

No waivers accepted in this pass.

## Ruthless closure verdict

- Smoke evidence is good but insufficient.
- Non-vacuity witness evidence is useful but scoped.
- Source/evidence sync is good.
- The primary full model is **not proven**: it timed out with `34,898,477` states still queued.
- `RecoveryErrorExhaustive` remains **partial/pending**; this model names error variants but does not causally reach most of them.
- Existing stale `APPROVED` claims must not be treated as current proof approval until proof-reviewer reviews this evidence.
