# Test Plan Review: test-plan-f64-contradiction.md

## VERDICT: REJECTED

## Mode

Mode 1 — Plan Inquisition. Reviewed `test-plan-f64-contradiction.md` against `velvet-ballistics-MASTER.md` Section 46 (no F64 eval mandate) and `vb-qi37.9.2/contract.md` (F64 bytecode execution semantics). No implementation or test code was edited.

---

## LETHAL FINDINGS

### LETHAL-1: Codegen F64 arithmetic behavior CONTRADICTS the contract

**Location**: `test-plan-f64-contradiction.md:107-115` (Codegen Add/Sub/Mul behavior)

**Contract Evidence** (`vb-qi37.9.2/contract.md:43-46`):
```
POST-001: eval_add_op on two SlotValue::F64 returns Ok(SlotValue::F64(finite))
POST-002: eval_sub_op on two SlotValue::F64 returns Ok(SlotValue::F64(finite))
POST-003: eval_mul_op on two SlotValue::F64 returns Ok(SlotValue::F64(finite))
```

**Contract Evidence** (`vb-qi37.9.2/contract.md:91-99` — F64 Arithmetic Policy Table):
```
| F64 + F64 | finite, finite | IEEE 754 sum | Err(NonFiniteFloat) if overflow to ±Inf |
| F64 - F64 | finite, finite | IEEE 754 diff | Err(NonFiniteFloat) if overflow to ±Inf |
| F64 * F64 | finite, finite | IEEE 754 prod | Err(NonFiniteFloat) if overflow to ±Inf |
```

**Test Plan Claims** (`test-plan-f64-contradiction.md:107-115`):
```
Then: The generated match arm for (SlotValue::F64(a), SlotValue::F64(b)) does NOT appear
And: Instead, the arm returns Err(ExprError::F64NotSupported { ... })
OR: The generated code explicitly truncates: SlotValue::I64(a as i64 + b as i64)
```

**Verdict**: The test plan requires codegen Add/Sub/Mul to reject F64 or lossy-cast to i64. The contract requires Add/Sub/Mul to return `Ok(SlotValue::F64(...))` for finite inputs. These are direct contradictions. The Div behavior is correctly handled (preserves F64, matches contract). But Add/Sub/Mil are LETHAL violations of contract parity.

**Required fix**: Either (a) the contract must be amended to forbid F64 in codegen Add/Sub/Mul (requires a new contract state with explicit waiver or revision), or (b) the test plan must be updated to assert `Ok(SlotValue::F64(finite))` for finite F64 inputs, with `Err(ExprError::NonFiniteFloat)` only for overflow cases.

---

### LETHAL-2: `Error::F64NotSupported` is an unverified error variant

**Location**: `test-plan-f64-contradiction.md:278` (Open Question 1)

**Finding**: The test plan uses `ExprError::F64NotSupported` throughout (lines 44, 52, 59, 66, 73, 81, 88, 101, 113, 141, 149, 158, 166) but the contract `vb-qi37.9.2/contract.md:61-68` defines the error taxonomy as:
```
ExprError::NonFiniteFloat — F64 arithmetic yields NaN or ±Inf
ExprError::DivisionByZero — I64 division by zero only
ExprError::TypeMismatch { expected, found } — F64 op received non-F64 type
```

`ExprError::F64NotSupported` does not appear in the contract error taxonomy. The contract uses `TypeMismatch` for wrong-type F64 operations and `NonFiniteFloat` for non-finite results. `F64NotSupported` is mentioned nowhere in the contract signatures or postconditions.

**Verdict**: Either `F64NotSupported` must be added to the contract error taxonomy with exact semantics (when is it returned vs TypeMismatch?), or all test assertions must use the contract-defined variants (`TypeMismatch` or `NonFiniteFloat`). Using an undefined error variant in all assertions without a contract amendment is LETHAL.

**Required fix**: Add `ExprError::F64NotSupported` to `vb-qi37.9.2/contract.md` error taxonomy with precise when/why semantics, OR replace all `F64NotSupported` references with contract-defined `TypeMismatch` or `NonFiniteFloat`.

---

## MAJOR FINDINGS (2)

### MAJOR-1: Open Questions indicate unresolved contract ambiguity

**Location**: `test-plan-f64-contradiction.md:276-285`

**Finding**: 5 open questions remain unresolved:
1. **OQ1**: Error variant name (F64NotSupported vs CodegenError::F64NotSupported) — unresolved, but used throughout all 13+ assertions
2. **OQ2**: Divergence between eval (uses FiniteF64::new) and codegen (raw `a / b`) — unresolved, affects Div behavior test
3. **OQ3**: NaN handling in codegen comparisons — unresolved, affects `test-plan-f64-contradiction.md:125-131`
4. **OQ4**: Constant folding vs typecheck boundary — unresolved, affects typecheck scenarios
5. **OQ5**: Lossy cast vs error for codegen Add/Sub/Mul — this IS the LETHAL-1 contradiction

**Verdict**: Writing tests for behaviors that have 5 unresolved open questions, including one that directly contradicts the contract, produces a test plan that cannot be executed correctly. Tests will be written to the wrong specification.

**Required fix**: Resolve all 5 open questions in the contract before writing tests. At minimum, OQ1 (error variant) and OQ5 (Add/Sub/Mul fate) must be resolved with contract amendments.

---

### MAJOR-2: NaN/Infinity boundary cases completely absent

**Location**: `test-plan-f64-contradiction.md` — entire plan

**Finding**: The contract `vb-qi37.9.2/contract.md:13` defines `FiniteF64` as "constructor rejects NaN and infinities" and `POST-001` through `POST-005` all require `Err(ExprError::NonFiniteFloat)` when results are NaN or infinite. The combinatorial coverage matrix (`test-plan-f64-contradiction.md:248-263`) has columns for F64 args, null args, I64 args, Symbol args, List args — but no column for `NaN`, `Inf`, `-Inf`.

The proptest invariants (`test-plan-f64-contradiction.md:173-186`) use `any::<FiniteF64>()` — this STRICTLY BOUNDS inputs to finite values. `FiniteF64` by construction cannot be NaN or infinite. The anti-invariant "any helper receiving SlotValue::F64 must NOT return Ok(...)" is satisfied by construction because `FiniteF64` can never be NaN/Inf — so the proptest cannot test the NonFiniteFloat error path.

**Verdict**: Zero test scenarios cover NaN, positive infinity, or negative infinity inputs. The proptest uses `FiniteF64` which is statically incapable of producing non-finite values. The codegen Div scenario at line 119-123 says "result is wrapped in FiniteF64::new check" but no test actually provides Inf as input to verify the error path.

**Required fix**: Add explicit NaN, +Inf, -Inf boundary scenarios for both eval and codegen paths. The proptest invariant should use `any::<f64>()` (not `FiniteF64`) mapped through `FiniteF64::new` to generate both finite and non-finite inputs, or add separate proptest cases explicitly covering the `Err(NonFiniteFloat)` path.

---

## MINOR FINDINGS (3/5 threshold)

### MINOR-1: `Empty` helper `Ok(true)` for null is in matrix but not in scenarios

**Location**: `test-plan-f64-contradiction.md:251` (combinatorial matrix), `test-plan-f64-contradiction.md:48-53` (scenarios)

**Finding**: The combinatorial matrix shows `Empty` with null arg returns `Ok(true)`, but no BDD scenario explicitly exercises `Empty` with null. The first `Empty` scenario (`test-plan-f64-contradiction.md:48-53`) tests F64 only. The arity mismatch scenario (`test-plan-f64-contradiction.md:84-90`) tests Length with two F64 args but does not test null.

**Required fix**: Add explicit BDD scenario for `Empty` with null/None input asserting `Ok(true)`.

---

### MINOR-2: Kani harness bound calculation is unclear

**Location**: `test-plan-f64-contradiction.md:215-221`

**Finding**: Harness `eval_helper_all_helpers_f64_rejection` claims "9 helpers × 3 f64 edge values (0.0, MIN_POSITIVE, MAX) = 27 paths". But `MIN_POSITIVE` is a sub-normal, not a normal floating point value, and the proptest uses `any::<FiniteF64>()` which already excludes non-finite values. The "3 f64 edge values" are not enumerated as named constants. The harness also does not cover the `NaN`/`Inf` cases needed per MAJOR-2.

**Required fix**: Enumerate exact f64 edge values used (e.g., `0.0`, `f64::MIN_POSITIVE`, `f64::MAX`, `f64::NAN`, `f64::INFINITY`, `f64::NEG_INFINITY`) and justify the bound of 27 paths.

---

### MINOR-3: Mutation checkpoint for `eval_helper_with_store` lacks specific test name

**Location**: `test-plan-f64-contradiction.md:238`

**Finding**: The mutation checkpoint table has a row:
```
| eval_helper_with_store removes F64 check | eval_helper_with_store_rejects_f64 |
```

But no BDD scenario named `eval_helper_with_store_rejects_f64` appears in Section 3. The general "store-aware helpers mirror F64 rejection" scenario at `test-plan-f64-contradiction.md:94-102` is too generic — it only shows `Length` as an example and says "All store-aware helpers... must mirror the F64 rejection behavior". This does not constitute a named test.

**Required fix**: Add explicit BDD scenario with a named test for `eval_helper_with_store` F64 rejection, or reference a specific test name that will be in the test suite.

---

## MANDATE

The following must exist before resubmission:

1. **Contract amendment or conflict resolution**: The Add/Sub/Mul codegen behavior contradiction (LETHAL-1) must be resolved. Either amend `vb-qi37.9.2/contract.md` to forbid F64 in Add/Sub/Mul codegen, OR update the test plan to assert `Ok(SlotValue::F64(finite))` for finite inputs.

2. **Error variant definition**: `ExprError::F64NotSupported` must be added to the contract error taxonomy with exact semantics (LETHAL-2), OR all test assertions must use contract-defined variants.

3. **NaN/Inf test scenarios**: Explicit BDD scenarios for NaN, +Inf, -Inf inputs asserting `Err(ExprError::NonFiniteFloat)` for both eval and codegen paths.

4. **All 5 Open Questions resolved**: OQ1-OQ5 in `test-plan-f64-contradiction.md:276-285` must be answered with contract amendments or implementation decisions before tests can be written correctly.

5. **Named tests for all mutation checkpoints**: Every row in the mutation checkpoint table (`test-plan-f64-contradiction.md:229-240`) must have a corresponding named BDD scenario in Section 3.

---

## Summary

| Axis | Verdict | Finding |
|------|---------|---------|
| Contract Parity | **LETHAL FAIL** | Codegen Add/Sub/Mul test contradicts contract POST-001/002/003 |
| Assertion Sharpness | **MAJOR FAIL** | Error variant `F64NotSupported` not in contract error taxonomy |
| Trophy Allocation | PASS (ratio) | 6+4+1+2 = 13 tests planned; ratio appears adequate |
| Boundary Completeness | **MAJOR FAIL** | NaN/Inf not covered; proptest uses FiniteF64 (can't produce non-finite) |
| Mutation Survivability | PASS | Mutation checkpoints are named and comprehensive |
| Evidence Plan Audit | **MINOR FAIL** | Open questions unresolved; named test gaps |

**STATUS: REJECTED** — Resubmit after resolving both LETHAL findings and both MAJOR findings.
