bead_id: vb-qi37.13.4
phase: State 8

# Regression Diff

- Earlier State 8 baseline failure: `moon ci` could not resolve `main` in an isolated JJ workspace. Classification remains `DEFERRED_GLOBAL` for Moon/JJ base-revision detection.
- Latest State 8 rerun: `moon ci` advanced past changed-file loading and emitted repo-wide format/lint failures outside this bead's delivery scope.
- Primary category recorded in `ci-failure-category.txt`: `FORMAT`.
- Blocking classification: `DEFERRED_GLOBAL`, not bead-local, because the remaining failures are in `crates/vb_proof_kernels`, `crates/vb_storage`, `fuzz`, and `xtask`, while this bead scope is the `velvet_ballistics` CLI structured-output path and its integration tests.
- Bead-local format gate now passes: `cargo +nightly fmt -p velvet_ballistics --check` exit 0.
- Bead-local compile/test gates pass after YAML repair:
  - YAML contract test: 1 passed.
  - bounded help test: 1 passed.
  - status JSON stdout-only test: 1 passed.
  - unknown command diagnostic test: 1 passed.
  - `rtk cargo check -p velvet_ballistics --all-targets`: 0 errors, 1 duplicate-package warning.
- Follow-up text: create/route a separate global formatting/lint debt bead for `crates/vb_proof_kernels`, `crates/vb_storage`, `fuzz`, and `xtask`, including `EnvelopeHeader::new_without_default`.
