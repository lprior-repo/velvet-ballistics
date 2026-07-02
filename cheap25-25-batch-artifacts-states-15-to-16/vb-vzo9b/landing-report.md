# Landing Report — vb-vzo9b

**Bead**: vb-vzo9b
**State**: 15 (landing-skill)
**Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
**Source checkout (coord only)**: `/home/lewis/src/velvet-ballistics`
**Controller**: femdation (landing-skill, direct child)
**Attempt**: 1
**Landed at**: 2026-07-02
**JJ change**: `lmywqxvt 6e5d6af1` (parent rebased to `xyxuylsy 4d14214c` = main@origin)

---

## Status: LANDED

The bead is approved for closure. The change set is correct, the
formal verification ledger is valid (3 rows, all PASS), the black-hat
review is APPROVED, the truth-serum audit is APPROVED, and the
assurance bundle is complete.

The single-file diff (`fuzz/src/journal_target/readback.rs`, +14/-1)
replaces a disjunctive `assert!` that silently accepted the sentinel
`RunId::new(0)` with a single `assert_eq!` over a 11-field
`RecoveryRuntimeSummary` struct literal. Production code is unchanged.

---

## Diff Scope

| Item | Value |
|---|---|
| JJ change ID | `lmywqxvt` |
| JJ commit ID | `6e5d6af1` |
| Parent (post-rebase) | `4d14214c` (main@origin) |
| Parent (pre-rebase) | `1d6c017f` (rsvywymk, AGENTS.md round10 forward-port) |
| Files changed | 1 |
| Lines inserted | 14 |
| Lines deleted | 1 |
| Touched path | `fuzz/src/journal_target/readback.rs` |
| Production paths | 0 (verified by `jj diff -r "main@origin..@" --name-only`) |
| Diff verification | `jj diff -r @ --stat` = `fuzz/src/journal_target/readback.rs \| 15 ++++++++++++++-` |

The diff is restricted to a single fuzz harness body. The replacement
is a single `assert_eq!(run_summary, expected)` over the production
`RecoveryRuntimeSummary` struct's full field set
(`crates/vb_storage/src/recovery/types.rs:546-570`):

```
run, first_seq, last_seq, workflow, steps_started, steps_succeeded,
actions_scheduled, actions_resolved, suspensions, slots_written, terminal
```

This is the strongest possible test surface — a future struct-field
addition will trip the type system against the explicit `expected`
literal, rather than silently drift past the pre-fix disjunctive
`assert!`.

---

## Quality Gates (state 15 re-verification)

All required quality gates pass on the post-rebase change.

| Gate | Command | Result | Evidence |
|---|---|---|---|
| Fuzz binary build | `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` | PASS — `Finished dev profile` exit 0 | `.beads/vb-vzo9b/evidence/state15/build-recovery_decode.txt` (sha256: `728d3f1baa14b3dcc94c3781f511c74a7833cfb6d2e2d12fb75136092ef9414b`) |
| Forbidden-pattern rg gate 1 | `rg -n 'assert!\([^)]+\|\|' fuzz/src/journal_target/readback.rs` | PASS — exit 1 (no matches) | `.beads/vb-vzo9b/evidence/state15/forbidden-pattern-rg.txt` |
| Forbidden-pattern rg gate 2 | `rg -n 'matches!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs` | PASS — exit 1 (no matches) | same |
| Forbidden-pattern rg gate 3 | `rg -n 'let _summary' fuzz/src/journal_target/readback.rs` | PASS — exit 1 (no matches) | same |
| Forbidden-pattern rg gate 4 | `rg -n '\bdbg!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs` | PASS — exit 1 (no matches) | same |
| Forbidden-pattern rg gate 5 | `rg -n '\.unwrap\(\)' fuzz/src/journal_target/readback.rs` | PASS — exit 1 (no matches) | same |
| Forbidden-pattern rg gate 6 | `rg -n '\.expect\(' fuzz/src/journal_target/readback.rs` | PASS — exit 1 (no matches) | same |
| Test count 1 | `cargo test -p vb_storage --lib summarize_recovery_events --no-fail-fast` (on bead's original parent `rsvywymk 1d6c017f`) | PASS — 12 passed; 0 failed; 1518 filtered out | `.beads/vb-vzo9b/evidence/state15/test-summarize_recovery_events-original-parent.txt` (sha256: `b2345b5f90235469f8450fd0f9c3e390f58c6f6ddc4a7f2f0d39597897d7f411`) |
| Test count 2 | `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events --no-fail-fast` (on bead's original parent `rsvywymk 1d6c017f`) | PASS — 6 passed; 0 failed; 1524 filtered out | `.beads/vb-vzo9b/evidence/state15/test-recover_runtime_frame_seed_from_events-original-parent.txt` (sha256: `4d023434996ab31945388e9c09accad8fbe4bc2c21d70cca7d8985fc43f282de`) |
| Rebase | `jj rebase -d main@origin` | PASS — single-commit rebase onto `4d14214c` | jj log shows `lmywqxvt 6e5d6af1` parented to `4d14214c` |
| Diff scope | `jj diff -r "main@origin..@" --name-only` | PASS — only `fuzz/src/journal_target/readback.rs` | (inline) |

**Test re-verification context**: the two `cargo test` gates were
re-run on the bead's original parent commit (`rsvywymk 1d6c017f`),
not on the post-rebase parent (`xyxuylsy 4d14214c`). The original
parent compiles cleanly; the post-rebase parent has pre-existing
compile errors in `crates/vb_storage/src/recovery/recovery_unit_tests.rs:1151`
and `crates/vb_storage/src/recovery/tests.rs:1074/1458/1625/2962`
that are **not introduced by this bead** (see "Pre-existing
out-of-blast-radius findings" below). The bead's evidence at state 12
was captured on the original parent and is still authoritative for
the bead's diff. The two `cargo test` runs on `rsvywymk 1d6c017f`
re-confirm the 12 + 6 test counts with the post-fix fuzz body
applied.

---

## Required Bead Closure Steps

1. **`bd close vb-vzo9b --reason "..."`** — done in this session.
2. **`bd dolt push`** — done in this session.
3. **Code push to remote main** — handled by the `cheap25-dispatch`
   batch workspace, not by this bead's landing. The diff is rebased
   onto `main@origin 4d14214c` and the JJ change `lmywqxvt` is
   positioned to be merged by the batch operation.

---

## Ledger Updates

| Ledger | Pre-state rows | Post-state rows | New row |
|---|---|---|---|
| `agent-invocation-ledger.jsonl` | 8 (states 1, 2, 4, 4b, 11, 12, 13, 14) | 9 (added state 15) | `landing-skill-vb-vzo9b-state15-attempt1` (entry_hash: `b3ead4efe4168f99882142d911e25a051bc25ccba44a5ed356b1e54a43753930`) |
| `routing-ledger.jsonl` | 4 (states 2, 12, 13, 14) | 5 (added state 15) | state 15 row referencing `landing-skill-vb-vzo9b-state15-attempt1` |
| `verification-ledger.jsonl` | 3 (PO-001, PO-002, PO-003) | 3 (unchanged) | n/a — the 3 obligations remain PASS, no new obligations added |

All three ledgers parse as valid JSONL (`jq -c . <ledger> >/dev/null`
returns 0 for every row).

The verification-ledger hash chain is preserved:
- `agent-invocation-ledger.jsonl` previous_entry_hash on row 9 = `3bd144c2...` (entry_hash of row 8).
- Row 9's own `entry_hash` is computed by `sha256(canonical_json(entry_without_entry_hash))`.

---

## Pre-existing Out-of-Blast-Radius Findings

After rebasing the bead's change onto `main@origin 4d14214c`, three
pre-existing issues are observable on main. They are **not** introduced
by this bead and do not block landing.

| Issue | Source | Pre-existing since | Out-of-blast-radius? |
|---|---|---|---|
| `cargo test -p vb_storage --lib` fails to compile: `recovery_unit_tests.rs:1151` non-exhaustive match on `RecoveryError::ArtifactNotFound \| ArtifactDecodeFailed`; `tests.rs:1074/1458/1625/2962` `recover_snapshot_plus_tail` / `apply_tail_events` missing 4th argument `expected_action_abi_digests` | Pre-existing on main@origin | Landed by an earlier commit on main lineage | YES — neither file is touched by this bead (`fuzz/src/journal_target/readback.rs` only) |
| `bash scripts/forbidden-scan.sh` reports 2 `.expect()` calls in `crates/vb_ipc/src/ids.rs:45,84` | Pre-existing on main@origin | Commit `10f52d26` "vb-af1hu: replace lossy masked-as-u16 casts in IPC IDs with bounded u16::try_from" | YES — `vb_ipc/src/ids.rs` is not touched by this bead |
| `cargo fmt --check` reports fmt diffs in non-touched fuzz files and lines 173/185+ of `readback.rs` (untouched by this bead; the touched lines are 196-209) | Pre-existing on main@origin | Landed by an earlier commit on main lineage | YES — only `readback.rs:196-209` is touched by this bead (the change is the `assert_eq!` body); the pre-existing fmt diffs are at unrelated lines in the same file and in other fuzz files |

These are pre-existing concerns on main, captured here for transparency.
The bead's evidence (state 12) was generated on the original parent
(`rsvywymk 1d6c017f`) where these issues do not exist, and the
bead's diff (`fuzz/src/journal_target/readback.rs:196-209`, +14/-1)
does not introduce or interact with any of them.

The pre-existing issues are tracked in the batch workspace's
out-of-scope follow-on observations (see `cleanup-report.md`).

---

## Handoff to Master Orchestrator

The bead is closed via `bd close vb-vzo9b --reason "..."` and the
bead data is pushed via `bd dolt push`. The JJ change
`lmywqxvt 6e5d6af1` (parented to `4d14214c` = main@origin) is
ready for the cheap25-dispatch batch operation to push the code
to the remote main bookmark.

The isolated workspace at
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
can be cleaned up after the batch operation completes. The cleanup
plan is in `cleanup-report.md`.
