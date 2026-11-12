# Regression Diff

STATUS: PASS

Baseline from `.beads/vb-m5gp/baseline-report.md`: shared-parent `moon ci` exited 0 with 23 tasks completed.

State 11 rerun after dependency-edge repair:

- Required exact obligations: PASS or WAIVED.
- New local/regression failures: none.
- `moon ci`: PASS, 23 tasks completed in 44s 161ms.
- Workspace test summary: 10771 passed, 44 skipped.
- Source-length delta: bead-local split files pass; pre-existing unrelated oversized files remain DEFERRED_GLOBAL only.
- Optional direct Miri delta: still environment-blocked by missing local nightly rust-src path; non-blocking because `MIRI-001` is `required:false` and `moon ci` Miri lane passed.

Classification: no FAIL_LOCAL and no FAIL_REGRESSION; approved for State 12 black-hat.
