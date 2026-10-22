# Bead: vb-tjnjt - Test Layout Repair

## Scope
Restore original test layout so test-integrity gate against main passes.
- Move seven original RuntimePolicy tests from `crates/vb_core/src/policy/original_tests.rs` inline into `crates/vb_core/src/policy.rs` as `#[cfg(test)] mod tests`
- Remove `crates/vb_core/src/policy/original_tests.rs` (created on feature branch, never existed on main)
- All test bodies remain byte-identical to original main commit
- No changes to production code, new_tests module, or any other files

## Files
1. `crates/vb_core/src/policy.rs` - inline original tests
2. `crates/vb_core/src/policy/original_tests.rs` - delete

## Evidence
- Full scoped test run (policy tests, vb_core + vb_runtime)
- Test-integrity gate against base 8707f8e976f9a1f936a65341c924468be57f2920
- Pinned nightly format check

## Constraint
Test-layout only. No algorithm, type, evaluator, or new test behavior changes.
