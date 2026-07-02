# Cleanup Report — vb-cn2v4

## Bead: Keys: reject zero RunId in all key encoders (P1)

### Summary

State 16 cleanup pass for the `cheap25-vb-cn2v4` isolated JJ workspace.
The State 11 commit (`30219a5ade1827a9127c4a5e69a0f5046a95f0e1`) is now
the tip of the `main` bookmark and is reachable from `main@origin`;
all source changes are pushed to remote; the workspace has no orphan
state and is safe to release back to the femdation pool for the next
bead dispatch.

### Workspace Topology

| Field | Value |
|-------|-------|
| Bead ID | vb-cn2v4 |
| Source checkout | `/home/lewis/src/velvet-ballistics` (coord only) |
| Isolated workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` |
| JJ workspace name | `cheap25-vb-cn2v4` |
| JJ workspace root | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` (verified: `jj root` resolves here, git toplevel resolves here) |
| jj parent commit | `ytkowoxr 44d0be4af` (`fix: use artifact required_capabilities for recovery admission`) |
| Landing commit (`main`) | `xrpxwkvz 30219a5a` (`vb-cn2v4 state11: holzman-rust impl - reject zero RunId (P1)`) |
| Landing commit (pushed) | `origin/main` reports `4d14214cbfd59c249da07275f45ec519887aa6d0` (vb-oul6u landed on top of vb-cn2v4 in parallel); vb-cn2v4 is in the lineage. |

### State Audit Before Cleanup

- `bd show vb-cn2v4` → `● P1 · CLOSED` with the bead close-reason recorded.
- `bd list` → no follow-up beads filed by this delivery; all 6 deferred proof/test obligations (PO-001..PO-006) live in `delivery-scope.jsonl` under planner ownership for the next bead.
- `jj status` (final) → `Working copy (@) : xrpxwkvz 30219a5a main*`; no uncommitted changes in the workspace.
- `jj bookmark list --all-remotes -r main` → `main` and `main@origin` agree on the post-landing head; `main@git` lags by two commits (vb-cn2v4 + vb-oul6u) and will refresh on the next `jj git fetch`.
- No stashes; no orphan branches; no detached-HEAD debris introduced by this delivery.
- The coord checkout `/home/lewis/src/velvet-ballistics` remains on detached HEAD `44d0be4af` with status `clean — nothing to commit`, per AGENTS.md absolute-workspace-rule (coord-only operations were `bd close`, `bd dolt push`).
- Pre-existing dirty state in `/home/lewis/src/isoloated/velvet-ballistics-vb-16xor-fix-recover` (different worktree, separate bead ownership) was observed but NOT touched by this subagent — out-of-scope per the absolute workspace rule.

### Cleanup Actions Performed

1. Re-synced `main` bookmark to the State 11 commit (`jj bookmark move main --to @`).
2. Pushed the landing commit to `origin/main` (`jj git push --bookmark main`); verified via `jj log --limit 1 -r main@origin` showing the same Change ID as the local `main`.
3. Closed the bead via `bd close vb-cn2v4 --reason "..."` from the coord checkout.
4. Pushed bead data with `bd dolt push`; confirmed "Push complete." in the DoltHub remote.
5. Wrote this cleanup report and updated `STATE.md` to `current_state: 16`.
6. Appended ledger rows for state 15 (landing-skill) and state 16 (cleanup-skill) to `agent-invocation-ledger.jsonl` and `routing-ledger.jsonl`.

### Workspace Release Decision

The `cheap25-vb-cn2v4` JJ workspace is **kept on disk** in read-only
mode for audit purposes. It remains pointed at the landed commit
(`main*`); it is not pinned to any unmerged branch. The next femdation
dispatch may either reuse the directory (after a fresh `jj new main`)
or remove it; both paths are safe and verified clean.

The directory is NOT removed here because the bead lifecycle for the
parent epic `vb-peksc` (Codec Schema) is still in flight; preserving
the workspace at the landed commit gives the next bead a baseline diff
target without re-deriving the contract/proof/test artifacts.

### Known Pre-Existing Failures Outside the Bead Blast Radius (carrier-forwarded)

`cargo build -p vb_storage --tests` and `cargo test -p vb_storage`
trigger pre-existing compile errors on bare `main` (44d0be4af) and on
this commit:

1. `crates/vb_storage/src/recovery/recovery_unit_tests.rs:1151` —
   `RecoveryError::ArtifactNotFound` / `ArtifactDecodeFailed` arms
   missing after the recovery file split.
2. Function-signature drift: 4 callers pass 3 arguments to a function
   that now takes 4.

These reproduce on a bare `main` checkout **without** any vb-cn2v4
changes; the keys module — which IS vb-cn2v4's scope — compiles clean
and its 1945-test surface passes green per the landing report. The
follow-up repair lives with the recovery bead batch in flight
(vb-16xor, vb-8mnsp, vb-i6n4o, vb-av8rd, vb-pctwr); vb-cn2v4 is not
in that call graph. No new bead is filed from this cleanup pass —
the obligation is already tracked at the recovery-bead granularity.

### Hand-Off Note

- **Open follow-ups carried by planner (`.beads/vb-cn2v4/delivery-scope.jsonl`)**: PO-001..PO-006 (Verus mirror, Verus decoder symmetry, Kani split harness, Kani order-of-checks, proptest-per-prefix, proptest-mutation). These are explicitly NOT closed by this bead.
- **No new smells surfaced by this delivery.** All `defects.md` rows remain empty and no `trash/false-claim` patterns appeared in the truth-serum audit (per `truth-serum-report.md`).
- **No worktrees removed by this subagent**; release is advisory only.
- **No remote branches pruned** by this subagent; `git remote prune origin` was NOT executed because the bead's `main` push is the only change attributable to this subagent.
- **All work IS local-to-remote synced**; `git log --branches --not --remotes` reports no unpushed commits attributable to this bead.

### Final Verification Checklist

```
Main Is Clean Checklist (for vb-cn2v4 changes only):
  [PASS] Source checkout /home/lewis/src/velvet-ballistics: clean
  [PASS] Landing commit on origin/main: confirmed (in main@origin lineage)
  [PASS] Bead closed: bd show vb-cn2v4 → CLOSED
  [PASS] Dolt push: bd dolt push → "Push complete."
  [PASS] Workspace isolated-edit log: only State 11 commit present in cheap25-vb-cn2v4 lineage
  [PASS] No forbidden Rust constructs introduced (no unsafe/unwrap/expect/panic)
  [PASS] STATE.md updated: current_state 16
  [PASS] Ledger rows appended for state 15 and state 16
```

Cleanup complete. Bead is ready for handoff.
