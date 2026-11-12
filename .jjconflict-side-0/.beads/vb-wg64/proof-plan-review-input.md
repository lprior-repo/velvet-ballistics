# vb-wg64 Proof Plan Review Input

## Review Target

Review the State 4 proof plan for bead `vb-wg64`.

Artifacts:

- `.beads/vb-wg64/proof-strategy.md`
- `.beads/vb-wg64/proof-obligations.planned.jsonl`
- `.beads/vb-wg64/contract.md`
- `.beads/vb-wg64/traceability-matrix.jsonl`
- `.beads/vb-wg64/delivery-scope.jsonl`

## Planning Claim

The bead is a CI repair, not a feature or algorithm change. The correct proof strategy is executable CI evidence plus diff review, not new formal proof artifacts.

## Required Obligations Under Review

- Formatting gate: `rtk cargo fmt --all -- --check`
- xtask strict lint gate: `rtk cargo clippy -p xtask --all-targets -- -D warnings`
- vb_cli strict lint and module resolution gate: `rtk cargo clippy -p vb_cli --all-targets -- -D warnings`
- vb_storage recovery test compile gate: `rtk cargo check -p vb_storage --test recovery_bdd_tests`
- Final clean-clone gate: `moon ci --base HEAD --head HEAD --force`
- Diff review gate for no behavior change, no assertion deletion, no broad allowlist, and no CI weakening

## Reviewer Questions

1. Does every obligation map to a contract requirement or invariant?
2. Are the planned commands specific enough to catch the known failures in `baseline-report.md` and `codebase-map.md`?
3. Is `rtk cargo check -p vb_storage --test recovery_bdd_tests` sufficient as the targeted recovery BDD compile gate, with `rtk cargo check -p vb_storage --tests` retained as optional broader confirmation?
4. Are formal lanes correctly marked not applicable given the contract prohibits behavior, concurrency, unsafe, parser, and state-machine changes?
5. Does the final forced clean-clone gate remain mandatory and non-substitutable?

## Known Risks For Later States

- Workspace rustfmt drift may touch files outside the narrow failure map. Such changes are acceptable only when rustfmt-only.
- The `mode_error` module must not widen production API surface beyond what existing tests require.
- Recovery BDD cleanup must not delete assertions or setup expressions with side effects.
- Clippy fixes must not add broad `allow` or `expect` attributes to hide source-lint failures.
- Passing targeted gates is preflight evidence only; acceptance requires forced `moon ci`.

## Expected Review Verdict

Approve if the reviewer agrees the obligation matrix is executable, traceable, and strict enough for a minimal CI repair. Reject if any required gate is optionalized, if a formal lane is needed by an actual risk trigger, or if final forced CI can be bypassed.
