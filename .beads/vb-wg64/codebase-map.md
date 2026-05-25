# vb-wg64 Codebase Map

## Isolation

- Source checkout: `/home/lewis/src/velvet-ballistics`
- Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`
- `pwd -P` evidence: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`
- The isolated workspace is outside the source checkout and is the only workspace used for State 2 reads/checks.

## Baseline Inputs

- Existing `.beads/vb-wg64/baseline-report.md` records the clean-clone forced `moon ci --base HEAD --head HEAD --force` failure.
- Known failing lanes: `fmt`, `lint-src`, and `check`.
- State 2 did not modify production or test code.

## Mapped Failure Areas

### `xtask/src/forbidden_scan.rs`

- `cargo fmt --all -- --check` reports formatting drift in this file, including import ordering, chained expressions, long `println!` calls, compact closures, and long match arms.
- `rtk cargo clippy -p xtask --all-targets -- -D warnings` reports source failures in this file:
- `ScanSummary::add_finding`: arithmetic side effects on `total_findings += 1` and map count increment.
- `resolve_crates`: explicit lifetime can be elided.
- `glob_match`: unchecked indexing into `parts[0]` and `parts[1]`.
- `scan_crate`: `line_num + 1` is an arithmetic side effect.
- `collect_rs_files`: takes `&PathBuf` where `&Path` is sufficient.
- Likely minimal fix: run rustfmt, change `&PathBuf` parameters to `&Path` where clippy demands, replace unchecked indexing with `first`/`get` matching, and use checked/saturating count handling that preserves scan behavior.

### `crates/vb_cli/src/app_impl.rs`

- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` reports `E0583` at line 4764: `mod mode_error;` has no matching module file.
- The failure is tied to the test-only `#[cfg(test)] mod mode_error;` declaration and `mode_activation_tests.rs` imports.
- Likely minimal fix: add the missing `crates/vb_cli/src/mode_error.rs` module matching the existing mode activation tests, or intentionally remove/gate the stale test module if the contract is obsolete. Adding the missing module is the lower-risk path if those tests are accepted contract.

### `crates/vb_cli/src/commands_ai_context.rs`

- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` reports collapsible nested `if` statements in `json_out` at lines 599 and 606.
- Likely minimal fix: collapse each `if let Ok(text)` plus `if let Err(error)` pair using a single conditional chain or equivalent helper while preserving current JSON/JSONL/text output behavior.

### `crates/vb_cli/src/mode_activation_tests.rs`

- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` reports `E0432` unresolved import for `crate::mode_error::{CommandMode, ModeError, command_mode}`.
- This is a downstream symptom of the missing `mode_error` module declared in `app_impl.rs`.
- Likely minimal fix is the same as `app_impl.rs`: provide or correctly gate the `mode_error` module.

### `crates/vb_storage/tests/recovery_bdd_tests.rs`

- `rtk cargo check -p vb_storage --tests` exits 0 but reports warnings that are expected to become failures under CI check settings.
- Unused imports at top of file: `Taint`, `RecoveredSlotEntry`, `is_terminal_event`, `recover_snapshot_plus_tail`, `DateTime`.
- Unused variables: `journal` at lines 294, 347, 927, 1070, 1117, 1149, 1632; `events_again` at line 632; `events_without_diagnostics` at line 1426.
- Likely minimal fix: remove unused imports where truly unused and prefix intentionally retained setup variables with `_` or remove unused bindings when no side effect is needed.

## Additional Gate Risk

- `cargo fmt --all -- --check` reported 179 formatting diff hunks across multiple crates, not only `xtask/src/forbidden_scan.rs`.
- If `velvet-ballistics:fmt` runs workspace-wide, State 10 may need a pure rustfmt-only sweep beyond the four known failure files.
- Because this state is scope-only, no formatting was applied.

## Expected Verification Gates

- Primary forced clean-clone gate: `moon ci --base HEAD --head HEAD --force`.
- Targeted preflight gates before full CI:
- `rtk cargo fmt --all -- --check`
- `rtk cargo clippy -p xtask --all-targets -- -D warnings`
- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings`
- `rtk cargo check -p vb_storage --tests`

## State 2 Command Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`.
- `rtk cargo fmt --all -- --check` failed with formatting diffs; output captured by OpenCode and RTK.
- `rtk cargo clippy -p xtask --all-targets -- -D warnings` failed; source failures include `xtask/src/forbidden_scan.rs` plus broader xtask test lint failures.
- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` failed; source failures include `app_impl.rs`, `commands_ai_context.rs`, and missing `mode_error` used by `mode_activation_tests.rs`, plus broader vb_cli test lint failures.
- `rtk cargo check -p vb_storage --tests` exited 0 with 12 warnings in `recovery_bdd_tests.rs`.
