# vb-wg64 Baseline Report

Truth-serum after P0 landing proved:

- `bd list --priority 0` returned no issues before this repair bead was opened.
- `origin/main` was at `796ca1be docs: finalize vb-qi37.5.3 landing state` after fetch.
- `bd dolt push` succeeded.
- A clean clone forced gate `moon ci --base HEAD --head HEAD --force` failed with exit 1.

Known failing lanes from direct evidence:

1. `velvet-ballastics:fmt`: formatting drift, including `xtask/src/forbidden_scan.rs`.
2. `velvet-ballastics:lint-src`: clippy failures in `xtask/src/forbidden_scan.rs` and `crates/vb_cli/*`.
3. `velvet-ballastics:check`: unused imports/variables in `crates/vb_storage/tests/recovery_bdd_tests.rs`.
4. `velvet-ballastics:miri`: passed scoped checks in the truth-serum run.

Acceptance:

- Clean current-main workspace runs `moon ci --base HEAD --head HEAD --force` with exit 0.
- Repair lands on `origin/main`.
- `bd close vb-wg64 --force` succeeds.
- `bd dolt push` succeeds.
