bead_id: vb-ssei
phase: 11
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Regression diff

Classification: `DEFERRED_GLOBAL` for canonical `moon ci` failures.

Evidence:
- `moon ci` changed files are only `crates/workspace_tests/src/acceptance_catalog.rs`, `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`, `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs`.
- `velvet-ballastics:fmt` failure reports unrelated files: `crates/vb_codegen/src/tests.rs`, `crates/vb_storage/src/kani_recovery_hydrate.rs`, `crates/vb_storage/src/recovery/recover.rs`, `crates/vb_storage/src/recovery/recovery_unit_tests.rs`, plus the new test before package formatting. After `rtk cargo fmt -p velvet-ballastics-workspace-tests`, touched package format check passes.
- `velvet-ballastics:check` failure reports unrelated `crates/vb_storage/src/recovery/recovery_unit_tests.rs` unused/dead-code issues.

No `BLOCK_LOCAL`, `BLOCK_REGRESSION`, `BLOCK_RELEASE`, or `REQUIRED_OBLIGATION_FAIL` remains for the `vb-ssei` touched scope.
