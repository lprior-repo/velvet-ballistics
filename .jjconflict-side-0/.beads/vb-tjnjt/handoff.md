# Handoff: vb-tjnjt Test Layout Repair

## Status
Complete. All three gates pass.

## Changes (2 files)
1. `crates/vb_core/src/policy.rs` - inlined original `#[cfg(test)] mod tests` block (7 RuntimePolicy tests) in place of `#[path="policy/original_tests.rs"]` include. Production code and `new_tests` module unchanged.
2. `crates/vb_core/src/policy/original_tests.rs` - deleted (feature-branch artifact, never existed on main).

## Evidence
- `.beads/vb-tjnjt/evidence/01-scoped-tests.*` - 51 vb_core + 93 vb_runtime policy tests, all passed (144 total)
- `.beads/vb-tjnjt/evidence/02-integrity-gate.*` - test-integrity gate vs 8707f8e976f9a1f936a65341c924468be57f2920 passed (exit 0)
- `.beads/vb-tjnjt/evidence/03-fmt-check.*` - nightly-2026-04-28 fmt check passed (exit 0)

## Verification
- Inline test block matches main's `#[cfg(test)] mod tests` exactly (verified by diff)
- No changes to production code, new_tests, or any other crate
- No VCS mutations
