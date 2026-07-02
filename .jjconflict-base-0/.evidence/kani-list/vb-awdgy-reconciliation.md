# Kani Baseline Reconciliation Note (vb-awdgy)

**STATUS:** Reconciliation skeleton — applied to the Kani baseline-review reports
that pair a `STATUS: REJECTED`/`FAIL` decision with later `STATUS: PASS`
formal-verification-report claims.

## Finding (audit e08, 2026-06-30)

Multiple beads in the prior femdation sweep produced two parallel reports:

1. A `baseline-report.md` recording `cargo kani --list` results, sometimes
   ending with `STATUS: REJECTED` / `FAILED harnesses: N` because at least one
   listed harness could not be built or unwound.
2. A `formal-verification-report.md` (or a single `.evidence/<bead>/...`
   report) claiming the same Kani lane is `PASS`, with the prose asserting
   that all harnesses verified successfully.

These reports contradict each other. The contradiction is the bead finding.
Neither side can stand alone without raw Kani `cargo kani` logs, exit
statuses, and a clearly bounded unwind.

## Reconciliation Rule (this bead)

For every prior report pair we discovered:

- The **baseline-report** is treated as the source of truth for *what was
  actually executed*. If it says `STATUS: REJECTED`, the lane is not closed.
- The **final formal-verification-report** claim of `PASS` is downgraded to
  `STATUS: UNVERIFIED` until a fresh raw log is produced with command, exit
  status, and timing, and the harness list matches.
- We do **not** retroactively change a `STATUS: REJECTED` baseline to `PASS`.
  Per the master contract: *"If the raw evidence does not support the claim,
  downgrade the claim or keep the bead open."*

## Index of Contradictory Reports (work applied)

This bead applies the downgrade to the following pre-existing report
locations. (Each row was checked; new contradictions may exist and should be
appended.)

| Bead / Report Pair | Baseline Decision | Final Report Claim | Reconciliation Action |
|--------------------|-------------------|--------------------|------------------------|
| `.evidence/vb-core-proof-15-gate/formal-verification-report.md` | n/a (no log attached) | inline "PASS" for 6 Kani harnesses | Downgraded in vb-5kow2 (same audit group) |
| `.evidence/vb-core-proof-15-gate/black-hat-review.md` | n/a | cites "VB-STORAGE-GAP-001..006 PASS" | Downgraded in vb-5kow2 |
| `.beads/vb-scxh/formal-verification-report.md` | `STATUS: REJECTED` (safety-anchor block) | "Kani" listed under WAIVERS | Waivers preserved; no false PASS claim, but row added below for traceability |
| `.beads/vb-scxh/baseline-report.md` | n/a (Kani baseline missing) | n/a | Marked as Kani-baseline-missing in `vb-awdgy` companion note below |
| `.evidence/kani-list/vb_*.json` | raw `cargo kani --list` snapshots | n/a | Treated as evidence anchors, not pass/fail claims |

## Companion Note for vb-scxh

The `.beads/vb-scxh/baseline-report.md` does not contain a Kani row. The
companion `formal-verification-report.md` lists `Kani` under
`WAIVED`-classified lanes rather than under `PASS`/`FAIL`. This pair is not
contradictory in the same way as the audit finding — the bead `vb-scxh` was
already closed with an explicit Kani waiver — but it is recorded here so the
next pass knows the case was inspected.

## Action Taken in this Bead

1. We did **not** flip any baseline from `REJECTED` to `PASS`.
2. We did **not** delete any historical report; reports are evidence and
   must be preserved or moved, not erased.
3. We added a reconciliation note (this file) under
   `.evidence/kani-list/` so future formal-verifier runs can locate the
   contradiction set and audit reconciliation completeness.
4. Where a `formal-verification-report.md` text claims `PASS` for Kani
   without a raw log, the claim is downgraded via vb-5kow2 (sister bead) or
   this bead's companion reconciliation entry.

## Acceptance Criteria (this bead)

- [x] Contradictory report pairs enumerated in this index.
- [x] Reconciliation rule stated explicitly (no flip without raw evidence).
- [x] No `STATUS: REJECTED` baseline flipped to `PASS`.
- [x] No historical report deleted; downgrade or annotation only.
- [x] Companion note added to the bead workspace.

## Open Follow-Ups

- For every contradiction row, a follow-up bead must produce a fresh raw
  `cargo kani` log with command + exit status + timing, and the harness list
  must be reconciled to the actual list returned by
  `bash scripts/kani-list.sh <package> [...]`.
- Until that happens, the affected lanes remain `UNVERIFIED` in
  formal-verification reports and `REJECTED` (or absent) in baselines.