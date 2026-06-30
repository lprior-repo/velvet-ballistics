# Machine Gate Report: vb-scxh State 11

STATUS: REJECTED

## Startup Authority

- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: version `1.5.0`, mission requires every scoped proof obligation to have real command evidence; missing required tools/evidence are not PASS.
- Read `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same version/content; per instruction this file wins on conflict.

## Mandatory Gate

- Command: formal mandatory `test -s`/`rg '^STATUS: APPROVED$'`/`jq -c` gate over State 11 inputs.
- Result: PASS; observed `3:STATUS: APPROVED` from contract-verification review.

## Blocking Gates

- Safety anchor: `FAIL_LOCAL` / `BLOCK_LOCAL`; fresh rerun of `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z` failed to open the bundle.
- TLA rerun: PASS after repo-local temp/metadir rerun; TLC reported `Model checking completed. No error has been found.`, `12277 states generated, 984 distinct states found, 0 states left on queue`.
- Moon CI freshness: PASS after source repair; fresh command `TMPDIR=/home/lewis/src/vb-scxh/target/tmp RUSTC_WRAPPER= moon ci --force --summary normal` exited 0, reported `Actions: 21 completed`, and the test lane reported `8185 tests run: 8185 passed, 6 skipped`.
- State 12 truth-serum/final-decision rows: not executed in State 11; closure/unblock remains blocked.

## Passing Audit Gates

- Workspace path guard: PASS, `pwd -P` returned `/home/lewis/src/vb-scxh`.
- Required artifact presence: PASS for listed `.beads/vb-scxh` and `.beads/vb-gvmt` inputs.
- BD false-closure extraction: PASS with exact 12 raw dependency IDs captured.
- Mutation classification: PASS for `FAIL_UNVIABLE`/`DEFERRED` and exact `35/35 unviable`; not adequacy PASS.
- Scope control: PASS; `vb-gvmt` and `vb-qi37.10` remain open owners for generated parity/codegen gaps.

## Decision

Do not proceed to State 12. Post-rerun update: the Moon CI PASS blocker is cleared by fresh evidence, but the safety anchor `BLOCK_LOCAL` remains. State 11 therefore remains rejected/blocked until the safety anchor is restored and rerun or an explicit owner-approved waiver exists.
