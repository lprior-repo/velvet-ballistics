# Martin Fowler Test Plan

## Happy Path Tests
- `test_vb_ipc_compiles_cleanly` -- `cargo check -p vb_ipc` exits 0 with zero errors.
- `test_workspace_tests_compiles_cleanly` -- `cargo check -p velvet-ballistics-workspace-tests --tests` exits 0 with zero errors.
- `test_clippy_passes_with_zero_warnings` -- `cargo clippy -p vb_ipc -- -D warnings` exits 0.

## Error Path Tests
- `test_string_literal_in_handler_produces_e0308` -- If a String literal is used where an enum variant is expected, the compiler produces E0308 (this is the bug being fixed, not a test to keep green).
- `test_orphaned_file_with_mod_rs_breaks_build` -- If an orphaned file is accidentally wired via a new `mod.rs`, compilation may fail; this test verifies the absence of such a `mod.rs`.

## Edge Case Tests
- `test_handlers_file_contains_no_unsafe` -- `#![forbid(unsafe_code)]` is present and `grep` finds no `unsafe` keyword.
- `test_handlers_file_contains_no_panicking_apis` -- `grep` finds no `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` in the changed code.
- `test_all_five_enums_have_variant_usage_in_handlers` -- At least one usage of each enum (`EdgeType`, `PassFail`, `GateKind`, `NodeKind`, `TaintPathStatus`) is found in `handlers.rs`.

## Contract Verification Tests
- `test_precondition_clean_checkout` -- No uncommitted changes exist that could interfere with compilation.
- `test_postcondition_vb_ipc_zero_errors` -- `cargo check -p vb_ipc` exits 0.
- `test_postcondition_workspace_tests_zero_errors` -- `cargo check -p velvet-ballistics-workspace-tests --tests` exits 0.
- `test_postcondition_no_safety_regression` -- No new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` in the diff.
- `test_invariant_type_consistency` -- All IPC payload construction in `handlers.rs` uses typed enum variants.
- `test_invariant_compilation_isolation` -- Orphaned files remain unreferenced by the module tree.

## Given-When-Then Scenarios

### Scenario 1: Compile Fix Restores Type Consistency
**Given:**
- The `vb_ipc` crate is checked out at the fixed revision.
- The Rust toolchain matches the repository's pinned nightly.
- `handlers.rs` contains enum variant references instead of String literals.

**When:**
- `cargo check -p vb_ipc` is executed.
- `cargo check -p velvet-ballistics-workspace-tests --tests` is executed.
- `cargo clippy -p vb_ipc -- -D warnings` is executed.

**Then:**
- All three commands exit with code `0`.
- No E0308 errors are emitted.
- No clippy warnings are emitted.
- No new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` appear in `handlers.rs`.

### Scenario 2: Orphaned Files Remain Inert
**Given:**
- Four orphaned files exist in `crates/vb_ipc/src/server/handlers/` (`command.rs`, `event.rs`, `query.rs`, `session.rs`).
- No `mod.rs` exists in that directory.

**When:**
- `cargo check -p vb_ipc` is executed.

**Then:**
- The command exits with code `0`.
- The orphaned files are not compiled and produce no errors.
