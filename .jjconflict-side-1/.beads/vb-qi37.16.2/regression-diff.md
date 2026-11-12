# State 8 Regression Diff — vb-qi37.16.2

## Baseline

Initial baseline `moon ci` failed before bead edits because the isolated JJ workspace could not resolve git revision `main`.

## Prior Rebase Failure

After rebase onto the local CI fix, `moon ci` failed on:

- scoped formatting diffs,
- leftover conflict marker/diff text in `xtask/src/main.rs`,
- duplicate `Default` implementation on `EnvelopeHeader`,
- then xtask proof CLI/module wiring and integration-test visibility fallout.

## Current Result

`moon ci` now passes globally.

```text
Tasks: 19 completed (2 cached)
Time: 2m 59s 716ms
```

Classification: PASS. No remaining State 8 blocker.
# Current rerun classification

STATUS: PASS

`moon ci` passed after the State 6 repair. No BLOCK_LOCAL or BLOCK_REGRESSION CI failure remains. State 12 formal verifier blockers are tracked in `formal-verification-report.md` and `verification-ledger.jsonl`.
