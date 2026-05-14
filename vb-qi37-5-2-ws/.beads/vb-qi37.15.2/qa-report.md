bead_id: vb-qi37.15.2
phase: State 9

# QA Report

Executed evidence:
- Manual smoke `submit <workflow> --input-bin <file> --db <tmp>/db --durability journaled --json` succeeded with digest/run_id/status/step_count.
- Scoped `cli_submit` tests passed: 4 passed, 74 filtered out.
- Red evidence showed previous submit failure `FjallError: Locked`; green evidence shows repaired JSON submit test passes.
- Canonical `moon ci` failed on pre-existing missing `main` revision, classified DEFERRED_GLOBAL.

Findings:
- No bead-local QA blocker found.
