# Contract: Digest Covers Together Semantics

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Context

The `canonical_digest()` function in `vb_compile::mod_compile_lowering::part_05.rs` computes a blake3 hash of a `WorkflowSource` to produce a `WorkflowDigest`. This digest is stored in compiled workflow artifacts. Currently, for `StepPrimitive::Together` steps, only the step ID and the string `"parallel"` (a bug — should be `"together"`) enter the digest. Branch labels, branch counts, branch ordering, and sub-step contents within branches are NOT hashed. This contract establishes the requirements for making the digest sensitive to together semantics.

## Preconditions

- **PRE-001**: `canonical_digest(source)` is called on a `WorkflowSource` that has passed YAML parsing and basic validation.
- **PRE-002**: `blake3::Hasher` is available and deterministic.
- **PRE-003**: The `WorkflowSource` AST is a finite tree; no cyclic references exist.
- **PRE-004**: Active digest code is in `mod_compile_lowering/part_05.rs`. Dead code in `compile/mod.rs` is not compiled.
- **PRE-005**: `StepPrimitive::Together` has `branches: Vec<TogetherBranch>` where `branches.len() > 0` (enforced by validation).

## Postconditions

- **POST-001**: `canonical_primitive_name(Together { .. })` returns `"together"`.
- **POST-002**: `canonical_digest(source)` includes for each `Together` step:
  - The canonical name `"together"`
  - The branch count as a `u16` little-endian value
  - Each branch's `label` string, hashed in array order
  - Each branch's sub-steps (IDs and primitives), recursively hashed
- **POST-003**: Changing any field of a `TogetherBranch` (label, steps) produces a different `WorkflowDigest`.
- **POST-004**: Adding, removing, or reordering branches produces a different `WorkflowDigest`.
- **POST-005**: `canonical_digest(source)` is deterministic: same source → same digest.
- **POST-006**: The digest computation does not panic for any valid `WorkflowSource`.
- **POST-007**: Digest computation remains pure: no I/O, no side effects.

## Invariants

- **INV-001 (Sensitivity)**: For all `a`, `b`: `WorkflowSource` where `a != b` and `a`, `b` differ only in together `branches` structure → `canonical_digest(a) != canonical_digest(b)`.
- **INV-002 (Name Correctness)**: `canonical_primitive_name(Together { .. }) == "together"` (not `"parallel"`).
- **INV-003 (Determinism)**: `canonical_digest(X) == canonical_digest(X)` always.
- **INV-004 (No Panic)**: `canonical_digest(source)` never panics.
- **INV-005 (Branch Ordering)**: `branches` are hashed in array-iteration order.
- **INV-006 (Recursive Completeness)**: Every `StepAst` within every `TogetherBranch.steps` is hashed via `digest_sub_step`, which processes `id` and `primitive` recursively.

## Contract Clauses (Verifiable)

### C-01: Canonical Name Fix
**Statement**: `canonical_primitive_name(StepPrimitive::Together { branches: _ }) == "together"`
**Source**: `part_05.rs:105` (currently `=> "parallel"`)
**Verifiers**: Kani (existing harness `canonical_name_together_harness`), Unit test

### C-02: Branch Count in Digest
**Statement**: `canonical_digest(source)` includes `branches.len() as u16` in the hash for each Together step.
**Source**: New code in `digest_step_primitive` Together arm
**Verifiers**: Proptest (generate different branch counts, assert digest inequality), Unit test

### C-03: Branch Labels in Digest
**Statement**: `canonical_digest(source)` includes each `branch.label` in array order.
**Source**: New code in `digest_step_primitive` Together arm
**Verifiers**: Proptest (generate workflows with different labels, assert digest inequality)

### C-04: Sub-Step Contents in Digest
**Statement**: `canonical_digest(source)` recursively hashes each `StepAst` within each `TogetherBranch.steps`.
**Source**: New `digest_sub_step` function
**Verifiers**: Proptest (generate workflows with different sub-step primitives, assert digest inequality)

### C-05: Branch Ordering in Digest
**Statement**: Reordering `branches` produces a different digest.
**Source**: Implicit from array-ordered iteration in the fix
**Verifiers**: Proptest (generate same branches in different order, assert digest inequality)

### C-06: Determinism Preservation
**Statement**: Same `WorkflowSource` always produces the same digest, even after the fix.
**Source**: All changed code must be deterministic
**Verifiers**: Proptest (idempotency: `digest(s) == digest(s)` after multiple calls), existing tests

### C-07: No Regression on Non-Together Digests
**Statement**: Workflows without Together steps produce the same digests as before the fix (modulo canonical name fix for Aggregate, which is out of scope).
**Source**: Only Together-arm changes in `canonical_primitive_name` and `digest_step_primitive`
**Verifiers**: Existing proptest suite (`proptest_equal_primitive_sources_compile_to_equal_digest_and_ir`)

### C-08: Kani Proof Update
**Statement**: Kani harness `canonical_name_together_harness` must pass after the fix.
**Source**: `kani_canonical_name.rs:42-62`
**Verifiers**: `cargo kani --harness canonical_name_together_harness`

## Non-Goals

- Fixing `for_each`, `collect`, `aggregate`, `repeat` nested-step blindness (future beads).
- Fixing `Aggregate` canonical name (`"aggregate"` → `"reduce"`) — adjacent bug, out of scope.
- Changing `compute_compiled_digest` (byte-level digest).
- Deleting dead code in `compile/mod.rs`.
- Adding new step-level fields (condition, with, retry, on_error, then) to digest.
- Changing the blake3 hashing algorithm.
- Digesting `StepAst` field-level data: `name`, `condition`, `with`, `retry`, `on_error`, `then`.

## Acceptance Criteria

1. `canonical_primitive_name(Together)` returns `"together"`.
2. `cargo kani --harness canonical_name_together_harness` passes.
3. Proptest: two together workflows with different branch labels → different digests.
4. Proptest: two together workflows with different branch counts → different digests.
5. Proptest: two together workflows with different sub-step primitives → different digests.
6. Proptest: two together workflows with branches in different order → different digests.
7. All existing tests continue to pass (no regressions).
8. No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unsafe`, or `dbg` in changed code.
