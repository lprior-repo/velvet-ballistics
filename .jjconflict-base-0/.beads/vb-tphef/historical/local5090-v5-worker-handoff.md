# VB-TPEHF Handoff

## Summary
Completed bounded repair of the typed policy foundation (VB-TPEHF). Fixed four remaining defects:

1. **compute_input_digest**: Replaced `Vec::with_capacity(256)` with streaming `blake3::Hasher::new()`. The hasher updates borrowed input bytes directly; `push_ticket_parts` now accepts `&mut blake3::Hasher`. No payload copy or heap buffer.

2. **evaluate**: Consolidated into single DecisionRecord construction. Added `evaluate_layer` helper that chooses outcome/layer/influence first; `evaluate` builds the record once from those values.

3. **new_tests.rs**: Split 414-line file into three focused <300-line files:
   - `ticket_field_variation_tests.rs` (10 tests: 8 field variations + input bytes)
   - `taint_tests.rs` (6 tests: all Taint variants)
   - `emergency_and_admission_tests.rs` (3 tests: pinning + emergency overlay)
   All 18 new runtime behaviors preserved. Original 7 RuntimePolicy tests in `tests.rs` untouched.

4. **encode_taint_canonical**: Corrected documentation. Function is exhaustive over all 5 Taint variants and does not return `None` for any future variant. No `_ => 255` needed.

## Gate Evidence
All gates passed with unique prefixes:

- **Clippy**: `V5-CLIPPY` (exit 0, 2.63s)
- **Tests**: `V5-TESTS` (exit 0, 3.629s, 49 vb_core + 92 vb_runtime = 141 tests)
- **Fmt**: `V5-FMT-CHECK` (exit 0 after `V5-FMT-FIX`)
- **Post-fmt clippy**: `V5-CLIPPY-POST` (exit 0, 0.743s)
- **Post-fmt tests**: `V5-TESTS-POST` (exit 0, 1.73s)

Evidence stored at `.beads/vb-tphef/evidence/V5-*`

## Scope
This is foundation-only source; not parent security acceptance. The compute digest fix and evaluate consolidation are complete. The original 7 RuntimePolicy durability tests are preserved verbatim in `tests.rs`. No production code outside the policy foundation was changed.

## Files Changed
- `crates/vb_core/src/policy.rs` - corrected `encode_taint_canonical` documentation
- `crates/vb_core/src/policy/new_tests.rs` - fixed `test_digest` helper to use direct literal
- `crates/vb_runtime/src/policy.rs` - streaming hasher, consolidated evaluate
- `crates/vb_runtime/src/policy/new_tests.rs` - removed (split into 3 files)
- `crates/vb_runtime/src/policy/ticket_field_variation_tests.rs` - new (10 tests)
- `crates/vb_runtime/src/policy/taint_tests.rs` - new (6 tests)
- `crates/vb_runtime/src/policy/emergency_and_admission_tests.rs` - new (3 tests)
