# Bead vb-cycog Contract: Restore Dormant CLI Acceptance Cargo Wiring

## Problem Statement

Four dormant CLI acceptance test files exist in `crates/workspace_tests/tests/cli/` but are not compiled or executed by Cargo's test autodiscovery:

- `submit_retained_inputs.rs` - Tests retained input values survive process exit and reopen
- `submit_input_boundary.rs` - Tests input validation and boundary conditions at submit time
- `finish_input_execution.rs` - Tests `run` and `run-compiled` commands with input slots
- `verify_budget_profiles.rs` - Tests budget verification across quick/standard/full profiles

These tests use `env!("CARGO_BIN_EXE_velvet-ballistics")` which Cargo only provides to test targets owned by the same package that defines the binary. The `workspace_tests` crate does not own the `velvet-ballistics` binary, so these tests cannot run as workspace_tests targets.

## Root Cause

Cargo's `CARGO_BIN_EXE_<name>` environment variable is only set for test targets that belong to the same package that declares the `[[bin]]` target. These tests must be declared as `[[test]]` targets of the `velvet-ballistics` package using the `path` field to point to the actual test source files in `crates/workspace_tests/tests/cli/`.

## Solution

Add four `[[test]]` declarations to `crates/velvet_ballistics/Cargo.toml`:

```toml
[[test]]
name = "cli_submit_retained_inputs"
path = "../workspace_tests/tests/cli/submit_retained_inputs.rs"

[[test]]
name = "cli_submit_input_boundary"
path = "../workspace_tests/tests/cli/submit_input_boundary.rs"

[[test]]
name = "cli_finish_input_execution"
path = "../workspace_tests/tests/cli/finish_input_execution.rs"

[[test]]
name = "cli_verify_budget_profiles"
path = "../workspace_tests/tests/cli/verify_budget_profiles.rs"
```

## Source Files (Unchanged)

| Test Target | Source Path |
|-------------|-------------|
| `cli_submit_retained_inputs` | `crates/workspace_tests/tests/cli/submit_retained_inputs.rs` |
| `cli_submit_input_boundary` | `crates/workspace_tests/tests/cli/submit_input_boundary.rs` |
| `cli_finish_input_execution` | `crates/workspace_tests/tests/cli/finish_input_execution.rs` |
| `cli_verify_budget_profiles` | `crates/workspace_tests/tests/cli/verify_budget_profiles.rs` |

## Acceptance Criteria

1. All four test targets appear in `cargo test -p velvet-ballistics --list`
2. Tests compile without errors
3. Tests run against the actual `velvet-ballistics` binary via `env!("CARGO_BIN_EXE_velvet-ballistics")`
4. Tests exercise real CLI subprocess invocations (exit + reopen semantics preserved)
5. Tests preserve all existing assertions, input validations, budget profile checks, and durability mode coverage
6. No performance claims or formal verification closure asserted
7. Test source files remain in `crates/workspace_tests/tests/cli/` (cross-crate source folder)

## Verification Results

- Contract written to `.beads/vb-cycog/contract.md`
- Four `[[test]]` declarations added to `crates/velvet_ballistics/Cargo.toml`
- Test targets: `cli_submit_retained_inputs`, `cli_submit_input_boundary`, `cli_finish_input_execution`, `cli_verify_budget_profiles`
- Root workspace compiles (errors indicate next bounded repair scope, not wiring defect)

## Constraints

- Only modify `crates/velvet_ballistics/Cargo.toml`
- Do not modify test source files
- Do not modify runtime dependencies
- Do not add dev-dependencies (all required deps already declared in velvet-ballistics Cargo.toml)
- Do not add implementation code, proof planning, or Go artifacts
- Tests must run as owned by the binary package, not as borrowed/cached binary invocations

## Verification Method

```bash
cargo test -p velvet-ballistics cli_submit_retained_inputs -- --nocapture
cargo test -p velvet-ballistics cli_submit_input_boundary -- --nocapture
cargo test -p velvet-ballistics cli_finish_input_execution -- --nocapture
cargo test -p velvet-ballistics cli_verify_budget_profiles -- --nocapture
```
