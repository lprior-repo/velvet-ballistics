bead_id: vb-qi37.4.4
phase: State 8 rerun after State 13 refactor
updated_at: 2026-05-11

# Moon Report

## Commands
- `rtk cargo test -p vb_runtime runtime_error --lib && rtk cargo test -p velvet_ballastics --test admission_durability_code`: PASS; 19 `vb_runtime` tests passed, 1297 filtered; 1 integration test passed.
- `moon run :quick`: PASS; rerun after refactor returned success.
- `moon run :test`: PASS; nextest summary reported 9831 tests passed, 0 skipped.
- `moon ci`: NON-ZERO; output saved at `/home/lewis/.local/share/opencode/tool-output/tool_e19d661b8001uZi7XItEOpIKJ6`.

## Classification
- Bead-local focused tests, `moon run :quick`, `moon run :test`, and `source-length` passed after the State 13 split.
- `moon ci` red items are outside the current delivery scope: workspace `fmt` diffs in unrelated `vb_proof_kernels`/`vb_storage`/`xtask`/`fuzz` files, `lint-src` `clippy::new_without_default` in `crates/vb_proof_kernels/src/envelope_header.rs`, and `feature-powerset` no-std failures in `crates/vb_ui_model`.
- Blocking classification for current bead-local code: none found in State 8 rerun evidence.
