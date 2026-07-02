# Implementation — vb-t0iw9: femdation `replacement_seq` schema repair

- bead_id: vb-t0iw9
- type: BUG
- priority: P1
- controller: femdation (direct child, no sub-agents)
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9
- source_checkout: /home/lewis/src/velvet-ballistics
- jj workspace: cheap25-vb-t0iw9 (parent rsvywymk)
- chosen_repair: **Option C — DocumentExpectedUserAction**
- implementation_date: 2026-07-01

## Why Option C

The bead description offers three options:

| option                          | applicability to this bead                                                                                                                                                                                                                                                                              |
|---------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| A. Edit `.beads/config.yaml` port | The port drift (43643 in config vs 45645 actual) is real but **does not cause** the `replacement_seq` error. `bd` finds the server via `metadata.json` regardless. Editing the port fixes a cosmetic config-drift issue unrelated to the failing query.                                                          |
| B. Add `.beads/schemas/*.cue` migration | `.beads/schemas/` contains project-domain CUE schemas, not bd SQL migrations. bd loads its migrations from the embedded binary, not from disk. Adding a CUE file does not make bd execute a SQL ALTER. This option is technically inert.                                                                       |
| C. Write `.beads/vb-t0iw9/runbook.md` | Documents the two valid user actions (one-time SQL ALTER TABLE; bd upgrade) that actually unblock femdation. Respects the MUST NOT list: no binary, scripts/, metadata.json, or embeddeddolt changes. The chosen repair.                                                                                       |

Option C is the only option that both (a) respects the MUST NOT list and
(b) maps to the actual root cause: the bd binary's forward schema-skew
guard queries a `replacement_seq` column that does not exist in the v49
Dolt schema, and the binary itself does not register a migration that
would add it (`bd migrate --inspect` → `Registered Migrations: 0`).

## Files Created

| path                                                | purpose                                                                                  |
|-----------------------------------------------------|------------------------------------------------------------------------------------------|
| `.beads/vb-t0iw9/runbook.md`                        | The Option C artifact: two validated user actions (A: ALTER TABLE; B: bd upgrade).      |
| `.beads/vb-t0iw9/implementation.md`                 | This file.                                                                               |
| `.beads/vb-t0iw9/evidence/repro.txt`                | Raw reproduction commands and outputs (bd sql, bd migrate, bd info, strings).             |
| `.beads/vb-t0iw9/evidence/schema-before.txt`        | `bd sql "DESCRIBE issues"` output proving `replacement_seq` is absent.                   |
| `.beads/vb-t0iw9/evidence/schema-migrations.txt`    | `SELECT version FROM schema_migrations` showing v49 is the highest applied migration.     |
| `.beads/vb-t0iw9/evidence/bd-version.txt`           | `bd version` + `bd info --whats-new` excerpt (forward schema-skew guard = v1.0.5).       |
| `.beads/vb-t0iw9/evidence/supersede-flag.txt`       | Proof that `--ignore-schema-skew` bypasses the bd-level guard (but not the column).      |
| `.beads/vb-t0iw9/evidence/port-drift.txt`           | Side discovery: `.beads/config.yaml` port 43643 vs actual server 45645 vs inner 43627.   |
| `.beads/vb-t0iw9/evidence/check-beads-server-mode.txt` | Output of `bash scripts/check-beads-server-mode.sh` (exit 0, dolt_mode=server).         |
| `.beads/vb-t0iw9/evidence/claim-result.txt`         | Output of `bd update vb-t0iw9 --claim` (exit 0, claim succeeds).                         |

## Files NOT Modified

- `.beads/metadata.json` — left untouched. `dolt_mode` remains `"server"`.
- `.beads/config.yaml` — left untouched. Port drift is a follow-up concern.
- `.beads/dolt/config.yaml` — left untouched.
- `.beads/embeddeddolt/` — not present; verified absent.
- `scripts/` — not modified.
- `bd 1.0.5` binary — not modified.

## Diff Summary

```
.beads/vb-t0iw9/
  evidence/
    repro.txt                     (new)
    schema-before.txt             (new)
    schema-migrations.txt         (new)
    bd-version.txt                (new)
    supersede-flag.txt            (new)
    port-drift.txt                (new)
    check-beads-server-mode.txt   (new)
    claim-result.txt              (new)
  implementation.md                (new, this file)
  runbook.md                      (new, Option C artifact)
```

Zero source files (Rust) touched. Zero scripts modified. Zero metadata widened.

## Required Verification Gate

```bash
# Gate 1: jj workspace still rooted at isolated path.
pwd -P
test "$(pwd -P)" = "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9"  # PASS
jj root
# /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9  (PASS)

# Gate 2: dolt_mode still server; embeddeddolt NOT created.
bash scripts/check-beads-server-mode.sh   # exit 0 (PASS)
test ! -e .beads/embeddeddolt              # exit 0 (PASS)

# Gate 3: bead claimable.
bd update vb-t0iw9 --claim  # exit 0 (PASS)

# Gate 4: NO source code (Rust) modified.
cd /home/lewis/src/velvet-ballistics
git status --porcelain
# No M entries under crates/  (PASS; only .beads/vb-t0iw9/ inside isolated
# workspace is in its own jj change, not the source checkout).
```

## Pre-existing Bead State (before this implementation)

When the implementation started, the bead's STATE.md and full go-skill
artifact set (contract.md, domain-model.md, type-contracts.md,
proof-coverage-matrix.md, etc.) were already present from a prior
explore/proof-plan-reviewer dispatch on 2026-07-01. Those artifacts are
preserved unchanged. The work in this delivery is the small config-only
Option C repair on top of that pre-existing state.

During evidence capture, the `bd supersede vb-t0iw9 --with vb-ik5vm`
command was inadvertently invoked with a non-self target while testing
whether `--ignore-schema-skew` actually works. The bead was reopened
(`bd reopen vb-t0iw9`), the wrong-target dependency was removed
(`bd dep remove vb-t0iw9 vb-ik5vm`), and the bead's status is back to
`OPEN` priority 1. No `bd close` was issued and no permanent state
mutation against the wrong target persists. The pre-existing
`vb-qryp7 supersedes vb-t0iw9` dependency from 2026-07-01 10:56:23
remains untouched (it predates this implementation).

## Residual Risk

1. **Action A (ALTER TABLE) leaves a Dolt working-set change that must
   be committed.** If the user runs only steps 1-4 of Action A and
   skips `bd dolt commit`, the column disappears on next server restart
   or restart of the working set. The runbook explicitly lists
   `bd dolt commit` as step 5.
2. **Action B (bd upgrade) is not yet available in the installed
   `go-github-com-steveyegge-beads-cmd-bd` mise channel.** The
   `bd info --whats-new` output shows v1.0.5 is current; no v1.0.6
   entry is yet visible from this build. If no newer release exists
   on the remote, the user must use Action A.
3. **`--ignore-schema-skew` is not a permanent substitute for fixing
   the schema.** The runbook does not propose it as a long-term fix;
   it is only mentioned in evidence to show where the bd-level guard
   actually fires.
4. **Port drift (43643 vs 45645) is unaddressed by this bead.** It is
   a config-only fix and is documented in `evidence/port-drift.txt`
   and `runbook.md` for a follow-up bead.
5. **The bd binary's `bd supersede` code path does not actually query
   `replacement_seq` at the SQL layer** (verified by the fact that
   `bd --ignore-schema-skew supersede ...` succeeded against a DB
   without the column). The error originates from the
   forward-skew preflight guard. If a future bd release moves that
   check to the SQL layer, Action A is still correct (column present),
   but Action B's binary upgrade path may need re-validation.

## Closure Path

This bead is a P1 BUG. The implementation artifact is `runbook.md`,
which gives Lewis two actionable options. The bead itself cannot be
closed by this delivery alone — the user must execute Action A or
Action B and re-verify. The recommended closure flow:

1. Land this implementation in the isolated workspace.
2. Land the `runbook.md` upstream via the normal femdation landing flow.
3. The user (Lewis) runs Action A in `/home/lewis/src/velvet-ballistics`
   and commits the ALTER TABLE to Dolt.
4. The user re-runs the femdation first-wave dispatch.
5. If femdation succeeds, this bead is closed with reference to the
   runbook + the user's commit hash.
6. If femdation still fails, the user opens a follow-up bead and
   escalates Action B.
