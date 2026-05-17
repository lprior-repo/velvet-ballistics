# Test Plan: MAJOR-2 — trybuild Silent Pass

## Summary

- Behaviors identified: 2
- Trophy allocation: 2 integration / 0 unit / 0 e2e / 0 static
- Proptest invariants: 0
- Fuzz targets: 0
- Kani harnesses: 0

## 1. Behavior Inventory

1. "trybuild test suite reports failure when compile-fail/ directory is empty"
2. "trybuild test suite detects expected compilation failures and reports unexpected compiler errors as failures"

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|----------|-------|-----------|
| Empty compile-fail/ detection | Integration | Requires real filesystem interaction; cannot be unit tested without faking FS |
| Compile-fail fixture validation | Integration | trybuild executes real compiler; integration by definition |

No deviation from target ratios. Both behaviors require integration-level testing with the real filesystem and compiler.

## 3. BDD Scenarios

### Behavior 1: trybuild fails when compile-fail/ is empty

**Scenario: empty compile-fail directory**

```
fn trybuild_reports_failure_when_compile_fail_directory_is_empty()
```

Given: The `compile-fail/` directory exists but contains no `.rs` files
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` containing a message indicating no compile-fail cases were found
And: The error message references the fixtures directory path

**Scenario: missing compile-fail directory**

```
fn trybuild_reports_failure_when_compile_fail_directory_is_missing()
```

Given: The `compile-fail/` directory does not exist
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` indicating the fixtures directory cannot be read
And: The error is a filesystem error (not silently swallowed)

**Scenario: compile-fail directory contains only non-.rs files**

```
fn trybuild_reports_failure_when_compile_fail_contains_no_rust_files()
```

Given: The `compile-fail/` directory exists but contains only `.stderr` files (no `.rs` files)
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` containing a message indicating no compile-fail cases were found

---

### Behavior 2: trybuild detects expected compilation failures

**Scenario: single compile-fail fixture compiles with expected error**

```
fn trybuild_passes_when_compile_fail_fixture_matches_expected_error()
```

Given: `compile-fail/` contains one valid compile-fail fixture (e.g., `forbid_unsafe.rs`) with a matching `.stderr` file
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Ok(())`
And: trybuild confirms the fixture produced the expected compiler error

**Scenario: compile-fail fixture produces unexpected error**

```
fn trybuild_reports_failure_when_compile_fail_fixture_produces_unexpected_error()
```

Given: `compile-fail/` contains a compile-fail fixture whose actual error does not match the `.stderr` file
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` with details about the mismatch
And: The error includes the fixture path and expected vs actual error diff

**Scenario: compile-fail fixture compiles successfully (should have failed)**

```
fn trybuild_reports_failure_when_compile_fail_fixture_incorrectly_passes()
```

Given: `compile-fail/` contains a fixture that uses `#![forbid(unsafe_code)]` but has no unsafe code
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` indicating the fixture unexpectedly compiled when a failure was expected

**Scenario: compile-fail fixture is missing .stderr file**

```
fn trybuild_reports_failure_when_compile_fail_fixture_is_missing_stderr()
```

Given: `compile-fail/` contains a `.rs` file with no corresponding `.stderr` file
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` indicating the expected error file is missing

**Scenario: multiple compile-fail fixtures — all valid**

```
fn trybuild_passes_when_all_compile_fail_fixtures_match()
```

Given: `compile-fail/` contains multiple valid compile-fail fixtures (e.g., `forbid_unsafe.rs`, `forbid_unwrap.rs`, `forbid_panic.rs`) each with matching `.stderr` files
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Ok(())`

**Scenario: multiple compile-fail fixtures — one invalid**

```
fn trybuild_reports_failure_when_any_compile_fail_fixture_mismatches()
```

Given: `compile-fail/` contains multiple fixtures, one of which produces an unexpected error
When: `trybuild_compile_fail_tests()` is executed
Then: The test result is `Err` identifying the failing fixture by path

## 4. Proptest Invariants

Not applicable. No pure functions with multiple inputs in this test harness.

## 5. Fuzz Targets

Not applicable. No parsing or deserialization boundaries in the test harness itself.

## 6. Kani Harnesses

Not applicable. No critical invariants requiring formal verification; behavior is driven by side effects (filesystem + compiler).

## 7. Mutation Checkpoints

Critical mutations to survive:
- The `return Ok(())` in the empty-directory branch must be caught by `trybuild_reports_failure_when_compile_fail_directory_is_empty`
- The early return pattern `if fixture_files.is_empty() { return Ok(()); }` must be caught — removing the early return must cause test failure

Threshold: 100% (both mutations must be caught — the entire point of this fix is to eliminate silent pass)

## 8. Combinatorial Coverage Matrix

| Scenario | Input Condition | Expected Output | Test Layer |
|----------|-----------------|-----------------|------------|
| empty dir | compile-fail/ has no .rs files | Err("no compile-fail cases") | integration |
| missing dir | compile-fail/ does not exist | Err(filesystem error) | integration |
| non-rs files only | compile-fail/ has only .stderr | Err("no compile-fail cases") | integration |
| single valid fixture | 1 .rs with matching .stderr | Ok(()) | integration |
| unexpected error | .rs error ≠ .stderr | Err(mismatch details) | integration |
| unexpected pass | .rs compiles but should fail | Err("unexpected pass") | integration |
| missing stderr | .rs exists, .stderr missing | Err("missing stderr") | integration |
| all valid | N fixtures all match | Ok(()) | integration |
| one invalid | N fixtures, 1 mismatch | Err(failing path) | integration |

## 9. Open Questions

1. Should the empty-directory error message be a specific error type (e.g., `NoCompileFailFixturesFound`) or a generic string error?
2. Should the test use `Result<(), String>` (current) or `Result<(), Box<dyn Error>>` for better error categorization?
3. Should there be a distinction between "directory missing" (fatal) vs "directory empty" (also fatal, same severity)?
