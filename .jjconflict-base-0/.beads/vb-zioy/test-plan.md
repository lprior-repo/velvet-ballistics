# Test Plan: vb-zioy — Diagnostic Step Index Fix for emit_single_body_set

## Summary

- **Bead:** vb-zioy — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)
- **Scope:** Add `diagnostic_step: usize` to `emit_single_body_set` in `part_04.rs`; pass original source index from all 5 callers
- **Behaviors identified:** 12
- **Trophy allocation:** 6 unit / 4 integration / 0 e2e / 2 static (compile-time + code review)
- **Proptest invariants:** 4
- **Fuzz targets:** 0 (no parsing change — waiver per proof strategy)
- **Kani harnesses:** 0 (no arithmetic, unsafe, or temporal properties — waived per verifier-lane-decisions)
- **Mutation threshold target:** ≥90%

---

## 1. Behavior Inventory

1. `emit_single_body_set` reports `diagnostic_step` (not synthetic `id`) in `StepFieldShape` when body is empty.
2. `emit_single_body_set` reports `diagnostic_step` (not synthetic `id`) in `StepFieldShape` when body has multiple steps.
3. `emit_single_body_set` reports `diagnostic_step` (not synthetic `id`) in `UnsupportedStepPrimitive` when body contains a non-Set primitive.
4. `lower_canonical_for_each` passes its `index` as `diagnostic_step` to `emit_single_body_set`.
5. `lower_canonical_collect` passes its `index` as `diagnostic_step` to `emit_single_body_set`.
6. `lower_canonical_aggregate` passes its `index` as `diagnostic_step` to `emit_single_body_set`.
7. `lower_canonical_repeat` passes its `index` as `diagnostic_step` to `emit_single_body_set`.
8. `emit_together_branches` passes `branch_index` (source ordinal) as `diagnostic_step` to `emit_single_body_set`.
9. All 5 call sites compile with the updated signature (Rust type-system enforcement).
10. Existing integration test `compile_workflow_rejects_multi_step_body_in_scoped_primitives` asserts source step index for each scoped primitive.
11. Empty body in any scoped primitive reports source step index, not synthetic.
12. Non-Set primitive body in any scoped primitive reports source step index, not synthetic.

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit (Calc) | 6 | `emit_single_body_set` direct error construction; proptest property harnesses |
| Integration | 4 | Full workflow compilation through public API; caller-callee integration for each scoped primitive |
| E2E | 0 | No CLI/UI change; diagnostic string text unchanged |
| Static | 2 | `cargo check` enforces signature at all call sites; code review enforces caller intent |

**Deviation justification:** This is a pure diagnostic fidelity fix. No user-facing workflow changes. The behavior change is entirely in which `usize` value appears in an error variant. Integration tests dominate because the bug is in caller-callee parameter passing, not in pure logic.

---

## 3. BDD Scenarios

### Behavior: emit_single_body_set reports diagnostic_step in StepFieldShape for empty body

**Given:** `emit_single_body_set` is called with `body: &[]`, `diagnostic_step: 99`, and `id: StepIdx::new(0)`  
**When:** the function executes  
**Then:** it returns `Err(CompileErrors([StepFieldShape { step: 99, field: "steps", expected: "exactly one set step" }]))`  
**And:** `step` is **not** `0` (the synthetic `id`)

**Test name:** `fn emit_single_body_set_empty_body_reports_diagnostic_step_not_synthetic_id()`

### Behavior: emit_single_body_set reports diagnostic_step in StepFieldShape for multi-step body

**Given:** `emit_single_body_set` is called with `body` containing 2 Set steps, `diagnostic_step: 99`, and `id: StepIdx::new(0)`  
**When:** the function executes  
**Then:** it returns `Err(CompileErrors([StepFieldShape { step: 99, field: "steps", expected: "exactly one set step" }]))`  
**And:** `step` is **not** `0` (the synthetic `id`)

**Test name:** `fn emit_single_body_set_multi_step_body_reports_diagnostic_step_not_synthetic_id()`

### Behavior: emit_single_body_set reports diagnostic_step in UnsupportedStepPrimitive for non-Set body

**Given:** `emit_single_body_set` is called with `body` containing one `StepPrimitive::Wait`, `diagnostic_step: 99`, and `id: StepIdx::new(0)`  
**When:** the function executes  
**Then:** it returns `Err(CompileErrors([UnsupportedStepPrimitive { step: 99, primitive: "wait" }]))`  
**And:** `step` is **not** `0` (the synthetic `id`)

**Test name:** `fn emit_single_body_set_non_set_body_reports_diagnostic_step_not_synthetic_id()`

### Behavior: lower_canonical_for_each passes source index as diagnostic_step

**Given:** A workflow YAML with `for_each` as step 0 and a multi-step body  
**When:** `compile_workflow` is called  
**Then:** the first error is `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`  
**And:** `step` is **not** `1` (the synthetic body_step)

**Test name:** `fn for_each_multi_step_body_error_reports_source_step_zero()`

### Behavior: lower_canonical_collect passes source index as diagnostic_step

**Given:** A workflow YAML with `collect` as step 0 and a multi-step body  
**When:** `compile_workflow` is called  
**Then:** the first error is `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`  
**And:** `step` is **not** `1` (the synthetic body_step)

**Test name:** `fn collect_multi_step_body_error_reports_source_step_zero()`

### Behavior: lower_canonical_aggregate passes source index as diagnostic_step

**Given:** A workflow YAML with `reduce`/`aggregate` as step 0 and a multi-step body  
**When:** `compile_workflow` is called  
**Then:** the first error is `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`  
**And:** `step` is **not** `1` (the synthetic body_step)

**Test name:** `fn aggregate_multi_step_body_error_reports_source_step_zero()`

### Behavior: lower_canonical_repeat passes source index as diagnostic_step

**Given:** A workflow YAML with `repeat` as step 0 and a multi-step body  
**When:** `compile_workflow` is called  
**Then:** the first error is `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`  
**And:** `step` is **not** `1` (the synthetic body_step)

**Test name:** `fn repeat_multi_step_body_error_reports_source_step_zero()`

### Behavior: emit_together_branches passes branch_index as diagnostic_step

**Given:** A workflow YAML with `parallel` as step 0 and branch 0 having a multi-step body  
**When:** `compile_workflow` is called  
**Then:** the first error is `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`  
**And:** `step` reflects the branch source index, not the synthetic entry step

**Test name:** `fn parallel_branch_multi_step_body_error_reports_branch_source_step()`

### Behavior: empty body in scoped primitive reports source step index

**Given:** A workflow YAML with `collect` as step 0 and `steps: []`  
**When:** `compile_workflow` is called  
**Then:** the first error is `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`  

**Test name:** `fn collect_empty_body_error_reports_source_step_zero()`

### Behavior: non-Set primitive body in scoped primitive reports source step index

**Given:** A workflow YAML with `collect` as step 0 and body containing a single `wait` step  
**When:** `compile_workflow` is called  
**Then:** the first error is `UnsupportedStepPrimitive { step: 0, primitive: "wait" }`  

**Test name:** `fn collect_non_set_body_error_reports_source_step_zero()`

---

## 4. Proptest Invariants

### Proptest: emit_single_body_set StepFieldShape empty body

**Invariant:** For any `diagnostic_step: usize` and any synthetic `id: StepIdx` where `diagnostic_step != id.as_usize()`, empty body returns `StepFieldShape.step == diagnostic_step`.  
**Strategy:** `diagnostic_step` ∈ `0..1000`, `id` ∈ `StepIdx::new(0..1000)`.  
**Anti-invariant:** If `diagnostic_step == id.as_usize()`, the test cannot distinguish the fix — still valid but not informative.

**Test name:** `fn proptest_body_dispatcher_empty()`
**Artifact:** `crates/vb_compile/src/proptest_body_dispatcher.rs`
**Update required:** Pass `diagnostic_step` as a separate parameter (different from `id`); assert `step == diagnostic_step`.

### Proptest: emit_single_body_set StepFieldShape multi-step body

**Invariant:** For any `diagnostic_step: usize` and any synthetic `id: StepIdx` where `diagnostic_step != id.as_usize()`, multi-step body returns `StepFieldShape.step == diagnostic_step` and `expected.contains("one")`.  
**Strategy:** `diagnostic_step` ∈ `0..1000`, `id` ∈ `StepIdx::new(0..1000)`, body length ∈ `2..10`.  
**Anti-invariant:** Single-step body should succeed, not error.

**Test name:** `fn proptest_body_dispatcher_multi_step()`
**Artifact:** `crates/vb_compile/src/proptest_body_dispatcher.rs`
**Update required:** Pass `diagnostic_step` as a separate parameter; assert `step == diagnostic_step`.

### Proptest: emit_single_body_set UnsupportedStepPrimitive non-Set body

**Invariant:** For any non-Set `StepPrimitive` variant, any `diagnostic_step: usize`, and any synthetic `id: StepIdx` where `diagnostic_step != id.as_usize()`, the error returns `UnsupportedStepPrimitive.step == diagnostic_step`.  
**Strategy:** Generate all 9 non-Set variants (Do, ForEach, Together, Collect, Aggregate, Repeat, Wait, Ask, Finish).  
**Anti-invariant:** Set primitive body should succeed.

**Test name:** `fn proptest_error_parity()`
**Artifact:** `crates/vb_compile/src/proptest_error_parity.rs`
**Update required:** Pass `diagnostic_step` as a separate parameter; assert `step == diagnostic_step`.

### Proptest: emit_single_body_set empty body with arbitrary diagnostic_step

**Invariant:** Empty body always returns `StepFieldShape { step: diagnostic_step, field: "steps", .. }` regardless of `diagnostic_step` value.  
**Strategy:** `diagnostic_step` ∈ `0..1000`.  
**Anti-invariant:** Non-empty valid body should not return this error.

**Test name:** `fn proptest_error_parity_empty()`
**Artifact:** `crates/vb_compile/src/proptest_error_parity.rs`
**Update required:** Use `diagnostic_step: usize` separate from `id`; assert `step == diagnostic_step`.

---

## 5. Fuzz Targets

**None applicable.** Per proof strategy and verifier-lane-decisions (VLD-008, VLD-016, VLD-024, VLD-032), this bead makes no parsing, codec, or untrusted-input boundary changes. The fix is pure parameter plumbing. Fuzzing would not target the diagnostic step parameter specifically.

**Waiver reference:** `delivery-scope.jsonl:verification_scope=unit_tests_only`

---

## 6. Kani Verification Harnesses

**None applicable.** Per verifier-lane-decisions (VLD-003, VLD-011, VLD-019, VLD-027), Kani is not applicable because:
- No panic, overflow, or index risk is introduced.
- The function already returns `Result` on all error paths.
- `body.first()` is guarded by the `body.len() != 1` check.
- No `unsafe` code is involved.

**Waiver reference:** `verifier-lane-decisions.jsonl` — all Kani lanes marked `not_applicable` with `risk_absent`.

---

## 7. Mutation Testing Checkpoints

### Critical mutations to survive (cargo-mutants target: ≥90% kill rate)

| Mutation | Must be caught by | Rationale |
|----------|-------------------|-----------|
| Replace `diagnostic_step` with `id.as_usize()` in `StepFieldShape` constructor | `proptest_body_dispatcher_invariant_empty`, `for_each_multi_step_body_error_reports_source_step_zero`, `collect_multi_step_body_error_reports_source_step_zero` | This is the exact bug being fixed; if the mutation survives, the fix is ineffective |
| Replace `diagnostic_step` with `id.as_usize()` in `UnsupportedStepPrimitive` constructor | `proptest_error_parity`, `collect_non_set_body_error_reports_source_step_zero` | Same bug pattern in the non-Set match arm |
| Remove `diagnostic_step` parameter from `emit_single_body_set` signature | `cargo check` (compile-time) | Type system will reject all call sites |
| Change a caller to pass `body_step` instead of `index` | Integration tests for that specific primitive | Caller-specific regression test catches wrong index |
| Swap `diagnostic_step` and `id` argument positions at a call site | `cargo check` (type mismatch: `usize` vs `StepIdx`) | Type system catches this |
| Remove `body.len() != 1` check | `proptest_body_dispatcher_invariant_set` (would panic on empty body) or existing tests | Existing behavior preserved |

**Threshold:** 90% mutation kill rate minimum. The `diagnostic_step → id.as_usize()` substitution mutation MUST be killed by at least one test in every error path (empty, multi-step, non-Set).

---

## 8. Combinatorial Coverage Matrix

### emit_single_body_set direct unit tests

| Scenario | Input Class | Expected Output | Test Layer | Test Name |
|----------|-------------|-----------------|------------|-----------|
| empty body | `body: &[]`, `diagnostic_step: 99`, `id: 0` | `Err(StepFieldShape { step: 99, .. })` | unit | `emit_single_body_set_empty_body_reports_diagnostic_step_not_synthetic_id` |
| multi-step body | `body: &[Set, Set]`, `diagnostic_step: 99`, `id: 0` | `Err(StepFieldShape { step: 99, .. })` | unit | `emit_single_body_set_multi_step_body_reports_diagnostic_step_not_synthetic_id` |
| non-Set body | `body: &[Wait]`, `diagnostic_step: 99`, `id: 0` | `Err(UnsupportedStepPrimitive { step: 99, .. })` | unit | `emit_single_body_set_non_set_body_reports_diagnostic_step_not_synthetic_id` |
| valid Set body | `body: &[Set]`, `diagnostic_step: 99`, `id: 0` | `Ok(())`, 1 node emitted | unit | `emit_single_body_set_valid_set_body_succeeds` |

### Scoped primitive integration tests (caller verification)

| Scenario | Primitive | Source Step | Body | Expected Error Step | Test Layer | Test Name |
|----------|-----------|-------------|------|---------------------|------------|-----------|
| multi-step body | for_each | 0 | 2 Set steps | 0 (not 1) | integration | `for_each_multi_step_body_error_reports_source_step_zero` |
| multi-step body | collect | 0 | 2 Set steps | 0 (not 1) | integration | `collect_multi_step_body_error_reports_source_step_zero` |
| multi-step body | aggregate | 0 | 2 Set steps | 0 (not 1) | integration | `aggregate_multi_step_body_error_reports_source_step_zero` |
| multi-step body | repeat | 0 | 2 Set steps | 0 (not 1) | integration | `repeat_multi_step_body_error_reports_source_step_zero` |
| multi-step body | parallel branch | 0 | branch 0 has 2 steps | branch_index (not entry) | integration | `parallel_branch_multi_step_body_error_reports_branch_source_step` |
| empty body | for_each | 0 | `[]` | 0 (not 1) | integration | `for_each_empty_body_error_reports_source_step_zero` |
| empty body | collect | 0 | `[]` | 0 (not 1) | integration | `collect_empty_body_error_reports_source_step_zero` |
| empty body | aggregate | 0 | `[]` | 0 (not 1) | integration | `aggregate_empty_body_error_reports_source_step_zero` |
| empty body | repeat | 0 | `[]` | 0 (not 1) | integration | `repeat_empty_body_error_reports_source_step_zero` |
| non-Set body | for_each | 0 | 1 Wait step | 0 (not 1) | integration | `for_each_non_set_body_error_reports_source_step_zero` |
| non-Set body | collect | 0 | 1 Wait step | 0 (not 1) | integration | `collect_non_set_body_error_reports_source_step_zero` |
| non-Set body | aggregate | 0 | 1 Wait step | 0 (not 1) | integration | `aggregate_non_set_body_error_reports_source_step_zero` |
| non-Set body | repeat | 0 | 1 Wait step | 0 (not 1) | integration | `repeat_non_set_body_error_reports_source_step_zero` |

### Existing integration test update

| Test | Current Assertion | Required Update |
|------|-------------------|-----------------|
| `compile_workflow_rejects_multi_step_body_in_scoped_primitives` | Asserts `field == "steps"` and `expected == "exactly one set step"` (uses `..` on `step`) | Also assert `step == 0` for each case, confirming source index (not synthetic) |

### Proptest coverage matrix

| Property | Input Generation | Assertion | Artifact |
|----------|-----------------|-----------|----------|
| empty → StepFieldShape with diagnostic_step | `diagnostic_step` arbitrary, `id` arbitrary | `step == diagnostic_step` | `proptest_body_dispatcher.rs` |
| multi-step → StepFieldShape with diagnostic_step | `diagnostic_step` arbitrary, `id` arbitrary, body len 2+ | `step == diagnostic_step` | `proptest_body_dispatcher.rs` |
| non-Set → UnsupportedStepPrimitive with diagnostic_step | all 9 non-Set variants, `diagnostic_step` arbitrary | `step == diagnostic_step` | `proptest_error_parity.rs` |
| empty → StepFieldShape with diagnostic_step (parity) | `diagnostic_step` arbitrary | `step == diagnostic_step` | `proptest_error_parity.rs` |

---

## 9. Test Implementation Obligations

### File: `crates/vb_compile/tests/v1_primitive_lowering.rs`

1. **Update** `compile_workflow_rejects_multi_step_body_in_scoped_primitives`:
   - Change `match` arm from `CompileError::StepFieldShape { field, expected, .. }` to `CompileError::StepFieldShape { step, field, expected }`
   - Add assertion: `assert_eq!(step, 0, "case {case_name} expected source step 0, got {step}")`

2. **Add** `compile_workflow_rejects_empty_body_in_scoped_primitives`:
   - Parameterized over `[for_each, collect, aggregate, repeat]`
   - YAML with `steps: []` for each primitive
   - Assert `StepFieldShape { step: 0, field: "steps", expected: "exactly one set step" }`

3. **Add** `compile_workflow_rejects_non_set_body_in_scoped_primitives`:
   - Parameterized over `[for_each, collect, aggregate, repeat]`
   - YAML with body containing one `wait` step
   - Assert `UnsupportedStepPrimitive { step: 0, primitive: "wait" }`

4. **Add** `compile_workflow_parallel_branch_rejects_multi_step_body_reports_branch_index`:
   - YAML with `parallel` at step 0, branch 0 with 2 steps
   - Assert `StepFieldShape { step: 0, .. }` (branch_index = 0)

### File: `crates/vb_compile/src/proptest_body_dispatcher.rs`

1. **Update** all `emit_single_body_set` calls to pass `diagnostic_step` as a new parameter:
   - `emit_single_body_set(&body, id, diagnostic_step, slot, None, &mut builder, false)`
   - Where `diagnostic_step` is deliberately different from `id.as_usize()` (e.g., `99` vs `0`, or generated independently)

2. **Update** all assertions on `step` field to compare against `diagnostic_step`, not `id.as_usize()`:
   - `matches!(e, CompileError::StepFieldShape { step, field, .. } if *step == diagnostic_step && *field == "steps")`
   - `matches!(e, CompileError::UnsupportedStepPrimitive { step, .. } if *step == diagnostic_step)`

### File: `crates/vb_compile/src/proptest_error_parity.rs`

1. **Update** `emit_single_body_set` call signature to include `diagnostic_step`.
2. **Update** `proptest_error_parity_empty` to use `diagnostic_step: usize` separate from `id`.
3. **Update** all `step` assertions to reference `diagnostic_step`.

### Compile-time gate

**Command:** `cargo check --package vb_compile`
**Expected:** Zero errors. All 5 call sites updated to pass `diagnostic_step`.

### Grep verification

**Command:** `grep -n 'emit_single_body_set' crates/vb_compile/src/mod_compile_lowering/*.rs`
**Expected:** Exactly 5 call sites + 1 definition. Each call passes an original source index (not a synthetic computed step) as the `diagnostic_step` argument.

---

## 10. Open Questions

1. **pub(super) visibility:** `emit_single_body_set` is `pub(super)`. Direct unit tests must live inside `mod_compile_lowering` (e.g., in `part_04.rs` via `#[cfg(test)]` module, or in `tests.rs` within the same module). The test-writer must confirm the exact test module location.

2. **Parallel branch diagnostic step:** For `emit_together_branches`, the proof-to-implementation input suggests passing `branch_index` as `diagnostic_step`. However, a user might expect the `parallel` primitive's source step (the parent step index) rather than the branch index. This is a caller-side design decision that should be confirmed with the product owner. **Current plan assumes `branch_index` as documented in proof-to-implementation-input.md.**

3. **Proptest module linking:** The proptest files (`proptest_body_dispatcher.rs`, `proptest_error_parity.rs`) are currently not linked in `lib.rs` (per trusted-base-ledger TB-002). The test-writer must either:
   - Link them via `mod` declarations in `lib.rs` under `#[cfg(test)]`, or
   - Move the tests into an existing linked test module.
   **Current plan assumes re-linking or moving into existing test surface.**

4. **Test naming for aggregate vs reduce:** The YAML primitive is `reduce:` but the lowering function is `lower_canonical_aggregate`. Tests should use the user-visible primitive name (`reduce`) in test names and YAML, while the implementation reference uses `aggregate`.

---

## Traceability

| Requirement | Contract Clause | Test Coverage |
|-------------|-----------------|---------------|
| REQ-001 | `emit_single_body_set` reports source AST step in `StepFieldShape` | Unit tests (empty, multi-step) + proptest |
| REQ-002 | `emit_single_body_set` reports source AST step in `UnsupportedStepPrimitive` | Unit test (non-Set) + proptest |
| REQ-003 | Signature accepts `diagnostic_step` separate from compiled node id | `cargo check` + grep verification |
| REQ-004 | `lower_canonical_collect` passes original source index | Integration test (collect cases) |
| REQ-005 | All scoped primitive lowering functions report correct source step | Integration tests (for_each, aggregate, repeat, parallel) |


## Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|-------------------|------------------|-------------------|------------------------|----------|-----------------|------------|
