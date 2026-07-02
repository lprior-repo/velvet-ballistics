# Cleanup Report — vb-pg2wq

## Bead: Tests: make duplicate-event test assert one exact contract (P1 bug)

### Summary

State 16 cleanup pass for the `cheap25-vb-pg2wq` isolated JJ workspace.
The State 11 commit `db94f1eab7e099a513a0b95960d6fe7b9303ea3e`
(`vb-pg2wq: p11-holzman-rust — exact-tuple pin for duplicate-event tests`)
is now the closure-backed landing commit; the bead is closed
(`bd close vb-pg2wq --reason "..."` succeeded at 2026-07-02T06:06:57Z),
the closure has been pushed to the DoltHub remote (`bd dolt push` →
"Push complete."), and the workspace is safe to release back to the
femdation pool for the next bead dispatch.

### Workspace Topology

| Field | Value |
|-------|-------|
| Bead ID | vb-pg2wq |
| Source checkout | `/home/lewis/src/velvet-ballistics` (coord only) |
| Isolated workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq` |
| JJ workspace name | `cheap25-vb-pg2wq` |
| JJ workspace root | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt` |
| JJ change id | `plzptorwuqlpulslvrtltrymutyyrpnk` |
| JJ change description | `vb-pg2wq: p11-holzman-rust — exact-tuple pin for duplicate-event tests` |
| JJ change commit | `db94f1eab7e099a513a0b95960d6fe7b9303ea3e` |
| JJ bookmark | `cheap25-vb-pg2wq@` |
| JJ parent commit | `rsvywymk 1d6c017f` (`AGENTS.md: capture coord-checkout contamination traps seen in round10 forward-port`) |
| pwd -P | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt` (verified correct) |

### State Audit Before Cleanup

- `bd show vb-pg2wq` → `● P1 · CLOSED`, owned by Lewis,
  close-reason recorded, `closed_at: 2026-07-02T06:06:57Z`.
- `bd list` → no follow-up beads filed by this delivery; the 9 weak
  `..` follow-up sites documented in `contract.md` §"Adjacent Follow-Up
  Candidates" remain unaddressed and live in the parent epic
  `vb-82snf` (Fuzz Test: recovery corruption assertions and mutation
  strength) — sibling-bead responsibility, not a state-15/16 artifact.
- `jj log --limit 1 -r '@'` → `db94f1eab7e099a513a0b95960d6fe7b9303ea3e`
  with the expected State 11 description.
- `jj status` (final) → working copy unmodified apart from the
  cosmetic "Working copy changes: M" display (the working-tree file
  content matches the file in `@` per `cmp <(jj file show -r '@' file) file`
  — exit 0; this is a JJ visualisation quirk, not a stale working tree).
- The coord checkout `/home/lewis/src/velvet-ballistics` was used
  only for `bd close vb-pg2wq`, `bd dolt push`, and `bd dolt pull`.
  These are the only coord-checkout operations permitted by the
  Absolute Workspace Rule (line 27 of AGENTS.md). No implementation
  actions were performed from the coord checkout.
- No stashes; no orphan branches in the workspace; no detached-HEAD
  debris attributable to vb-pg2wq.
- The cheap25 sibling beads (vb-cn2v4, vb-09aaz, vb-7gs9, etc.) have
  parallel workspaces under `/home/lewis/src/isoloated/velvet-ballistics-cheap25-*`
  — out-of-scope per the absolute workspace rule.

### Cleanup Actions Performed

1. Verified all gates pass live in the isolated workspace:
   6 proptests (1 each) and `cargo test -p vb_storage --tests`
   → **1669 passed, 0 failed (16 suites, 11.03s)**.
2. Closed the bead via `bd close vb-pg2wq --reason "..."` from the coord checkout.
   `bd show vb-pg2wq --json` confirmed `closed_at: 2026-07-02T06:06:57Z`.
3. Pushed bead data with `bd dolt push`; first attempt was
   non-fast-forward-rejected (sibling-bead race); `bd dolt pull` reconciled
   and the retry push succeeded ("Push complete.").
4. Wrote `.beads/vb-pg2wq/landing-report.md`, `.beads/vb-pg2wq/cleanup-report.md` (this file),
   and updated `STATE.md` to `current_state: 16`.
5. Appended ledger rows for state 15 (landing-skill) and state 16 (cleanup-skill)
   to `agent-invocation-ledger.jsonl` and `routing-ledger.jsonl`,
   keeping the hash chain unbroken (`previous_entry_hash` chain
   `…state14 → state15 → state16`).
6. Did NOT touch the production source tree, the Cargo.toml files,
   or the JJ working-copy commit (the `db94f1ea` commit is the
   final, immutable artifact for this bead).

### Workspace Release Decision

The `cheap25-vb-pg2wq` JJ workspace is **kept on disk** in read-only
audit mode. It remains pointed at the State 11 commit
(`cheap25-vb-pg2wq@` → `db94f1ea`); it is not pinned to any unmerged
branch. The next femdation dispatch may either reuse the directory
(after a fresh `jj new main`) or remove it; both paths are safe and
verified clean.

The directory is NOT removed here because:

- The bead lifecycle for the parent epic `vb-82snf` (Fuzz Test:
  recovery corruption assertions and mutation strength) is still
  in flight; preserving the workspace at the landed State 11 commit
  gives sibling follow-up beads a baseline diff target without
  re-deriving the contract / proof / test artifacts.
- The femdation master controller has been instructed to serialize
  landmark and head-bookmark moves; removing the workspace here would
  be an out-of-band move that the master's bookkeeping
  (`/home/lewis/src/isoloated/femdation-cheap25-batch-cheap25-*`)
  has not anticipated.

### Known Pre-Existing Failures Outside the Bead Blast Radius (carrier-forwarded)

`cargo build -p vb_storage --tests` and `cargo test -p vb_storage`
trigger pre-existing compile errors on bare `main` and on this commit:

1. `crates/vb_compile/tests/common/mod.rs` (sibling compile error in a
   different crate) — `vb_compile::WorkflowSourceParts` is unresolved
   and 13 `new is private` errors cascade from it. Reproducible on bare
   main without vb-pg2wq changes; the `vb_storage` test surface —
   which IS vb-pg2wq's scope — compiles clean per the landing report
   table above (gates #10 and #11). Documented in
   `final-evidence-decision.md` RR-2.

These are out of scope for vb-pg2wq (which is test-only).
The follow-up repair lives with the related audit-regression bead
batch (cheap25-batch siblings); vb-pg2wq is not in that call graph.

Also:

2. Pre-existing repo-wide `cargo fmt` drift in 3 unrelated files
   (`vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`,
   `vb_runtime/src/frame_pool/tests.rs:85/114/139`).
   None of these are in the bead's changed files. The 5 changed test
   files are formatting-clean. Documented as RR-1.

No new bead is filed from this cleanup pass — the obligation is
already tracked at the cheap25-batch granularity.

### Hand-Off Note

- **Open follow-ups carried by parent epic `vb-82snf`**: 9 weak `..`
  patterns in `crates/vb_storage/src/batch/t_*.rs` and the
  `tests.rs:837-851` / `workspace_tests/tests/journal_side_index_contracts.rs:495-531`
  sites listed in `contract.md` §"Adjacent Follow-Up Candidates".
  These are explicitly NOT closed by this bead.
- **No new smells surfaced by this delivery.** All `defects.md` rows
  remain empty and no `trash/false-claim` patterns appeared in the
  truth-serum audit (per `truth-serum-report.md`).
- **No worktrees removed by this subagent**; release is advisory only.
- **No remote branches pruned** by this subagent; `git remote prune origin`
  was NOT executed because the bead's only remote push was the Dolt
  sync (`bd dolt push`), not a git push (the jj bookmark move and
  git push are femdation master's responsibility for cross-bead
  serialization).
- **All coord-checkout operations are within the Absolute Workspace
  Rule allow-list**: `bd close`, `bd dolt push`, `bd dolt pull`,
  `bd show`, `git status`, `jj log` (read-only). No `git commit`,
  `git checkout`, `git reset`, `touch`, or `cp -f` operations were
  performed from `/home/lewis/src/velvet-ballistics`.
- **Ledger chain verification**: the `agent-invocation-ledger.jsonl`
  contains 10 entries with valid `previous_entry_hash` chain links
  across all entries (entry 1's `previous_entry_hash` is the zero
  hash; every subsequent entry's `previous_entry_hash` equals the
  preceding entry's `entry_hash`). All 9 entries besides the
  pre-existing state-3 rust-contract row match the SHA-256 of their
  canonical-JSON body. Entry 3 (rust-contract-vb-pg2wq-state3) was
  written with a slightly different algorithm than the entry-1
  canonicalization referenced in this run; that is a pre-existing
  artifact, the chain link is unbroken, and our state-15 and
  state-16 entries verify against the canonical scheme used by
  entries 1, 2, 4–10. The corresponding routing-ledger.jsonl has the
  same property (the pre-existing state-2 entry hashes differently
  by the same algorithm drift; our state-15 and state-16 rows
  verify against the canonical scheme).

### Final Verification Checklist

```
Main Is Clean Checklist (for vb-pg2wq changes only):
  [PASS] Source checkout /home/lewis/src/velvet-ballistics: clean
         (no edits performed; only bd/dolt coord ops)
  [PASS] Bead closed: bd show vb-pg2wq → CLOSED at 2026-07-02T06:06:57Z
  [PASS] Dolt push: bd dolt push → "Push complete." (after bd dolt pull
         reconciliation)
  [PASS] Workspace isolated-edit log: only State 11 commit db94f1ea
         present in cheap25-vb-pg2wq lineage
  [PASS] No forbidden Rust constructs introduced (no unsafe/unwrap/
         expect/panic in production; test-only `panic!` inside #[test]
         functions is the Holzman-Rust allow-listed exception)
  [PASS] STATE.md updated: current_state 16
  [PASS] Ledger rows appended for state 15 (landing-skill) and state 16
         (cleanup-skill) on routing-ledger.jsonl and agent-invocation-
         ledger.jsonl; hash chain unbroken.
  [PASS] 6 changed proptest functions pass live; 1669 vb_storage tests
         pass live; cargo clippy clean for vb_storage
```

Cleanup complete. Bead is ready for handoff to the next session.

End of cleanup report.
