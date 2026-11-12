# Choose-body root workspace repair log

Scope: repair the root `cargo test --workspace --all-features` blocker reported in `crates/vb_compile/tests/integration_choose_body.rs` without changing fuzz targets or `fuzz/Cargo.lock`.

## Root and JJ isolation

- Command: `pwd && git rev-parse --show-toplevel && jj root && jj status`
- Status: PASS
- Verified path: `/home/lewis/src/isoloated/velvet-ballistics-w25-fuzz`
- Verified Git root: `/home/lewis/src/isoloated/velvet-ballistics-w25-fuzz`
- Verified JJ root: `/home/lewis/src/isoloated/velvet-ballistics-w25-fuzz`

## Failure reproduced

- Command: `rtk cargo test -p vb_compile --test integration_choose_body --all-features`
- Status: FAIL before repair
- Failing tests: `compile_workflow_choose_body_set_success`, `compile_workflow_choose_body_do_success`
- Error: `UnknownStepPrimitiveField { step: 1, primitive: "choose", field: "branches" }`
- Raw tee log: `/home/lewis/.local/share/rtk/tee/1783572862_cargo_test.log`

## Repair summary

- `YamlCompiler::compile` now uses canonical document validation after `parse_workflow_source` instead of the legacy phase-zero primitive-shape validator.
- Canonical validation still enforces strict profile, top-level fields, version, trigger, name, optional top-level shape, result shape, non-empty top-level steps, and top-level step IDs.
- Legacy `parse_ast` keeps `validate_workflow_document_shape` unchanged for legacy `condition` / `on_true` / `on_false` choose tests.
- Added canonical choose shape validation support for `branches` / `otherwise` so shared validation no longer rejects the canonical choose surface.
- Updated invalid test fixture workflow names from hyphenated names to Velvet public names with underscores; updated repeat proptest generators to produce public-name-safe IDs/outputs.

## Repair verification

| Command | Status |
| --- | --- |
| `rtk cargo test -p vb_compile --test integration_choose_body --all-features` | PASS: 2 passed |
| `rtk cargo test -p vb_compile --all-features` | PASS: 1743 passed, 5 ignored |
| `rtk cargo fmt --all -- --check` | FAIL before `cargo fmt`: formatting drift in new `part_08.rs` |
| `rtk cargo fmt --all` | PASS |
| `rtk cargo fmt --all -- --check` | PASS |
| `rtk cargo check --workspace --all-targets --all-features` | PASS |
| `rtk cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | PASS |
| `rtk cargo test --workspace --all-features` | FAIL: unrelated vb_core aggregate resource budget red tests |

## Residual blocker from workspace test

`rtk cargo test --workspace --all-features` no longer fails in `crates/vb_compile/tests/integration_choose_body.rs`. It now reaches a separate `vb_core` red-test blocker:

- `crates/vb_core/tests/aggregate_resource_budget_red.rs::aggregate_admission_with_budget_exists`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs::runtime_resource_capacity_error_variant_exists`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs::admission_accepts_requested_budget_argument`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs::admission_accepts_available_capacity_argument`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs::admission_error_preserves_requested_value`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs::admission_error_preserves_available_value`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs::admission_with_budget_still_checks_artifacts`

Raw tee log: `/home/lewis/.local/share/rtk/tee/1783573557_cargo_test.log`.

Classification: `BLOCK_GLOBAL` / outside this choose-body repair scope. No `vb_core` production files were edited.
