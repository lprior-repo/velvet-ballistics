bead_id: vb-qi37.4.4
phase: State 15 preflight
updated_at: 2026-05-11

# Landing Evidence

## Workspace Guard
- Source checkout forbidden path: `/home/lewis/src/Velvet-ballistics`.
- Isolated workspace used for all commands/writes: `/home/lewis/src/Velvet-ballistics-vb-qi37-4-4-go`.
- Path guard command returned success: isolated workspace realpath is not equal to and not nested under source checkout realpath.

## Canonical Artifact Gate
- Required State 1-14 artifacts are present and non-empty in `.beads/vb-qi37.4.4/`.
- JSONL parse gate passed for `delivery-scope.jsonl`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `verification-ledger.jsonl`.
- Exact status lines observed:
  - `contract-verification-review.md`: `STATUS: APPROVED`
  - `test-plan-review.md`: `STATUS: APPROVED`
  - `manual-qa-smoke.md`: `STATUS: PASS`
  - `qa-review.md`: `STATUS: APPROVED`
  - `test-suite-review.md`: `STATUS: APPROVED`
  - `black-hat-review.md`: `STATUS: APPROVED`
  - `formal-verification-report.md`: `STATUS: APPROVED`
  - `tla-report.md`: `STATUS: APPROVED`
  - `architectural-drift-review.md`: `STATUS: REFACTORED`; downstream States 8-14 artifacts have later mtimes and are present.
  - `manual-qa-final.md`: `STATUS: APPROVED` and added exact `STATUS: PASS` because the artifact contains real manual QA pass evidence for `--help` and `--version` invocations.
- Optional/failure-only artifacts absent and not synthesized: `compiler-errors.log`, `defects.md`, `verus-report.md`, `kani-report.md`, `kani-justification.md`.
- Verification ledger counts: `PASS=6`, `WAIVED=1`, `DEFERRED_GLOBAL=1`, `FAIL_LOCAL=0`.

## JJ / Rebase Preflight
- `jj git fetch`: `Nothing changed.`
- `main`: `lxwyustnlklk c993943126cc landing: merge landable vb-jkrk wave3 qi37.16.3`.
- Rebase command: `jj rebase -r @ -d main`.
- Rebase outcome: success, no conflicts. Current parent is `main` at `c993943126cc`.
- Current change after rebase: `uoxvlsmwxmxl c159d7a0` before writing this landing evidence file.
- Working-copy diff summary after rebase: 41 files changed, 1563 insertions, 913 deletions.

## Commands Run During State 15 Preflight
- `jj status`: showed isolated working-copy changes for bead artifacts, `vb_runtime` error split, admission durability integration tests, and TLA specs.
- `jj log`: current change `uoxvlsmwxmxl`; pre-rebase parent `5fb2d246b0f6`, then successfully rebased to parent `c993943126cc`.
- `jj diff --stat`: 41 changed files after rebase.
- `moon ci`: PASS after rebase; 19 tasks completed in 6m24s, including `source-length`, `fmt`, `lint-src`, `test`, `feature-powerset`, `doc-test`, `hardened-build`, `maxperf`, and `maxperf-native`.
- `rtk cargo test -p vb_runtime runtime_error --lib && rtk cargo test -p velvet_ballastics --test admission_durability_code`: PASS; `19 passed, 1324 filtered out`; `1 passed`.

## Landing Decision
- Ready-to-land preflight: YES for code/artifact/gate state, subject to normal State 15 landing actions not requested here.
- Not performed by request: moving main, pushing, closing bead, forgetting/removing workspace.
