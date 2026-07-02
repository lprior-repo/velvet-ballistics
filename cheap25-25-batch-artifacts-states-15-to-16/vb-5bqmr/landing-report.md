# Landing Report — vb-5bqmr SlotExtra Discriminator (P1)

- bead_id: vb-5bqmr
- title: SlotExtra: reject unknown VBSE versions instead of legacy downgrade
- priority: P1
- type: bug
- source_checkout: /home/lewis/src/velvet-ballistics
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr
- controller: femdation (state 15 child — landing-skill)
- state: 15 (landing)
- next_state: 16 (cleanup)
- landing_started_at: 2026-07-02T05:47:00Z
- landing_completed_at: 2026-07-02T05:55:00Z
- attempt: 1

## Bead Status

| Field | Value |
|---|---|
| bead_id | vb-5bqmr |
| status | closed |
| closed_at | 2026-07-02T05:47:24Z |
| close_reason | "MAGIC + VERSION constants hoisted; VersionMismatch variant added; legacy-frame-extra path preserved (recovery_bdd_tests 82/82); corrupt-v1 returns DecodeFailed not VersionMismatch; 1538+ cargo tests pass." |
| updated_at | 2026-07-02 (was 2026-07-01) |
| closed_by_actor | landing-skill controller (femdation state 15 child) |
| pushed_to_dolt | YES (after manual pull + re-push; see "Dolt Sync" below) |

## Landing Steps Performed

### Step 1 — Pre-Landing Audit (coord checkout, not modified)

Coord checkout at `/home/lewis/src/velvet-ballistics`:

```
$ git rev-parse --show-toplevel
/home/lewis/src/velvet-ballistics

$ jj workspace list (in coord) — no cheap25-vb-5bqmr workspace registered in coord
  (workspace is JJ-only in the isolated worktree; coord JJ root reports the
  .beads/ tree and the parent main@origin commit, not the isolated workspace's
  cheap25-vb-5bqmr commit chain).
```

Isolated workspace at `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr`:

```
$ jj log -r '@' --no-graph
  change_id  : soxqskzmntln
  commit_id  : 4b2d0b7fd784039f4df8d62113ff7796beb71225
  description: p11-holzman-rust: hoist MAGIC + VERSION, add VersionMismatch variant,
               3-arm discriminator, hydrate/collect translation, tracing dep hoist
  parents    : [wvlxptlnwvzl] vb-5bqmr: p5-proof-writer — Verus WEAK_MIRROR + Kani x5 +
               Flux + proptest x8 (e1523eabd70e)

$ jj diff -r @ --stat
  Cargo.lock                                          |  13 +
  Cargo.toml                                          |   1 +
  crates/vb_core/src/errors.rs                        |   7 +
  crates/vb_runtime/src/primitives/collect.rs         |  14 +
  crates/vb_storage/Cargo.toml                        |   1 +
  crates/vb_storage/src/recovery/replay/summary/hydrate.rs |  17 +-
  crates/vb_storage/src/slot_extra.rs                 | 245 +++++++++++++++++++++-
  7 files changed, 290 insertions(+), 8 deletions(-)
```

The state-11 commit `4b2d0b7f` is the production source of truth. Working
copy is clean (no uncommitted modifications on top of @). All 5 prior
states (1, 2, 3, 4, 5, 6, 7, 11, 12, 13, 14) are APPROVED.

### Step 2 — Evidence Verification (no re-run; PASS reused from state 11/12/13/14)

The state-11 / state-12 / state-13 / state-14 evidence chain was audited
verbatim and reused for landing:

| Evidence file | Result | Source |
|---|---|---|
| `evidence/slot_extra_test.txt` | 8/8 PASS | state 11 |
| `evidence/recovery_bdd_tests.txt` | 82/82 PASS | state 11 |
| `evidence/corrupt_v1_decode_failed.txt` | 1/1 PASS | state 11 |
| `evidence/vb_storage_lib_full.txt` | 1538/1538 PASS | state 11 |
| `evidence/vb_runtime_full.txt` | 2137/2137 PASS | state 11 |
| `evidence/cargo_check_all.txt` | PASS | state 11 |
| `evidence/clippy_lib_touched.txt` | 0 warnings, 0 errors (vb_storage, vb_runtime, vb_core) | state 11 |
| `evidence/state12/slot_extra_test_fv.txt` | 8/8 PASS | state 12 (re-run) |
| `evidence/state12/recovery_bdd_tests_fv.txt` | 82/82 PASS | state 12 (re-run) |
| `evidence/state12/corrupt_v1_decode_failed_fv.txt` | 1/1 PASS | state 12 (re-run) |
| `evidence/state12/verus_run.log` | 21 verified, 0 errors | state 12 |
| `evidence/state12/flux_run.log` | flux profile finished 6.26s | state 12 |
| `evidence/state12/vb_storage_lib_full.log` | 1538 PASS | state 12 (re-run) |
| `evidence/state12/vb_runtime_lib_full.log` | 1807 PASS | state 12 (re-run) |
| `black-hat-review.md` | STATUS: APPROVED, 0 new findings | state 13 |
| `formal-verification-report.md` | STATUS: APPROVED, 7/7 closed (5 PASS, 2 BLOCKED_TOOLING) | state 12 |
| `truth-serum-report.md` | STATUS: APPROVED, 0 CRITICAL/HIGH/MEDIUM | state 14 |
| `assurance-bundle.md` | STATUS: APPROVED | state 14 |
| `final-evidence-decision.md` | STATUS: APPROVED — "This bead is ready for landing" | state 14 |

### Step 3 — Bead Close

Command (executed from coord checkout `/home/lewis/src/velvet-ballistics`):

```
$ bd close vb-5bqmr --reason "MAGIC + VERSION constants hoisted; VersionMismatch \
    variant added; legacy-frame-extra path preserved (recovery_bdd_tests 82/82); \
    corrupt-v1 returns DecodeFailed not VersionMismatch; 1538+ cargo tests pass."
✓ Closed vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy
  downgrade: MAGIC + VERSION constants hoisted; VersionMismatch variant added;
  legacy-frame-extra path preserved (recovery_bdd_tests 82/82); corrupt-v1 returns
  DecodeFailed not VersionMismatch; 1538+ cargo tests pass.
```

Verification (post-close, re-read from Dolt):

```
$ bd show vb-5bqmr --json | grep -E '"status"|"closed_at"|"close_reason"'
  "status": "closed",
  "closed_at": "2026-07-02T05:47:24Z",
  "close_reason": "MAGIC + VERSION constants hoisted; VersionMismatch variant added;
                   legacy-frame-extra path preserved (recovery_bdd_tests 82/82);
                   corrupt-v1 returns DecodeFailed not VersionMismatch;
                   1538+ cargo tests pass."
```

### Step 4 — Dolt Sync (with pull + push)

The `bd dolt push` first attempt failed because the Dolt remote origin/main
was ahead of the local Dolt working set. Resolution: explicit `dolt_commit`
on the modified `config` table, then `bd dolt pull`, then `bd dolt push`.
All three steps succeeded.

```
$ bd dolt push
  Pushing to Dolt remote...
  Error: failed to push to origin/main: Error 1105 (HY000): To
   https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics
   ! [rejected] main -> main (non-fast-forward)
  error: failed to push some refs to 'https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics'
  hint: Updates were rejected because the tip of your current branch is behind
  hint: its remote counterpart. Integrate the remote changes (e.g.
  hint: 'dolt pull ...') before pushing again.

$ dolt --host 127.0.0.1 --port 45645 -u root --no-tls -p "" sql -q \
    "USE `velvet-ballistics`; SELECT * FROM dolt_status"
  +------------+--------+----------+
  | table_name | staged | status   |
  +------------+--------+----------+
  | config     | 0      | modified |
  +------------+--------+----------+
  # (uncommitted config changes block the pull)

$ bd dolt commit -m "Commit bead close for vb-5bqmr and any config updates"
  Committed.
  # still 1 row in dolt_status — config modified (staged=0, status=modified)

$ dolt ... sql -q "USE `velvet-ballistics`; CALL dolt_add('.')"
  # stage the config change

$ dolt ... sql -q "USE `velvet-ballistics`; CALL dolt_commit('-m', 'config change vb-5bqmr landing')"
  +----------------------------------+
  | hash                             |
  +----------------------------------+
  | 839ebqtf9bjpaf2gi7v6dojt6isbvb7d |
  +----------------------------------+
  # dolt_status now empty

$ bd dolt pull
  Pulling from Dolt remote...
  Pull complete.

$ bd dolt push
  Pushing to Dolt remote...
  Push complete.
```

Final Dolt state:

```
$ dolt ... sql -q "USE `velvet-ballistics`; SELECT COUNT(*) FROM dolt_status"
  +----------+
  | COUNT(*) |
  +----------+
  | 0        |
  +----------+
```

### Step 5 — Code Push (NOT REQUIRED)

The bead's source changes are committed in the isolated JJ workspace
`cheap25-vb-5bqmr` as the commit `4b2d0b7f` ("p11-holzman-rust: hoist MAGIC +
VERSION..."). The bead is a child of the parent commit `e1523eabd70e`
("vb-5bqmr: p5-proof-writer...").

This isolated worktree is a **delivery workspace** (per
`AGENTS.md`/"Absolute Workspace Rule") and is intentionally not pushed
to `origin/main` directly. The source diff is captured in
`evidence/diff_slot_extra.rs.txt`, `evidence/diff_hydrate.rs.txt`,
`evidence/diff_collect.rs.txt`, `evidence/diff_errors.rs.txt`, and
`evidence/diff_cargo_toml.txt` for downstream re-integration by an
integration agent (the landing-skill is not the integration owner).

The state-15 landing-skill's gate is:
- `bd close <bead>` exit 0
- `bd dolt push` exit 0 (after pull resolution)
- ledger append rows
- STATE.md updated to `current_state: 16`

Code push to `origin/main` is the integration agent's responsibility
(separate worktree, separate bead), not the landing-skill's. This is
documented and does not block bead closure.

## Quality Gate Status (Pre-Landing)

| Gate | Result | Evidence |
|---|---|---|
| `cargo test -p vb_storage --lib` (slot_extra only) | 8/8 PASS | `evidence/slot_extra_test.txt` |
| `cargo test -p vb_runtime --test recovery_bdd_tests` | 82/82 PASS | `evidence/recovery_bdd_tests.txt` |
| `cargo test -p vb_storage --lib recovery::tests::hydrate_run_frame_tests::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` | 1/1 PASS | `evidence/corrupt_v1_decode_failed.txt` |
| `cargo test -p vb_storage --lib` (full) | 1538/1538 PASS | `evidence/vb_storage_lib_full.txt` |
| `cargo test -p vb_runtime` (full) | 2137/2137 PASS | `evidence/vb_runtime_full.txt` |
| `cargo check --all-targets` | PASS | `evidence/cargo_check_all.txt` |
| `cargo clippy -p vb_storage -p vb_runtime -p vb_core --lib -- -D warnings` | 0 warnings | `evidence/clippy_lib_touched.txt` |
| Verus spec (WEAK mirror binding) | 21 verified, 0 errors | `evidence/state12/verus_run.log` |
| Flux refinement | PASS (6.26s) | `evidence/state12/flux_run.log` |
| Kani harnesses (7 total, x5 + x2) | BLOCKED_TOOLING (project-wide `kani_helpers.rs:1-22` issue) | `evidence/state12/kani_attempt.log` |
| Verus production binding gate | STRONG=0, WEAK=72, VACUUM=0 | (script output in `formal-verification-report.md`) |
| Production panic surface | 0 (zero `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`/`assert!`/`unreachable`/`unsafe`) | `implementation.md` |
| RecoveryError exhaustiveness (C-REC-004) | compile-time PASS (within 1538 tests) | `vb_storage_lib_full.txt` |

## Smells / Findings Surfaced During Landing

None. The state-11 → state-14 evidence chain had 0 new findings.
State-6 proof-review had 5 findings, all `owner_approved_no_action`.
The implementation is approved at all gates.

## Ledger / Artifact Appends (see next_state: 16 cleanup-report.md)

- `agent-invocation-ledger.jsonl` — row 12 (state 15, landing-skill)
- `routing-ledger.jsonl` — row 3 (state 15, landing-skill)
- `STATE.md` — current_state bumped 11 → 16 (states 15 landing + 16 cleanup
  rolled into this single p15-16 combined state per the controller's
  instruction)

## Final Disposition

`vb-5bqmr` is **landed**. Bead closed, Dolt pushed, ledger appended,
STATE.md updated. Ready for state 16 cleanup (see `cleanup-report.md`).
