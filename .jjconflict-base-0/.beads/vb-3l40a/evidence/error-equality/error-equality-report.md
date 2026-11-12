# Error Equality Fix Report — vb-3l40a

## Date
2026-10-21

## Agent
Local5090 bounded implementation repair agent

## Workspace
`/home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-runtime-5090`

## Root Cause
The `RuntimeError::eq` implementation in `crates/vb_runtime/src/error/equality.rs` was missing explicit fieldwise equality cases for three runtime error variants:

- `RuntimeError::Recovery { error: String }` — durable recovery failures
- `RuntimeError::ContractValidationError { action, error }` — admission contract validation failures
- `RuntimeError::RuntimeInputSlotKindMismatch { slot, expected, actual }` — runtime input slot kind validation failures

These variants fell through the exhaustive match to `_ => false`, causing structurally identical instances to compare unequal. This violated the mathematical contract that `RuntimeError`'s `PartialEq`/`Eq` is a true identity relation on its structural fields.

## Fix
Added fieldwise equality cases in `runtime_error_core_field_eq`:

```rust
(
    RuntimeError::Recovery { error: a },
    RuntimeError::Recovery { error: b },
) => a == b,
(
    RuntimeError::RuntimeInputSlotKindMismatch {
        slot: a_slot, expected: a_exp, actual: a_act,
    },
    RuntimeError::RuntimeInputSlotKindMismatch {
        slot: b_slot, expected: b_exp, actual: b_act,
    },
) => a_slot == b_slot && a_exp == b_exp && a_act == b_act,
(
    RuntimeError::ContractValidationError {
        action: a_act, error: a_err,
    },
    RuntimeError::ContractValidationError {
        action: b_act, error: b_err,
    },
) => a_act == b_act && a_err == b_err,
```

All existing equality cases preserved. No blanket Debug-string comparisons added.

## Tests Added
New test module: `crates/vb_runtime/src/error/tests_equality.rs` (16 tests)

Coverage matrix:
- **Recovery**: reflexivity, identical strings equal, different strings unequal
- **ContractValidationError**: reflexivity, identical equal, different action unequal, different message unequal, different both unequal
- **RuntimeInputSlotKindMismatch**: reflexivity, identical equal, different slot unequal, different expected unequal, different actual unequal
- **Cross-variant**: Recovery vs ContractValidationError, ContractValidationError vs RuntimeInputSlotKindMismatch, RuntimeInputSlotKindMismatch vs Recovery

## Verification Results

### New Equality Tests
```
running 16 tests
test error::tests_equality::contract_validation_different_both_unequal ... ok
test error::tests_equality::contract_validation_different_action_unequal ... ok
test error::tests_equality::contract_validation_different_message_unequal ... ok
test error::tests_equality::contract_validation_reflexivity ... ok
test error::tests_equality::contract_validation_identical_equal ... ok
test error::tests_equality::cross_variant_slot_mismatch_vs_recovery ... ok
test error::tests_equality::cross_variant_contract_vs_slot_mismatch ... ok
test error::tests_equality::recovery_different_strings_unequal ... ok
test error::tests_equality::cross_variant_recovery_vs_contract ... ok
test error::tests_equality::recovery_identical_strings_equal ... ok
test error::tests_equality::recovery_reflexivity ... ok
test error::tests_equality::slot_kind_mismatch_different_actual_unequal ... ok
test error::tests_equality::slot_kind_mismatch_different_expected_unequal ... ok
test error::tests_equality::slot_kind_mismatch_different_slot_unequal ... ok
test error::tests_equality::slot_kind_mismatch_identical_equal ... ok
test error::tests_equality::slot_kind_mismatch_reflexivity ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 1878 filtered out
```

### Full vb_runtime Library Test Suite
```
running 1894 tests
test result: ok. 1894 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Recovery Subsystem Tests
```
running 32 tests
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 1862 filtered out
```

### All Equality Tests
```
running 104 tests
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 1790 filtered out
```

## Constraints Satisfied
- ✅ Fieldwise equality (no Debug string comparison)
- ✅ All existing equality cases preserved
- ✅ Different messages/actions/slot/kinds compare unequal
- ✅ No alteration of mathematical/test contract to green tests
- ✅ Only edited `equality.rs` and added registered test module
- ✅ No unwrap/expect/panic/unchecked indexing/allowlist changes
- ✅ No edits to lifecycle tests or behavior
- ✅ File size ≤ 300 lines (no exception added)
- ✅ Real exec_command writes (not simulated XML)

## Files Modified
1. `crates/vb_runtime/src/error/equality.rs` — added 3 fieldwise equality cases
2. `crates/vb_runtime/src/error/tests_equality.rs` — new test module (16 tests)
3. `crates/vb_runtime/src/error/mod.rs` — registered new test module

## Files NOT Modified
- All lifecycle tests (unchanged)
- All behavior code (unchanged)
- All other error modules (unchanged)

## Status
PASS — Implementation repaired and verified against full test suite.
