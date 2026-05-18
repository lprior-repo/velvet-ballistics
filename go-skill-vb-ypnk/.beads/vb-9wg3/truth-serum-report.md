# vb-9wg3 Truth Serum Report

## Verdict

PASS.

## Raw Evidence Checks

- `cargo check -p vb_core` transcript includes the command and PASS result.
- Kani evidence contains all five scoped harness names and `Complete - 5 successfully verified harnesses, 0 failures, 5 total.`
- TLC evidence contains `Model checking completed. No error has been found.`, `166 states generated`, `84 distinct states found`, and depth `2`.
- `verification-ledger.jsonl` and `traceability-matrix.jsonl` are valid JSONL.
- TLA source hash evidence matches `.beads/vb-9wg3/tla-transcription-map.md`.
- `moon ci` is honestly reported as `FAIL_OUT_OF_SCOPE_DIRTY_WORKTREE`.

Reviewer task: `ses_1ca0f2fa8ffem7ejcNm8ZQfy0S`.
