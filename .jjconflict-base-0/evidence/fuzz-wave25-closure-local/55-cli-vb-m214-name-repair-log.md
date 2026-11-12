# CLI vb-m214 valid-workflow name repair log

Scope: repair the remaining root `cargo test --workspace --all-features` blocker reported in `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` without changing CLI behavior, canonical language version, Fjall dependency policy, or `fuzz/Cargo.lock`.

## Root and JJ isolation

- Command: `pwd && git rev-parse --show-toplevel && jj root && git status --short --branch && jj status`
- Status: PASS
- Verified path: `/home/lewis/src/isoloated/velvet-ballistics-w25-fuzz`
- Verified Git root: `/home/lewis/src/isoloated/velvet-ballistics-w25-fuzz`
- Verified JJ root: `/home/lewis/src/isoloated/velvet-ballistics-w25-fuzz`

## Failure reproduced

- Command: `cargo test -p velvet-ballistics --test cli_vb_m214_bdd_scenarios --all-features exit_code_tests::exit_code_zero_on_success -- --nocapture`
- Status: FAIL before repair
- Raw log: `41-cli-vb-m214-exit-code-zero-before-repair-raw.txt`
- Diagnostic command: `./target/debug/velvet-ballistics validate target/tmp/vb-cli-valid-workflow.yaml`
- Status: FAIL before repair
- Raw log: `42-cli-validate-minimal-workflow-manual-before-repair.txt`
- Root cause: the shared BDD fixture claimed to be a valid Velvet v1 workflow while using `name: test-workflow`; previous canonical compile validation correctly requires public names to contain lowercase ASCII, digits, or underscores only, so the CLI returned validation exit code 2.

## Repair summary

- `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs`: changed the valid fixture workflow name from `test-workflow` to `test_workflow`.
- `crates/vb_cli/tests/cli_integration.rs`: changed the same stale valid fixture and expected parsed name to `test_workflow`.
- CLI implementation behavior was not weakened; the canonical language version remains `velvet-ballistics/v1`, and canonical public-name validation remains strict.

## Repair verification

| Command | Status | Raw log |
| --- | --- | --- |
| `cargo test -p velvet-ballistics --test cli_vb_m214_bdd_scenarios --all-features exit_code_tests::exit_code_zero_on_success -- --nocapture` | PASS | `44-cli-vb-m214-exit-code-zero-after-name-repair.txt` |
| `cargo test -p velvet-ballistics --test cli_vb_m214_bdd_scenarios --all-features bdd_scenarios::cli_explain_valid_workflow_emits_diagnostic_details -- --nocapture` | PASS | `45-cli-vb-m214-explain-valid-after-name-repair.txt` |
| `cargo test -p velvet-ballistics --test cli_vb_m214_bdd_scenarios --all-features bdd_scenarios::cli_graph_valid_workflow_emits_dot_format -- --nocapture` | PASS | `46-cli-vb-m214-graph-valid-after-name-repair.txt` |
| `cargo test -p velvet-ballistics --test cli_vb_m214_bdd_scenarios --all-features exit_code_tests::exit_code_seven_on_action_policy_error -- --nocapture` | PASS | `47-cli-vb-m214-action-policy-after-name-repair.txt` |
| `cargo test -p velvet-ballistics --test cli_vb_m214_bdd_scenarios --all-features` | PASS: 44 passed | `48-cargo-test-vb-cli-m214-bdd-all-after-name-repair.txt` |
| `cargo test -p velvet-ballistics --test cli_integration --all-features yaml_parse_valid_minimal_workflow -- --nocapture` | PASS | `49-cli-integration-valid-minimal-after-name-repair.txt` |
| `cargo fmt --all -- --check` | PASS | `52-cargo-fmt-check-after-cli-name-repair.txt` |
| `cargo check -p velvet-ballistics --all-targets --all-features` | PASS | `53-cargo-check-vb-cli-all-targets-all-features-after-cli-name-repair.txt` |
| `cargo clippy -p velvet-ballistics --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | PASS | `54-cargo-clippy-vb-cli-source-strict-after-cli-name-repair.txt` |

## Workspace retest

- Command: `cargo test --workspace --all-features`
- Status: FAIL after CLI repair, but no longer in `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs`.
- Raw log: `50-cargo-test-workspace-all-features-after-cli-name-repair.txt`
- New residual: `crates/vb_runtime/tests/recovery_hydration_tests.rs::checkpoint_snapshot_roundtrip_preserves_all_fields` hit `ProcessLockHeld` on a Fjall `.process.lock` during default parallel test execution.
- Targeted rerun command: `cargo test -p vb_runtime --test recovery_hydration_tests --all-features checkpoint_snapshot_roundtrip_preserves_all_fields -- --nocapture`
- Targeted rerun status: PASS
- Targeted rerun raw log: `51-vb-runtime-recovery-hydration-checkpoint-targeted-after-workspace-fail.txt`

Classification: CLI blocker repaired. The remaining full-workspace failure is a `BLOCK_GLOBAL` runtime storage-lock test flake/residual outside this CLI fixture repair.
