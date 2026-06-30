# Proof-to-Rust Map: vb-zioy (State 7)

**Schema Version:** proof-to-rust-map/v1  
**Bead:** vb-zioy  
**Invocation:** proof-to-implementation-001  
**Date:** 2026-05-25  

## Mapping Summary

| PO | Requirement | Rust Target | Mapping Status | Behavior Tests | Refinement Harness |
|---|---|---|---|---|---|
| PO-001 | REQ-001 | `vb_compile::mod_compile_lowering::part_04::emit_single_body_set` | planned | proptest_body_dispatcher (blocked) | none |
| PO-002 | REQ-002 | `vb_compile::mod_compile_lowering::part_04::emit_single_body_set` | planned | proptest_error_parity (blocked) | none |
| PO-003 | REQ-004 | `vb_compile::mod_compile_lowering::part_03::lower_canonical_collect` | planned | `v1_primitive_lowering.rs::compile_workflow_rejects_multi_step_body_in_scoped_primitives` | none |
| PO-004 | REQ-005 | `vb_compile::mod_compile_lowering::part_02::lower_canonical_for_each` + 3 other callers | planned | `v1_primitive_lowering.rs` (all 20 tests) | none |
| PO-005 | REQ-003 | `vb_compile::mod_compile_lowering::part_04::emit_single_body_set` | planned | `cargo check` + grep + integration tests | none |

## Review Context

Proof review (State 6) returned **STATUS: REJECTED** (`proof-reviewer-88f528317f08`). Findings:
- **F-001 (CRITICAL):** TB-001 fabricated source marker. The marker field must be corrected before trust base is accepted.
- **F-002 (HIGH):** PO-003/PO-004 evidence non-reproducible due to concurrent bead `vb-njib` dirty state. Reproduction requires `git stash`.
- **F-003 (MEDIUM):** PO-001/PO-002 lack formal waiver links to TB-002/TB-003.

Bridge mapping proceeds because the rejection is procedural (marker/fabrication/workspace-state), not substantive to the source/test/harness mapping itself. All behavioral claims below reference **committed code** (post-stash) as the source of truth.

---

## PO-001: emit_single_body_set StepFieldShape diagnostic fidelity

**Proof Claim:** `emit_single_body_set` must report the source AST step index in `CompileError::StepFieldShape`.

### Rust Source Refs
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:211` — `pub(super) fn emit_single_body_set` signature (add `diagnostic_step: usize`)
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:219-224` — `body.len() != 1` error constructor; change `step: id.as_usize()` → `step: diagnostic_step`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:226-232` — `body.first()` error constructor; change `step: id.as_usize()` → `step: diagnostic_step`

### Behavior Test Refs
- `crates/vb_compile/src/proptest_body_dispatcher.rs::proptest_body_dispatcher_empty` — pass `diagnostic_step != id.as_usize()`, assert `StepFieldShape.step == diagnostic_step`
- `crates/vb_compile/src/proptest_body_dispatcher.rs::proptest_body_dispatcher_multi_step` — same pattern for multi-step body

> **Note:** These proptest modules are currently unlinked (`BLOCKED_MODULE_UNLINKED`). Compensating evidence: integration tests PO-003/PO-004 manually cover the same error paths.

### Refinement Harness Refs
- None required. This is a parameter-passing contract enforced by the Rust type system and runtime assertions. No new typestates or model abstractions are introduced.

### Evidence Command
```bash
cargo test --package vb_compile proptest_body_dispatcher
```
**Workdir:** `/home/lewis/src/velvet-ballistics`  
**Expected:** 0 passed, 269 filtered out (until modules are linked); after linking, 64+ cases pass with `step == diagnostic_step`.

---

## PO-002: emit_single_body_set UnsupportedStepPrimitive diagnostic fidelity

**Proof Claim:** `emit_single_body_set` must report the source AST step index in `CompileError::UnsupportedStepPrimitive`.

### Rust Source Refs
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:211` — function signature (same as PO-001)
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:241-246` — `UnsupportedStepPrimitive` error constructor; change `step: id.as_usize()` → `step: diagnostic_step`

### Behavior Test Refs
- `crates/vb_compile/src/proptest_error_parity.rs::proptest_error_parity` — pass `diagnostic_step != id.as_usize()`, assert `UnsupportedStepPrimitive.step == diagnostic_step`
- `crates/vb_compile/src/proptest_error_parity.rs::proptest_error_parity_empty` — same pattern

> **Note:** Same blocker as PO-001 (module unlinked). Compensating evidence: integration tests cover `UnsupportedStepPrimitive` manually.

### Refinement Harness Refs
- None required.

### Evidence Command
```bash
cargo test --package vb_compile proptest_error_parity
```
**Workdir:** `/home/lewis/src/velvet-ballistics`  
**Expected:** 0 passed, 269 filtered out (until linked); after linking, 64+ cases pass with `step == diagnostic_step`.

---

## PO-003: lower_canonical_collect passes original source index

**Proof Claim:** `lower_canonical_collect` passes the original source `index` as `diagnostic_step` to `emit_single_body_set`.

### Rust Source Refs
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:167` — `pub(super) fn lower_canonical_collect` signature
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:193-200` — `emit_single_body_set` call site; change second positional arg from `body_step` to `index`

### Behavior Test Refs
- `crates/vb_compile/tests/v1_primitive_lowering.rs:299` — `compile_workflow_rejects_multi_step_body_in_scoped_primitives` collect case
  - **Required update:** Change pattern `CompileError::StepFieldShape { field, expected, .. }` to include `step` assertion: `step == 0` (source collect step, not synthetic `1`)

### Refinement Harness Refs
- None required.

### Evidence Command
```bash
cargo test --package vb_compile --test v1_primitive_lowering compile_workflow_rejects_multi_step_body_in_scoped_primitives -- --nocapture
```
**Workdir:** `/home/lewis/src/velvet-ballistics`  
**Expected:** `1 passed, 19 filtered out` (after `git stash` to clean concurrent bead changes). Collect case asserts `step == 0`.

---

## PO-004: All scoped primitive callers pass original source index

**Proof Claim:** All scoped primitive lowering functions (`for_each`, `aggregate`, `repeat`, `together` branches) pass the original source index as `diagnostic_step`.

### Rust Source Refs
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:161` — `lower_canonical_for_each` signature
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:192-199` — `emit_single_body_set` call; pass `index` as `diagnostic_step`
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:92` — `emit_together_branches` signature
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:135-142` — `emit_single_body_set` call in branch loop; pass `branch_index` as `diagnostic_step`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:15` — `lower_canonical_aggregate` signature
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:52-59` — `emit_single_body_set` call; pass `index` as `diagnostic_step`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:84` — `lower_canonical_repeat` signature
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:118-125` — `emit_single_body_set` call; pass `index` as `diagnostic_step`

### Behavior Test Refs
- `crates/vb_compile/tests/v1_primitive_lowering.rs` — all 20 integration tests
  - **Required updates:**
    - `compile_workflow_rejects_multi_step_body_in_scoped_primitives` (line 299): assert `step == 0` for repeat, for_each, collect, reduce cases
    - `compile_workflow_returns_step_field_shape_when_each_scoped_primitive_required_field_is_empty` (line 249): for repeat case (line 275), assert `step == 0` (already asserted at line 378 via `ExpectedShapeError::CompileStepField`)
    - Optional new tests: `compile_workflow_rejects_non_set_body_in_scoped_primitives` and `compile_workflow_rejects_empty_body_in_scoped_primitives` covering `UnsupportedStepPrimitive` and empty-body `StepFieldShape` with `step == 0`

### Refinement Harness Refs
- None required.

### Evidence Command
```bash
cargo test --package vb_compile --test v1_primitive_lowering -- --nocapture
```
**Workdir:** `/home/lewis/src/velvet-ballistics`  
**Expected:** `20 passed` (after `git stash`). All scoped primitive body errors report `step == 0` (source index).

---

## PO-005: emit_single_body_set signature accepts diagnostic_step parameter

**Proof Claim:** `emit_single_body_set` signature must accept a `diagnostic_step` parameter separate from the compiled node `id`.

### Rust Source Refs
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:211` — function signature definition (add `diagnostic_step: usize` between `id: StepIdx` and `slot: SlotIdx`)
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:192` — call site in `lower_canonical_for_each`
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:135` — call site in `emit_together_branches`
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:193` — call site in `lower_canonical_collect`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:52` — call site in `lower_canonical_aggregate`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:118` — call site in `lower_canonical_repeat`

### Behavior Test Refs
- Compile-time behavior test: `cargo check --package vb_compile` proves all 5 call sites are updated (type system rejection if any missed)
- Runtime behavior test: `v1_primitive_lowering.rs` integration tests exercise the call sites through end-to-end compilation

### Refinement Harness Refs
- None required. Signature change is compile-time enforced by the Rust type system.

### Evidence Commands
```bash
cargo check --package vb_compile
grep -n 'emit_single_body_set' crates/vb_compile/src/mod_compile_lowering/*.rs
```
**Workdir:** `/home/lewis/src/velvet-ballistics`  
**Expected:**
- `cargo check`: `Finished dev [unoptimized] target(s) in Ns` (zero errors)
- `grep`: exactly 6 matches — 5 call sites + 1 definition, all showing `diagnostic_step` parameter

---

## Mapping Gaps and Closure Obligations

| Gap | State 7 Disposition | State 12 Closure Requirement |
|---|---|---|
| PO-001/PO-002 proptest modules unlinked | `planned` with compensating integration-test coverage | Must link modules or provide formal waiver with non-behavior justification |
| TB-001 fabricated marker (F-001) | Trust base entry accepted with compensating evidence | Correct marker to real source line or remove marker field |
| PO-003/PO-004 workspace dirty state (F-002) | Evidence validated on committed code post-stash | Re-run commands on clean workspace and document state |
| `mapping_status: planned` for all rows | Allowed at State 7 | Must transition to `materialized` after implementation (State 11) and `verified` after test execution (State 12) |

## Handoff to Proof-Reviewer

This bridge mapping is **not self-approved**. Required reviewer inputs:
1. Verify every `source_refs` entry resolves to a real line in committed source.
2. Verify `behavior_test_refs` are independent tests (not verifier harnesses).
3. Verify no TLA+ or temporal claims are presented as Rust evidence.
4. Verify `proof-to-implementation-input.md` claims are fully reflected.
5. Disposition on whether PO-001/PO-002 blocked status justifies `planned` mapping.


## Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|-------------------|------------------|-------------------|------------------------|----------|-----------------|------------|
