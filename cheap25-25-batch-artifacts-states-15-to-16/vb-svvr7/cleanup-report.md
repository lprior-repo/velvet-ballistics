# cleanup-report.md — vb-svvr7

> State 15/16 (cleanup + terminal state update) report combined
> for the IPC CLI postcard trailing-bytes rejection guard bead.

- bead_id: `vb-svvr7`
- bead_title: IPC: reject trailing bytes in CLI postcard frame decoder
- type: `bug`
- priority: `P1`
- phase: 15/16
- controller: femdation
- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7`
- jj_workspace: `cheap25-vb-svvr7`
- produced_at: 2026-07-02

## STATUS: COMPLETE_WITH_WORKSPACE_PRESERVED

Landing evidence exists in `landing-report.md`. The bead is closed
and `bd dolt push` completed cleanly. The source-checkout guard
holds: no canonical `.beads/vb-svvr7/` artifacts were written in
`/home/lewis/src/velvet-ballistics`; that checkout was used only
for the `bd close` and `bd dolt push` lifecycle commands because
the bead Dolt server is reached via the coord-checkout path.

## Verified landing evidence

- `.beads/vb-svvr7/landing-report.md` exists and documents the
  full State 15 evidence: production-code diff, master-contract
  compliance, final quality gates, formal-verification, bead
  close, and Dolt push.
- `bd close vb-svvr7 --reason "..."` returned
  `✓ Closed vb-svvr7 — IPC: reject trailing bytes in CLI postcard
  frame decoder: ...` from the source checkout.
- `bd dolt push` returned `Pushing to Dolt remote...` →
  `Push complete.` from the source checkout.
- `bd show vb-svvr7` confirms `[● P1 · CLOSED]` with the documented
  close reason.
- 21 cli_postcard tests pass on the isolated workspace at the
  `lrutlkzunmkq ca97a6023b45` commit (4 new + 17 existing).
- 540 vb_ipc parity tests pass.
- `cargo clippy --workspace --all-features -- -D warnings` exits 0
  with 0 warnings.
- `bash scripts/check-panic-surface.sh` exits 0 with
  `NoViolationFound`.
- `bash scripts/check-ignored-fallible-results.sh` exits 0.
- All four reviewer artifacts carry `STATUS: APPROVED`:
  - `.beads/vb-svvr7/formal-verification-report.md`
  - `.beads/vb-svvr7/black-hat-review.md` (defects.md empty)
  - `.beads/vb-svvr7/truth-serum-report.md`
  - `.beads/vb-svvr7/final-evidence-decision.md`
- 3/4 proof-obligation rows in
  `.beads/vb-svvr7/verification-ledger.jsonl` carry
  `result: PASS`; the 4th row is `result: BLOCKED_TOOLING` for
  PO-TB-PROP-01 (proptest not wired — waived as
  WVR-TB-01-PROPTEST-WIRING in formal-waivers.jsonl, with
  compensating unit-test boundary coverage at PO-TB-UNIT-01).

## Ledger integrity

The agent-invocation ledger
(`.beads/vb-svvr7/agent-invocation-ledger.jsonl`) has been
appended with two new rows covering the p15-16 combined
landing+cleanup phase:

| sequence | state | skill | invocation_id | parent | status |
|---|---|---|---|---|---|
| 8 | 15 | landing-skill | `landing-skill-vb-svvr7-state15` | `evidence-packaging-vb-svvr7-state14` (sequence 7) | completed |
| 9 | 16 | landing-skill | `landing-skill-vb-svvr7-state16` | `landing-skill-vb-svvr7-state15` (sequence 8) | completed |

Each new row carries the schema-version, ledger-sequence
(monotonic), previous-entry-hash (chains to the prior tip),
entry-hash (SHA-256 over canonicalised JSON), input-artifact-
hashes (over the existing reviewer artifacts and
implementation.md at `lrutlkzunmkq ca97a6023b45`), and the
output-artifact-hashes for `landing-report.md`, `cleanup-report.md`,
and the final `STATE.md`. Validation: each `previous_entry_hash`
matches the actual preceding row's `entry_hash`; each new row's
`entry_hash` matches a local SHA-256 of the canonicalised JSON
payload; no skipped sequences.

## Workspace cleanup decision

The cheap25-vb-svvr7 JJ workspace
(`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7`)
is intentionally preserved for now instead of being removed
via `jj workspace forget cheap25-vb-svvr7`, because:

1. The parent cheap25 dispatch orchestrator integrates the
   accepted p11 fix into the shared dispatch bookmark chain at
   `cheap25/vb-pg2wq-holzman` and runs the full cheap25 batch's
   `moon ci` integration sweep across all 25 P0/P1 beads. It is
   not safe to discard the workspace until that sweep has
   completed and the parent dispatch orchestrator has explicitly
   chosen to retire the workspace.
2. The bead-local `.beads/vb-svvr7/` artifact tree
   (50+ evidence files) lives inside the isolated workspace;
   retiring the workspace without first copying the artifacts to
   a durable host would silently destroy the bead's
   evidence-of-record.
3. The bead closure + Dolt push has succeeded and there is no
   pending undo operation; preserving the workspace is a
   pure-readiness measure for the parent's cheap25 batch
   integration sweep. No write-side state on this workspace is
   "in flight" beyond what the bead-lifecycle already recorded.

After the parent cheap25 batch integration sweep completes, the
parent orchestrator will retire the
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7`
worktree / JJ workspace with the standard
`jj workspace forget cheap25-vb-svvr7 && rm -rf <worktree>`
gesture.

## Source-checkout guard

- Source checkout remains `/home/lewis/src/velvet-ballistics`
  (HEAD detached at `44d0be4af`, clean, no changes).
- Production-code edits live in
  `crates/vb_cli/src/cli_postcard/error.rs`,
  `crates/vb_cli/src/cli_postcard/validation.rs`, and
  `crates/vb_cli/src/cli_postcard/tests.rs` inside the isolated
  workspace at the `lrutlkzunmkq ca97a6023b45` commit.
- `.beads/vb-svvr7/` artifact tree (50+ evidence files plus the
  final `landing-report.md`, `cleanup-report.md`, and updated
  `STATE.md`) lives in the isolated workspace at
  `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7/.beads/vb-svvr7/`.
- Source checkout was used only for `bd show`, `bd close`, and
  `bd dolt push` because the bead Dolt server is reachable from
  the source-checkout path, not from within the JJ workspace's
  bead-storage dir.
- No `git commit`, `git checkout`, `git reset`, `jj new`,
  `jj describe`, `jj edit`, `jj cherry-pick`, or any
  implementation-side mutation was performed from the source
  checkout. All implementation work happened in the isolated
  workspace per `AGENTS.md` workspace-isolation rules.

## Pre-existing FAIL_GLOBAL classifications (honest report)

The `bash scripts/check-production-inner-drift.sh` and
`bash scripts/verify-verus.sh` scripts return
`FAIL_GLOBAL` against the workspace as a whole. Both failures
are pre-existing, unrelated to vb-svvr7's call-graph blast
radius, and honestly reported in the
`.beads/vb-svvr7/final-evidence-decision.md` and
`.beads/vb-svvr7/black-hat-review.md` files. Neither constitutes
a blocker per the formal-verifier skill rule "Existing
unrelated global failures: classify honestly". The blast-radius
drift count for vb-svvr7 is zero; no Verus or Flux obligations
are affected; the cli_postcard module is not gated by any
verifier lane.

## Terminal state

State 15/16 combined is complete. No pending gate remains for
bead `vb-svvr7`. The bead is closed. The Dolt tracker is in
sync with the remote. The bead-local evidence tree lives in the
isolated workspace until the parent cheap25 batch integration
sweep is finished.

The next-person-up work is owned by the femdation parent
dispatch orchestrator and is out of scope for this bead landing.
