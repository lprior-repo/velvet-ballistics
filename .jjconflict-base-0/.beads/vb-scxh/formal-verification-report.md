# Formal Verification Report

STATUS: REJECTED

## Inputs

- proof-obligations.jsonl: `.beads/vb-scxh/proof-obligations.jsonl`, 33 rows.
- delivery-scope.jsonl: `.beads/vb-scxh/delivery-scope.jsonl`.
- baseline-report.md: `.beads/vb-scxh/baseline-report.md`.
- tla-spec.md: `.beads/vb-scxh/tla-spec.md`.
- contract-verification-review.md: `STATUS: APPROVED`.

## Tool / Command Evidence

- `pwd -P`: PASS, `/home/lewis/src/vb-scxh`.
- Mandatory input gate: PASS.
- `bd --db /home/lewis/src/.beads/dolt show vb-scxh --json`: PASS; exact 12 dependency IDs captured.
- `bd --db /home/lewis/src/.beads/dolt list --json`: command executed; raw output too large for terminal and saved by tool.
- Per-ID `bd show <id> --json`: PASS for all 12 extracted false-closure IDs.
- `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`: FAIL_LOCAL, `could not open` bundle.
- `git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`: no stdout in `/home/lewis/src/vb-scxh` or read-only `/home/lewis/src/Velvet-ballistics`; no valid ref-resolution evidence captured.
- TLC repo-local temp/metadir rerun: PASS. Command used `TMPDIR=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp`, `JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp'`, `tlc -metadir .beads/vb-scxh/tla-metadir -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla`; terminal marker `Model checking completed. No error has been found.`; `12277 states generated, 984 distinct states found, 0 states left on queue`.
- Moon CI audit: PASS after source repair. Fresh forced command `TMPDIR=/home/lewis/src/vb-scxh/target/tmp RUSTC_WRAPPER= moon ci --force --summary normal` exited 0 from `/home/lewis/src/vb-scxh`; summary reported `Actions: 21 completed`, `Time: 34s 838ms`, and test lane `8185 tests run: 8185 passed, 6 skipped` with Nextest run ID `084a71cb-efd5-4dd3-9c50-13d96a71a9fc`. Artifact-path evidence is recorded in `.beads/vb-scxh/moon-ci-evidence-audit.md`.
- Mutation audit: PASS classification integrity; `35 mutants tested in 34s: 35 unviable`, `FAIL_UNVIABLE / DEFERRED`, not adequacy PASS.
- Scope audit: PASS; `vb-gvmt` and `vb-qi37.10` remain open owners for generated parity/codegen gaps.

## Obligation Summary

- PASS: 19
- FAIL_LOCAL: 6
- FAIL_REGRESSION: 0
- WAIVED: 8
- DEFERRED_GLOBAL: 0
- Total: 33

## Blocking Failure Packets

### SAFETY-SCXH-001 / ERR-SCXH-006

- Command: `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`.
- Failure: `error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'`.
- Classification: `FAIL_LOCAL` / `BLOCK_LOCAL`.
- owner_state: 11 for raw repair, then 12 only if a waiver/final decision is justified.
- rerun_from: 11.

### TRUTH/Final Decision Rows

- Rows: `TRUTH-SCXH-001`, `ERR-SCXH-003`, `ERR-SCXH-004`, `ERR-SCXH-009`.
- Failure: State 12 artifacts were not run/written in State 11 and must not be invented.
- Classification: `FAIL_LOCAL` until State 12 executes after State 11 blockers are fixed or waived.

## Waivers

Waiver rows in `proof-obligations.jsonl` were validated as planned scope waivers and classified `WAIVED`: Verus, Lean/Aeneas/Hax, Kani, Flux, Loom/Shuttle, Miri/cargo-careful, proptest/fuzz, performance/API/release-provenance.

## Decision

State 11 remains `REJECTED/BLOCKED` solely on the remaining safety-anchor and downstream State 12 rows. Moon CI freshness is now PASS. Do not close `vb-scxh`, do not unblock `vb-engine-yaml`, and do not proceed to State 12 until the safety anchor is restored or explicitly approved by owner waiver.
