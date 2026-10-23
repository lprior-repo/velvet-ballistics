# vb-8eocj.2.1 Contract

## Scope
Relocate three scalar test modules into a subdirectory to prevent erroneous standalone
integration target discovery by Cargo. No behavior or assertion changes.

## Files to move (unchanged content)
- tests/do_action.rs -> crates/workspace_tests/tests/vb_f1_canonical_scalar_do/do_action.rs
- tests/negative.rs -> crates/workspace_tests/tests/vb_f1_canonical_scalar_do/negative.rs
- tests/positive.rs -> crates/workspace_tests/tests/vb_f1_canonical_scalar_do/positive.rs

## Entrypoint updates
Add #[path] attributes to existing mod declarations in tests/vb_f1_canonical_scalar_do.rs.
Keep mod vb_f1_scalar_do_support declaration unchanged.

## Verification
SHA256 identity of moved content before and after. No Cargo/metadata changes in this lane.
