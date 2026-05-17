# Test Plan Review: validate_taint SecretResultLeak Finish Pass-Through

## VERDICT: REJECTED

---

## Mode 1: Plan Inquisition
**Input**: `test-plan-validate-taint.md` (no implementation yet)
**Contract source**: Section 1 "Behavior Inventory" of the test plan itself

---

### Axis 1 — Contract Parity: PARTIAL FAIL

| Function | Contract Behaviors | BDD Scenarios | Status |
|----------|-------------------|---------------|--------|
| `validate_taint` (vb_validate) | 8 (behaviors 1-6, 11, 13, 14) | 8+ scenarios | ✓ Covered |
| `validate_workflow_ast` (vb_compile, pub(crate)) | 3 (behaviors 8-10) | 3 scenarios | ✓ Covered |
| `validate_public_result` (vb_compile, pub(crate)) | 1 (behavior 7) | Indirectly via compile pipeline | ⚠️ Internal |

**Findings**:
- `validate_public_result` and `validate_workflow_ast` are `pub(crate)` — internal implementation details, not public API. The contract concern is the public entry points (`validate_taint` in vb_validate, `compile` in vb_compile).
- **`ValidationError::UntrustedInput` does not exist** — behavior 11 specifies this exact variant, but it is absent from `vb_validate/src/lib.rs:97` (`ValidationError` enum). The test plan acknowledges this (line 140 note) but the contract cannot be satisfied until the enum variant is added. **LETHAL**.
- **`CompileError::UntrustedInput` does not exist** — behavior 12 specifies this variant, but it is absent from `vb_compile/src/lib.rs:1948` (`CompileError` enum). **LETHAL**.

---

### Axis 2 — Assertion Sharpness: PASS (with 1 LETHAL exception)

| Scenario | Then: clause | Assertion | Status |
|----------|-------------|-----------|--------|
| validate_taint accepts secret direct reference | `Ok(())` | Exact value ✓ | PASS |
| validate_taint accepts secret slot relay | `Ok(())` | Exact value ✓ | PASS |
| validate_taint accepts secret composite | `Ok(())` | Exact value ✓ | PASS |
| validate_taint accepts deep slot chain | `Ok(())` | Exact value ✓ | PASS |
| validate_taint accepts clean finish | `Ok(())` | Exact value ✓ | PASS |
| validate_taint rejects untrusted input | `Err(ValidationError::UntrustedInput)` | **Variant missing** | **LETHAL** |
| validate_taint rejects secret input in save | `Err(ValidationError::SecretResultLeak)` | Exact variant ✓ | PASS |
| compile accepts secret finish | `Ok(CompiledWorkflow)` | Exact value ✓ | PASS |
| compile rejects non-boolean choose | `Err(CompileErrors(...))` with `TypeMismatch` | Exact variant with field assertion ✓ | PASS |
| compile rejects uninitialized slot in finish | `Err(CompileErrors(...))` with `UnknownSlotType` | Exact slot index ✓ | PASS |

**Findings**:
- All concrete assertions use exact values or exact error variants with field discrimination.
- No `is_ok()` / `is_err()` as sole assertions.
- **Behavior 11**: `ValidationError::UntrustedInput` does not exist in the enum — the test cannot pass as written.

---

### Axis 3 — Trophy Allocation: FAIL

**Planned counts** (from Section 2):
- Unit: 18
- Integration: 12
- E2E: 2
- Static: 3
- **Total: 35**

**Public function count** (from grep of `vb_validate/src/type_taint.rs` and `vb_compile/src/type_taint.rs`):
- `pub fn validate_taint` — 1 public function
- `pub(crate) fn validate_workflow_ast` — internal
- `pub(crate) fn validate_public_result` — internal

**Ratio**: 18 unit tests / 1 public function = **18x** — exceeds 5x threshold ✓

**However**:
- **`validate_taint` is a pure function** with non-trivial input space (WorkflowTypes with various taint states). **No dedicated proptest invariant** directly on `validate_taint` for arbitrary valid WorkflowTypes — the proptest section (4) has `validate_taint_accepts_secret_finish_proptest` which covers the positive case, but no anti-invariant test that `validate_taint` correctly REJECTS secret in Save (non-Finish). The regression test (`validate_taint_rejects_secret_input_in_save_slot_for_regression`) documents this but is marked as a regression target. **MAJOR**.

**Fuzz targets**:
- 4 fuzz targets planned ✓ — adequate for parser/YAML handling

---

### Axis 4 — Boundary Completeness: MINOR ISSUES

| Function | Min | Max | Empty | Overflow | Status |
|----------|-----|-----|-------|----------|--------|
| validate_taint (Finish path) | Direct `$secrets.*` reference | Deep 5-hop slot chain | Empty composite ✓ | Slot chain depth N ✓ | PASS |
| validate_taint (non-Finish path) | Clean `$input.*` | Secret `$secrets.*` | Not specified | Slot overflow not explicitly named | **MINOR** |
| Taint::merge | Clean+Clean | Secret+Secret | Clean identity ✓ | 3-level lattice overflow not modeled | **MINOR** |
| compile pipeline | Clean YAML finish | Secret YAML finish | Empty finish not specified | Depth overflow in AST | **MINOR** |

**Findings**:
- `ValidationError::UntrustedInput` boundary (min untrusted value, max untrusted value) is entirely unspecified because the variant doesn't exist.
- Slot chain overflow (what happens when chain depth > MAX_SLOTS) is mentioned in Kani bound (≤1000 steps, ≤100 slots) but not in the test scenarios.

---

### Axis 5 — Mutation Survivability: MAJOR GAP

Mental mutations applied to `validate_taint`:

| Mutation | Catching test | Status |
|---------|--------------|--------|
| `SecretResultLeak` rejection → `Ok(())` in Finish arm | `validate_taint_accepts_secret_direct_reference_in_finish` | ✓ Caught |
| Remove taint merge in `resolve_composite` | `validate_taint_accepts_secret_composite_in_finish` | ✓ Caught |
| Remove secret check in `validate_public_result` | `compile_accepts_secret_finish_result` | ✓ Caught |
| Change `Taint::Secret` to `Taint::Clean` in `save_fact` | `validate_taint_rejects_secret_input_in_save_slot_for_regression` | ⚠️ Regression only — AFTER fix, no test catches this mutation in the Finish path |
| Remove slot read in `expression_fact` Slot arm | `compile_rejects_uninitialized_slot_in_finish` | ✓ Caught |
| Change Clean+Secret merge result to Clean | `taint_merge_propagates_secret` (proptest) | ✓ Caught |

**Critical gap**: The mutation "Change `Taint::Secret` to `Taint::Clean` in `save_fact`" is caught only by the regression test (`validate_taint_rejects_secret_input_in_save_slot_for_regression`). After the Section 47 fix is applied, if `save_fact` starts returning `Taint::Clean` instead of `Taint::Secret`, the Finish path would incorrectly accept secret data — and no positive test catches this.

**Proposed fix**: Add a proptest anti-invariant: `validate_taint` with secret-tainted Finish MUST return `Ok(())` (positive case), and a separate test must prove that `Taint::Secret` in any intermediate slot propagates correctly to the Finish result.

---

### Axis 6 — Evidence Plan Audit: MINOR ISSUES

**Given/When/Then structure**: All BDD scenarios have explicit Given/When/Then ✓

**Precondition explicitness**:
- Scenario 1 (validate_taint accepts secret direct reference): Preconditions stated ("secret `api_key` is declared in the workflow's secrets list") ✓
- Scenario 6 (untrusted data returns UntrustedInput): Precondition of "untrusted (non-secret, non-clean) data" is vague — untrusted is not defined as a type, and `ValidationError::UntrustedInput` doesn't exist. **MAJOR**.

**Bounded reproducible inputs**:
- Proptest strategies are named and bounded ✓
- Fuzz corpus seeds listed explicitly ✓
- Chain depth bounds (1..10 for proptest, 0..100 for fuzz) ✓

**Side effects**: Test helpers (`make_workflow`, `finish_step`, etc.) are not detailed in the plan — the plan references them as scaffold ("DO NOT IMPLEMENT — test-writer executes"). This is acceptable for a plan-phase document.

---

## LETHAL FINDINGS (any single = REJECTED)

1. **`ValidationError::UntrustedInput` does not exist** — test-plan-validate-taint.md:138 behavior 11 specifies this exact error variant, but it is absent from `vb_validate/src/lib.rs:97` (`ValidationError` enum). The test cannot pass. The enum must be updated BEFORE tests can be written.

2. **`CompileError::UntrustedInput` does not exist** — test-plan-validate-taint.md:138 behavior 12 specifies this exact error variant, but it is absent from `vb_compile/src/lib.rs:1948` (`CompileError` enum). The test cannot pass. The enum must be updated BEFORE tests can be written.

3. **Pure function `validate_taint` lacks a direct proptest invariant** — Section 4 plans `validate_taint_accepts_secret_finish_proptest` as the only proptest, which covers the positive case. However, there is no anti-invariant proving that `validate_taint` correctly REJECTS secret taint in non-Finish contexts. The regression test only documents the current (buggy) behavior, not the desired contract. After the fix, `validate_taint` needs a proptest that proves: (a) secret in Finish → `Ok(())`, AND (b) secret in Save → `Err(SecretResultLeak)`.

---

## MAJOR FINDINGS (≥3 = REJECTED)

1. **Anti-invariant missing for `validate_taint` secret-rejection path** — After the Section 47 fix, no test explicitly proves that `validate_taint` still rejects secret-tainted data in non-Finish steps. The regression test documents the OLD buggy behavior, not the NEW correct contract.

2. **`Taint::Secret → Taint::Clean` mutation in `save_fact` is not caught by the positive test suite** — If `save_fact` accidentally downgrades secret taint, the Finish-path tests would still pass (they only check that Finish accepts secret). A separate test must verify that intermediate slot taint is preserved.

3. **Untrusted data boundary is unspecified** — Behavior 11 says "untrusted data" but provides no definition of what constitutes untrusted data vs. secret vs. clean. The three-level taint lattice (Clean < DerivedFromSecret < Secret) from Section 47/contract is not reflected in the current `Taint` enum (which only has Clean and Secret). Open question 2 in Section 9 documents this but doesn't resolve it.

---

## MINOR FINDINGS (≥5 = REJECTED)

1. **Slot chain overflow not explicitly named** — Kani harness bounds mention ≤100 slots, but no scenario explicitly tests "100 slots + 1 (overflow)" for the Finish path.

2. **Empty composite boundary case** — `validate_taint_accepts_empty_composite_in_finish` is covered, but no "empty composite in non-Finish" scenario exists.

3. **Clean+Clean merge identity** — Proptest covers this (invariant 3), but the assertion is not explicit in the scenario descriptions.

4. **`DerivedFromSecret` taint level missing** — The master plan specifies a three-level lattice but the implementation only has two levels. Open question 2 acknowledges this but it's a gap in the contract itself.

5. **Unknown reference root boundary** — `validate_taint_unknown_reference_resolves_clean_in_finish` tests `$unknown_root.field`, but the boundary of "what makes a valid reference root" is not specified.

---

## MANDATE

Before resubmission, the following MUST be resolved:

1. **Add `ValidationError::UntrustedInput` to `vb_validate/src/lib.rs`** — or replace behavior 11 with `ValidationError::SecretResultLeak` if untrusted data is to be treated as a secret leak variant.

2. **Add `CompileError::UntrustedInput` to `vb_compile/src/lib.rs`** — or replace behavior 12 with the appropriate existing `CompileError` variant.

3. **Add explicit proptest anti-invariant for `validate_taint` secret rejection** — After the Section 47 fix, `validate_taint` must prove both: (a) secret in Finish → `Ok(())`, AND (b) secret in non-Finish → `Err(SecretResultLeak)`. The current regression test documents the past bug, not the future contract.

4. **Add mutation-catch test for `Taint::Secret` downgrade in `save_fact`** — A dedicated test that verifies intermediate slot taint is preserved through the Finish path.

5. **Resolve Open Questions 1-4** (Section 9) — Specifically question 1 (`UntrustedInput` existence) and question 2 (`DerivedFromSecret` taint level) before tests can be finalized.

Resubmit for full Mode 1 re-review after all LETHAL and MAJOR findings are resolved.
