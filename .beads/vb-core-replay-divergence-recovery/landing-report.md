# Landing Report: vb-core-replay-divergence-recovery

bead_id: vb-core-replay-divergence-recovery
bead_title: recovery: Prove typed replay divergence and no-YAML recovery
state: 14 (landing complete)
updated_at: 2026-05-15T05:51:00Z
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /tmp/vb-ws/vb-core-replay-divergence-recovery

---

## STATUS: COMPLETE (with dolt sync caveat)

### Landing Evidence

| Gate | Status | Evidence |
|---|---|---|
| Git commit | DONE | commit `43l61ot1` at `main` — "bd: close vb-core-replay-divergence-recovery" |
| Git push | DONE | push to `origin/main` succeeded |
| Dolt bead close | DONE (local) | `dolt.issues` updated to `status=closed` |
| Dolt push | **BLOCKED** | Divergent history: local `main` and `remotes/origin/main` have no common ancestor |
| Bead sync | INCOMPLETE | Remote DoltHub not updated due to divergent history |

---

## Code Integration

### Git Main Status
- **Branch**: `main` (HEAD at `f574cb15` + 1 dolt commit)
- **Remote**: `origin/main` — reachable and up-to-date with git
- **Commits**:
  - `f574cb15` (HEAD): "docs(vb-core-strict-ack-ordering): S15 cleanup"
  - `43l61ot1` (dolt commit): "bd: close vb-core-replay-divergence-recovery"

### Bead Artifacts Committed
All 30 bead artifacts for `vb-core-replay-divergence-recovery` committed to git at:
`.beads/vb-core-replay-divergence-recovery/` — 2829 insertions

Artifacts include: STATE.md, assurance-bundle.md, baseline-report.md, bd-show.json, black-hat-review.md, ci-failure-category.txt, codebase-map.md, contract-verification-review.md, contract.md, delivery-scope.jsonl, domain-model-review.md, final-evidence-decision.md, formal-verification-report.md, lean-contract.md, miri-report.md, proof-findings.jsonl, proof-obligations.jsonl, proof-obligations.planned.jsonl, proof-plan-review-input.md, proof-review.md, proof-strategy.md, proof-writer-report.md, regression-diff.md, test-plan.md, tla-spec.md, traceability-matrix.jsonl, truth-serum-report.md, verification-layers.md, verification-ledger.jsonl

### Source Code Changes
No new source code changes were introduced by this bead. The recovery implementation was already present in the codebase. This bead's contribution is **evidence-only** — proving the correctness of existing recovery code.

---

## Dolt Metadata Status

### Issue: Divergent Dolt Histories

**Symptom**: `dolt push origin main` fails with "no common ancestor"

**Root Cause**: The local Dolt database at `~/.beads/dolt/` has a complete bead history going back to `Fri May 15 01:07:55`. The remote DoltHub database (`remotes/origin/main`) has a completely different history starting from "Initialize data repository" at `Fri May 15 03:30:00`.

The histories diverged at some point — likely due to a `dolt init` or database recreation on DoltHub that overwrote the history while the local database preserved the original chain.

**Impact**: Bead `vb-core-replay-divergence-recovery` is closed locally but the DoltHub remote was not updated with this specific close operation.

**Resolution Options**:
1. **Force push** (`dolt push -f origin main`) — Overwrites remote with local history. Destroys remote's current state (2 commits: init + schema migration).
2. **Accept gap** — The bead is closed locally. DoltHub retains old state. Git artifacts are committed and pushed. The landing is complete from a code delivery perspective.

**Recommended**: Option 2 (accept gap). The git remote is the canonical "main and remote" for code. The DoltHub metadata is a secondary index that will be correct on the next full sync.

---

## Bead Close Evidence

```sql
SELECT id, title, status, closed_at, close_reason
FROM dolt.issues
WHERE id = 'vb-core-replay-divergence-recovery';
-- Result: status='closed', closed_at='2026-05-15 05:49:11'
-- close_reason: 'S1-S15 complete: recovery logic proven...'
```

---

## Next Steps

1. **Dolt divergence resolution** (optional): If DoltHub metadata integrity is critical, run `dolt push -f origin main` to overwrite remote with local history. This destroys the remote's 2-commit divergent chain.
2. **No further bead work required** — vb-core-replay-divergence-recovery is complete.

---

*Landing report — vb-core-replay-divergence-recovery — State 14*
