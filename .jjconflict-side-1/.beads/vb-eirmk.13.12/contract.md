# Contract: stable prebuilt CLI execution for scope tests

## Goal

When the parent acceptance run supplies `CARGO_BIN_EXE_velvet-ballistics`, each covered `vb_nf2u_current_scope_tests` scenario executes that exact prebuilt CLI path. No nested Cargo invocation may build or replace the acceptance binary in this mode. A supplied invalid path fails visibly and never falls back to `cargo run`.

## Existing behavior and scope

`run_cli` in `crates/workspace_tests/tests/vb_nf2u_current_scope_tests.rs:20-34` currently invokes `cargo run -p velvet-ballistics`; all five callers are `removed_ui_command_is_refused_with_unknown_command_error`, `removed_ui_release_command_is_not_registered`, `cli_help_lists_current_command_surface`, `cli_version_prints_version_string`, and `validate_command_runs_on_valid_workflow` (lines 63, 77, 90, 114, 124). Preserve each scenario's argv, status checks, stdout/stderr checks, fixture behavior, and names.

Reuse the existing workspace CLI selection convention from `vb_binary` in `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs:15-24` and the `CARGO_BIN_EXE_velvet-ballistics` invocation in `crates/velvet_ballistics/tests/cli_integration.rs:113-120`. `vb_binary` is private to a separate integration-test target, so this contract requires its precedence semantics, not cross-target access to that function: any supplied OS path is authoritative; absence alone permits the existing standalone fallback. Do not change unrelated callers or introduce a public application API.

Scope is the `vb_nf2u` selector/runner and its five callers, plus the nearest existing test-binary selection pattern used to align behavior. No production CLI, argv, output, dependency, build gate, Moon configuration, or unrelated test changes.

## Selection contract

- Distinguish `Prebuilt(path)` from the existing standalone Cargo-run fallback before launching a process; do not infer absence from a failed Unicode conversion or path validation.
- If `CARGO_BIN_EXE_velvet-ballistics` is present, preserve its `OsString` path and execute it directly with the original CLI arguments. Do not invoke Cargo, search `PATH`, canonicalize into a different executable, or substitute `target/debug/velvet-ballistics`.
- If that variable is present but the path cannot be launched, return/report the path-specific error through the existing `Result`/error-output flow. Never silently fall back.
- If the variable is absent, preserve the existing standalone `cargo run` behavior needed by these five tests. Do not add a new profile or feature selection.
- Preserve process status and captured stdout/stderr semantics. Do not rewrite or weaken the five scenario assertions.

## Behavior evidence required for implementation and TEST

- A pure selector regression test supplies a distinctive explicit path and proves the selected mode is that exact prebuilt path; it must fail against the current always-`cargo run` selector without spawning Cargo or executing the old path. This is the safe baseline-red oracle and cannot overwrite the shared profile binary.
- A pure selector case for an absent path proves standalone fallback remains available. A supplied invalid path remains classified as explicit and yields a visible error; the test must not launch Cargo as a fallback.
- Run all five unchanged named scenarios with the parent-supplied valid prebuilt path and confirm their original public behavior assertions remain intact. The runner must execute the explicit path in every scenario.
- For the selected Moon acceptance run, record the exact candidate/profile and compare the prebuilt CLI SHA-256 immediately before and after. Require the hash to remain identical; report test totals and skips exactly, without claiming full-CI acceptance.

## Constraints and non-goals

No source/API/schema/dependency/gate/config changes, test deletion, assertion weakening, Cargo rebuild during explicit-path scenarios, environment mutation shared across parallel tests, root `/tmp` access, database access, commit, or push. This contract defines behavior only; it does not claim the prior hash writer or feature-unification cause is established.
