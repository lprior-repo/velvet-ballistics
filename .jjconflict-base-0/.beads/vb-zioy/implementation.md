# vb-zioy Implementation Report

## Bead
**vb-zioy** — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)

## Summary
Fixed `emit_single_body_set` in `crates/vb_compile/src/mod_compile_lowering/part_04.rs` to report error step indices using the original source step index (`diagnostic_step`) instead of the synthetic compiled `StepIdx` (`id`). This ensures that when collect/for_each/repeat/aggregate/parallel have invalid bodies, the diagnostic points to the correct source step.

## Changes Made

### 1. `emit_single_body_set` signature change
**File:** `crates/vb_compile/src/mod_compile_lowering/part_04.rs`

Added `diagnostic_step: usize` parameter to the function signature after `id: StepIdx`.

### 2. Error reporting updated
All three error sites in `emit_single_body_set` now use `diagnostic_step` instead of `id.as_usize()`:
- `body.len() != 1` → `StepFieldShape` error
- `body.first()` None → `StepFieldShape` error  
- Non-Set primitive → `UnsupportedStepPrimitive` error
- `body_constant_index` call → passes `diagnostic_step` for `parse_i64_field` error reporting

### 3. Caller updates

| Function | File | Change |
|----------|------|--------|
| `lower_canonical_for_each` | `part_02.rs` | Pass `index` as `diagnostic_step` |
| `lower_canonical_collect` | `part_03.rs` | Pass `index` as `diagnostic_step` |
| `lower_canonical_aggregate` | `part_04.rs` | Pass `index` as `diagnostic_step` |
| `lower_canonical_repeat` | `part_04.rs` | Pass `index` as `diagnostic_step` |
| `emit_together_branches` | `part_03.rs` | Added `diagnostic_step: usize` param, passed from `lower_canonical_parallel` |

### 4. Test file updates
Updated direct `emit_single_body_set` callers in non-compiled proptest files to pass `id.as_usize()` as `diagnostic_step` to preserve existing test semantics:
- `proptest_error_parity.rs`
- `proptest_body_dispatcher.rs`
- `proptest_collect.rs`

## Verification

### Compilation
```bash
cargo check -p vb_compile --all-targets
```
**Result:** PASS

### Tests
```bash
cargo test -p vb_compile
```
**Result:** PASS for all tests related to modified code (for_each, collect, repeat, aggregate, body lowering). 9 pre-existing failures in `lower_canonical_choose_*` tests are unrelated to this change.

```bash
cargo test -p vb_compile -- for_each collect repeat aggregate body
```
**Result:** PASS — 20 passed, 0 failed (filtered)

### Power-of-Ten Compliance
- **Zero forbidden constructs:** No unsafe, unwrap, expect, panic, todo, unimplemented, dbg added.
- **Simple control flow:** No new loops or recursion; only parameter passing changes.
- **Checked returns:** All existing Result propagation preserved.
- **Warnings:** No new warnings introduced.

## Performance Impact
None. This is a diagnostic-only change; no runtime behavior or hot path modified.

## Residual Risks
- The 9 pre-existing `lower_canonical_choose_*` test failures are unrelated to this bead and existed before changes.
- Proptest verification files are not currently included in the module tree (`property_tests` module disabled in `lib.rs`), so their compilation is not verified by CI.

## State
Completed: State 11 (holzman-rust implementation)


## Source Coverage Matrix

| Source File | Function | Line Range | Test Coverage | Status |
|------------|----------|-----------|---------------|--------|
