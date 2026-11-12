STATUS: PASS

## Scope

- First actionable State 8 failure: `velvet-ballistics:fmt` rustfmt drift.
- Repair performed by running rustfmt only; no manual semantic Rust edits were made.
- Behavior preservation: only formatting was changed by this repair, plus this evidence report was added.

## Commands Run

1. `bd prime`
   - PASS: loaded beads workflow context.

2. `rustup run nightly-2026-04-28 cargo fmt --all`
   - PASS: command exited successfully with no output.

3. `moon run velvet-ballistics:fmt`
   - PASS summary:
     - `▮▮▮▮ velvet-ballistics:fmt (968ms, 2f6e0872)`
     - `Tasks: 1 completed`
     - `Time: 1s 969ms`

4. `moon ci --base HEAD --head HEAD`
   - FAIL overall on later non-format tasks, after `velvet-ballistics:fmt` passed.
   - PASS summary for fmt within CI:
     - `▮▮▮▮ velvet-ballistics:fmt (1s 25ms, f49dad0e)`
   - Overall summary:
     - `Tasks: 10 completed, 3 failed, 7 skipped`
     - `Time: 4m 11s 404ms`
   - Full captured output: `/home/lewis/.local/share/opencode/tool-output/tool_e0fba7312001ydKljVr8naa6H2`

## Files Changed By Formatting Repair

Rustfmt was applied workspace-wide. The resulting Rust-file working-copy changes include:

- `crates/vb_compile/src/lib.rs`
- `crates/vb_runtime/src/collect_tests.rs`
- `crates/vb_storage/src/recovery/replay/summary.rs`
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs`
- `crates/vb_ui_model/src/envelope.rs`
- `crates/vb_ui_snapshot/src/report.rs`
- `crates/velvet_ballistics/src/main.rs`
- `crates/velvet_ballistics/tests/cli_integration.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `tests/vb_qi37_1_1_red_recovery_contract_test.rs`
- `xtask/src/evidence.rs`
- `xtask/src/gates.rs`
- `xtask/src/main.rs`
- `xtask/tests/integration_gates.rs`

Added evidence file:

- `.beads/vb-nf2u/state8-format-repair.md`

## Explicit Formatting-Only Statement

Only repository formatter output was applied to Rust source by this repair. I did not hand-edit Rust logic, dependencies, tests, or supply-chain configuration.

## Residual Non-Format Failures

`moon ci --base HEAD --head HEAD` still fails on later, non-format failure classes. Per State 8 boundary, these were not repaired:

- `velvet-ballistics:supply-chain`: `ERROR × Couldn't acquire the store` / `No such file or directory (os error 2)` after `advisories ok, bans ok, licenses ok, sources ok`.
- `velvet-ballistics:lint-src`: clippy failures in fuzz binaries, including `clippy::bool-assert-comparison` and `clippy::let_underscore_must_use`.
- `velvet-ballistics:miri`: `vb_validate` proptest path attempted `std::env::current_dir`, reported as unsupported by Miri, causing `test failed, to rerun pass -p vb_validate --lib`.
