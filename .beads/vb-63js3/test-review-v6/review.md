# vb-63js3 Test Review v6

## Scope

Review of 4 new Flux routing unit tests added to `xtask/src/lanes.rs` (lines 207-264). The tests verify command-line argument construction for the Flux verification lane, specifically that paths containing shell metacharacters are passed as single argv elements rather than shell-expanded.

## Source Read

Read complete `xtask/src/lanes.rs` (265 lines) from three source packets: source-001.txt, source-101.txt, source-201.txt. Read actual-source.patch showing the diff. Read actual execution logs ROOT_FLUX_ROUTING_V4_TESTS.stdout and .json.

## New Tests

Four new tests added:

1. `test_flux_command_uses_wrapper_script` (lines 207-219): Verifies the Flux lane uses the wrapper script at `/workspace/scripts/flux-check-package.sh` with package name `vb_core`. Full Vec equality assertion with 3 elements: `["bash", "/workspace/scripts/flux-check-package.sh", "vb_core"]`.

2. `test_flux_command_workspace_root_with_spaces` (lines 221-234): Verifies path with spaces `/home/user/my workspace` is passed as single argv element without shell splitting.

3. `test_flux_command_workspace_root_with_metacharacters` (lines 236-249): Verifies path with shell metacharacters `/tmp/$HOME (test) & co` is passed as-is without shell expansion. This is the key shell-injection protection test.

4. `test_flux_command_package_metacharacters_retained_as_one_argv` (lines 251-264): Verifies package name metacharacters `vb_core;$(printf nope)` are retained as a single argv element. Tests the package name argument, not just the path.

## Old Tests

Three existing tests remain unchanged:
- `test_lane_command_test` (lines 178-186)
- `test_lane_command_clippy` (lines 189-197)
- `test_required_lanes_always_available` (lines 200-204)

## Implementation Change

The patch replaced inline Flux command construction:
```rust
"flux" => vec!["cargo", "flux", "-p", crate_name],
```
with a dedicated helper function that constructs the command as separate argv elements:
```rust
"flux" => flux_command(crate_name, workspace_root),
```

The helper builds `["bash", script_path, crate_name]` where script_path is constructed via `Path::join`, not string concatenation. This is the correct approach for shell-injection protection because `std::process::Command` passes each element of the argv vector directly to `execvp`, bypassing shell parsing entirely.

## Actual Execution Results

From ROOT_FLUX_ROUTING_V4_TESTS.stdout (actual execution log, not inferred):
```
running 7 tests
test lanes::tests::test_flux_command_package_metacharacters_retained_as_one_argv ... ok
test lanes::tests::test_flux_command_uses_wrapper_script ... ok
test lanes::tests::test_flux_command_workspace_root_with_spaces ... ok
test lanes::tests::test_flux_command_workspace_root_with_metacharacters ... ok
test lanes::tests::test_lane_command_test ... ok
test lanes::tests::test_lane_command_clippy ... ok
test lanes::tests::test_required_lanes_always_available ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 64 filtered out; finished in 0.20s
```

From ROOT_FLUX_ROUTING_V4_TESTS.json:
- exit_code: 0
- argv: `["env", "RUSTFLAGS=-Dwarnings", "RUSTC_WRAPPER=", "cargo", "test", "--manifest-path", "xtask/Cargo.toml", "--lib", "lanes::tests", "--locked"]`

All 7 tests pass. All 3 old tests pass. All 4 new tests pass.

## Test Quality Assessment

### Strengths

1. **Full Vec equality assertions**: All 4 new tests use `assert_eq!(cmd, expected)` with complete 3-element expected vectors. No element-level or substring checks.

2. **Correct edge case coverage**: Tests cover the three critical shell-injection scenarios:
   - Basic path (baseline)
   - Spaces in path (shell word splitting)
   - Shell metacharacters in path (`$HOME`, parentheses, `&`)
   - Shell metacharacters in package name (`;`, `$()`)

3. **Correct verification target**: Tests verify argv element count and exact values, not the string representation of the command. This is the correct way to test shell-injection protection.

4. **Independent test cases**: Each test uses a different workspace root path and/or package name. No shared mutable state.

5. **Physical line count**: Each test is 13-16 physical lines. The `flux_command` helper function is 9 lines. Total added: ~65 lines (patch shows +64 in test module).

### Limitations

1. **Command construction only, not execution**: Tests verify the argv vector is built correctly but do not actually execute the shell command. The wrapper script itself is not tested. A bug in `scripts/flux-check-package.sh` would not be caught.

2. **Bounded scope**: Only the Flux lane is tested for shell-injection protection. The other 11 lane types (test, clippy, nextest, kani, miri, loom, fuzz, mutants, coverage, verus, tla) are not tested for the same issue. The `verus_command` and `tla_command` functions use `format!` strings that could theoretically have similar issues, but they are not tested here.

3. **No error-path testing**: Tests do not verify behavior when the script path does not exist, the package name is empty, or other error conditions occur.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| F1 | Info | Tests correctly use full Vec equality assertions with exact 3-element argv arrays |
| F2 | Info | All 3 old tests remain unchanged and pass |
| F3 | Info | Actual execution log confirms 7 passed, 64 filtered out, 0 ignored, exit 0 |
| F4 | Limitation | Tests verify command construction only, not shell execution of wrapper script |
| F5 | Limitation | Only Flux lane tested; other 11 lanes not tested for shell-injection protection |
| F6 | Limitation | No error-path or edge-case testing for empty package names or missing script paths |

## Verdict

**APPROVE** (bounded test source only)

The 4 new tests are correctly written and pass. They use the proper assertion style (full Vec equality), cover the critical shell-injection edge cases for the Flux lane, and are verified against actual execution logs showing all 7 tests pass.

The review is bounded to the test source and the specific change to the Flux lane command construction. It does not constitute a comprehensive review of all 12 lane types or the actual shell script execution.

Source frozen at commit 00cd (full SHA from contract manifest). Docs do not require new requirements.