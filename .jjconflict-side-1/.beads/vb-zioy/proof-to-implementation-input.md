# Proof-to-Implementation Input: vb-zioy

## Bridge Document

This artifact maps approved proof claims to Rust source/test/harness obligations for the proof-to-implementation bridge (State 5).

## Source File Changes

### 1. `crates/vb_compile/src/mod_compile_lowering/part_04.rs`

**Claim**: `emit_single_body_set` signature accepts `diagnostic_step: usize` and uses it in all error constructors.

**Rust source refs**:
- Line 211: Update function signature to add `diagnostic_step: usize` parameter
- Lines 219-224: Change `step: id.as_usize()` → `step: diagnostic_step` in `body.len() != 1` error
- Lines 226-232: Change `step: id.as_usize()` → `step: diagnostic_step` in `body.first()` error (redundant path, but must be consistent)
- Lines 241-246: Change `step: id.as_usize()` → `step: diagnostic_step` in `UnsupportedStepPrimitive` error

**Behavior test refs**:
- PO-001: `proptest_body_dispatcher.rs` must be updated to pass `diagnostic_step` and assert it appears in `StepFieldShape.step`
- PO-002: `proptest_error_parity.rs` must be updated to pass `diagnostic_step` and assert it appears in `UnsupportedStepPrimitive.step`

### 2. `crates/vb_compile/src/mod_compile_lowering/part_03.rs`

**Claim**: `lower_canonical_collect` passes `index` as `diagnostic_step`.

**Rust source refs**:
- Line 193: Update `emit_single_body_set` call to pass `index` as `diagnostic_step`

**Behavior test refs**:
- PO-003: `v1_primitive_lowering.rs` test `compile_workflow_rejects_multi_step_body_in_scoped_primitives` must assert `step == 0` for collect case

**Claim**: `emit_together_branches` passes appropriate source index as `diagnostic_step`.

**Rust source refs**:
- Line 135: Update `emit_single_body_set` call in `emit_together_branches` to pass `branch_index` (or appropriate source index) as `diagnostic_step`

### 3. `crates/vb_compile/src/mod_compile_lowering/part_02.rs`

**Claim**: `lower_canonical_for_each` passes `index` as `diagnostic_step`.

**Rust source refs**:
- Line 225: Update `emit_single_body_set` call to pass `index` as `diagnostic_step`

### 4. `crates/vb_compile/src/mod_compile_lowering/part_04.rs` (continued)

**Claim**: `lower_canonical_aggregate` passes `index` as `diagnostic_step`.

**Rust source refs**:
- Line 52: Update `emit_single_body_set` call to pass `index` as `diagnostic_step`

**Claim**: `lower_canonical_repeat` passes `index` as `diagnostic_step`.

**Rust source refs**:
- Line 118: Update `emit_single_body_set` call to pass `index` as `diagnostic_step`

## Test Obligations

### Unit/Integration Tests (`crates/vb_compile/tests/v1_primitive_lowering.rs`)

**Test: `compile_workflow_rejects_multi_step_body_in_scoped_primitives`**
- **Current**: Asserts `field == "steps"` and `expected == "exactly one set step"`
- **Required update**: Also assert `step == 0` (the source step index) for each scoped primitive case, confirming it is NOT the synthetic step (which would be 1 for collect/for_each/aggregate/repeat)
- **Rationale**: This is the primary regression test that will catch a caller passing the wrong index

**New Test: `compile_workflow_rejects_non_set_body_in_scoped_primitives`** (optional)
- **Purpose**: Cover the `UnsupportedStepPrimitive` arm for each scoped primitive
- **Input**: YAML with a scoped primitive whose body contains a single non-Set step (e.g., a `wait` step)
- **Assertion**: Error is `UnsupportedStepPrimitive` with `step == 0` (source), not `1` (synthetic)

**New Test: `compile_workflow_rejects_empty_body_in_scoped_primitives`** (optional)
- **Purpose**: Cover the empty body path explicitly for each scoped primitive
- **Input**: YAML with a scoped primitive with `steps: []`
- **Assertion**: Error is `StepFieldShape` with `step == 0` (source), not `1` (synthetic)

### Property Tests (`crates/vb_compile/src/proptest_body_dispatcher.rs`)

**Update: `proptest_body_dispatcher_empty`**
- **Current**: `emit_single_body_set(&empty_body, id, slot, ...)` — no diagnostic_step
- **Required**: `emit_single_body_set(&empty_body, id, diagnostic_step, slot, ...)` where `diagnostic_step != id.as_usize()`
- **Assertion**: `StepFieldShape.step == diagnostic_step` (not `id.as_usize()`)

**Update: `proptest_body_dispatcher_multi_step`**
- Same pattern: pass `diagnostic_step` and assert it appears in error

**Update: `proptest_body_dispatcher_invariant_empty`**
- **Current**: `id = StepIdx::new(42)` and asserts `step == 42`
- **Required**: Pass `diagnostic_step = 99` (different from id), assert `step == 99`

### Property Tests (`crates/vb_compile/src/proptest_error_parity.rs`)

**Update: `proptest_error_parity`**
- **Current**: `emit_single_body_set(&body, id, slot, ...)` — no diagnostic_step
- **Required**: `emit_single_body_set(&body, id, diagnostic_step, slot, ...)` where `diagnostic_step != id.as_usize()`
- **Assertion**: `UnsupportedStepPrimitive.step == diagnostic_step`

**Update: `proptest_error_parity_empty`**
- **Current**: Uses `step_idx: u16` for id and asserts `step == step_idx as usize`
- **Required**: Use `diagnostic_step: usize` separate from `id`, assert `step == diagnostic_step`

## Compile-Time Enforcement

**Command**: `cargo check --package vb_compile`
- **Expected**: Zero errors. If any caller was not updated, the type system will reject it.
- **Evidence**: stdout showing `Finished dev [unoptimized] target(s) in Ns`

## Grep Verification

**Command**: `grep -n 'emit_single_body_set' crates/vb_compile/src/mod_compile_lowering/*.rs`
- **Expected**: Exactly 5 call sites, all showing the new `diagnostic_step` parameter.
- **Evidence**: grep output listing lines in part_02.rs, part_03.rs, part_04.rs

## Refinement Harness References

No refinement harnesses are required. This bead does not introduce new types, typestates, or model abstractions. The contract is enforced through:
1. Compile-time signature enforcement (Rust type system)
2. Runtime test assertions (unit, integration, proptest)
3. Human code review (caller verification)

## Mapping Status

All claims are in `planned` status. They must transition to `materialized` after implementation and `verified` after test execution.
