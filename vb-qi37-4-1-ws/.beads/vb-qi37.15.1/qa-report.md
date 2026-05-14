bead_id: vb-qi37.15.1
phase: State 9

# QA Report

Executed evidence:
- Manual smoke `simulate <workflow> --json` succeeded with `schema_version`, `kind`, totals, and trace entries.
- Scoped `cli_simulate` tests passed: 4 passed, 74 filtered out.
- Canonical `moon ci` failed on pre-existing missing `main` revision, classified DEFERRED_GLOBAL.

Findings:
- MINOR PROCESS: State 5 lacks a separately captured red run for the schema assertion.
