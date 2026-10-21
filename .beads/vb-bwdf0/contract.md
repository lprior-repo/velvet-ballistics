# vb-bwdf0: Restore input-slot schema in stale test fixtures

## Problem
Fresh isolated test compilation fails because `WorkflowParts` struct literals
in compiler and runtime integration tests are missing the `input_slots` field,
added by a recent compiler change. The production `WorkflowParts` struct
requires `input_slots: Box<[CompiledInputSlot]>` with `#[serde(default)]`
for backward compatibility, but Rust struct literals still require it.

## Scope
Test fixture drift only. Repair stale `WorkflowParts` literals in:
- `crates/vb_compile/tests/vb_xi2f_error_variant_proptest.rs` (lines 37, 153, 199)
- `crates/vb_runtime/tests/durability_matrix_integration.rs` (lines 81, 109, 156, 224)
- Any other test files that fail to compile due to this drift

## Constraints
- No production behavior changes
- No ignored tests or weakened assertions
- Input slots must match the actual fixture's declared input kinds
- Use empty boxed slice `Box::new([])` where a workflow truly has no declared external input
- No `cfg(test)` replacements or invented capabilities/values
- No modification of `proptest_finish_digest.rs` or its contract

## Acceptance
Both packages compile their test suite:
```bash
cargo test -p vb_compile --features test-util --tests --no-run
cargo test -p vb_runtime --tests --no-run
```
Then run the actual impacted integration tests and report results.
