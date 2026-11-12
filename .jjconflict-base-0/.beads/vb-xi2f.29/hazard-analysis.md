# Hazard Analysis: Digest Coverage for Together

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Hazard Categories

- **Temporal / Lifecycle**: Digest computed once during compilation; used immutably at runtime.
- **Rust-Core Invariant**: Memory safety, no panics, no unsafe code.
- **Bounded State**: Recursion depth, branch count, step count limits.
- **Refinement / Correctness**: Semantic correctness of digest computation.
- **Concurrency**: Digest computation is single-threaded, pure, no shared mutable state.
- **Hostile Input**: Malformed YAML, excessively deep nesting, empty branches.
- **Performance**: Hashing overhead for deeply nested workflows.
- **Release / API**: Breaking digest format changes.

---

## H-001: Nested Step Recursion Depth Exhaustion

**Category**: Bounded State
**Severity**: LOW (mitigated by existing validation)
**Status**: Mitigated

**Description**: The `canonical_digest` function must recursively traverse nested sub-steps within together branches. If recursion depth exceeds `MAX_CONSTRUCT_DEPTH` (32), a stack overflow could occur in a naive recursive implementation without a guard.

**Mitigation**:
- `MAX_CONSTRUCT_DEPTH` (defined in `vb_core::limits`) is checked during validation before digest computation.
- Any workflow exceeding this depth is rejected at the validation boundary.
- The compiler never feeds a workflow with unbounded nesting depth into `canonical_digest`.

**Residual Risk**: If `canonical_digest` is called on a `WorkflowSource` built programmatically (bypassing validation), deeply nested data could cause stack overflow. Test-only risk; production paths always pre-validate.

---

## H-002: Branch Ordering in Digest

**Category**: Refinement / Correctness
**Severity**: MEDIUM
**Status**: Needs fix

**Description**: Branches in `Together { branches: Vec<TogetherBranch> }` have a defined order (array order in the YAML). The digest must be sensitive to this order. Reordering two branches should produce a different digest.

**Risk**: If branches are hashed via an unordered data structure (e.g., `HashMap`, `BTreeSet`), branch reordering would not change the digest. Current code uses no per-branch hashing at all (higher severity). After the fix, the `for branch in branches` loop naturally preserves ordering.

**Mitigation**: Use `for branch in branches` (ordered iteration). Do not sort, collect into a set, or otherwise reorder branches before hashing.

---

## H-003: Canonical Name Drift

**Category**: Release / API
**Severity**: HIGH
**Status**: Needs fix (bug exists)

**Description**: Fixing `canonical_primitive_name(Together)` from `"parallel"` to `"together"` will change the digest for EVERY existing together workflow. This is a breaking semantic change.

**Impact**: Any existing compiled artifacts (serialized `CompiledWorkflow` values) will have digests that no longer match freshly compiled digests. If digests are used for caching, deployment verification, or artifact deduplication, this will invalidate all cached together workflow artifacts.

**Mitigation**:
- Document the breaking change explicitly.
- Bump the language version or add a migration note.
- Consider whether to batch this fix with the nesting fix to minimize digest-change events.

---

## H-004: Empty Branch List Edge Case

**Category**: Refinement / Correctness
**Severity**: LOW (rejected by validation)
**Status**: Mitigated

**Description**: A `Together { branches: vec![] }` with zero branches is semantically invalid. The digest must handle this case (or never encounter it).

**Mitigation**:
- `validate_together()` in `validation/nodes.rs:47` rejects TogetherStart with zero branches before lowering starts.
- `validate_together_start_edges()` in `workflow/mod.rs:1601` checks branch list is non-empty.
- Digest will never receive a zero-branch together for a valid workflow.

**Residual Risk**: If `canonical_digest` is called on a programmatically constructed `WorkflowSource` with zero branches, the digest computation should still be total (hash "together" + 0u16 + no branches). Adding a debug assertion is recommended.

---

## H-005: Digest Collision with Non-Together Primitives

**Category**: Refinement / Correctness
**Severity**: LOW (blake3 collision resistance)
**Status**: Mitigated

**Description**: After fixing canonical_primitive_name, the digest for a Together step includes `"together"` while the digest for, say, a Collect step includes `"collect"`. A hash collision between two different canonical prefix strings is not possible under the blake3 security model.

**Mitigation**: blake3 provides 128-bit collision resistance. Domain separation via unique prefix strings (canonical names) prevents cross-primitive collisions.

---

## H-006: Incomplete Branch Field Hashing

**Category**: Refinement / Correctness
**Severity**: HIGH
**Status**: Needs fix

**Description**: Even after fixing the canonical name and adding branch label/steps hashing, the `TogetherBranch` struct may gain new fields in the future (e.g., a `condition` field that already appeared in `kani_canonical_name.rs:49` but not in the current `types.rs`). If the digest does not hash new fields, it becomes insensitive to those semantic changes.

**Mitigation**:
- Use exhaustive destructuring `TogetherBranch { label, steps }` in the match arm to get a compiler error if fields are added.
- If `TogetherBranch` is `#[non_exhaustive]`, add a wildcard arm that panics in tests but falls through gracefully in production.
- Current `TogetherBranch` is NOT `#[non_exhaustive]` and has exactly `label` and `steps` fields. If `condition` is added, the compiler will flag the destructuring.

---

## H-007: Dead Code Reactivation

**Category**: Release / API
**Severity**: LOW
**Status**: Mitigated (dead code)

**Description**: `compile/mod.rs` contains duplicate `canonical_digest`, `digest_step_primitive`, and `canonical_primitive_name` with the SAME bugs. If a future developer adds `mod compile;` to `lib.rs`, the dead code becomes active with stale bugs.

**Mitigation**: Delete dead code or add `#[cfg(any())]` compile-error guard. Mark with comment: "Do not activate without syncing fixes from mod_compile_lowering/part_05.rs."

---

## H-008: Recursive Hashing Performance on Deeply Nested Workflows

**Category**: Performance
**Severity**: LOW
**Status**: Mitigated

**Description**: `canonical_digest` is called once during compilation. Deeply nested together structures increase hash computation time linearly with total step count.

**Mitigation**:
- Compilation is a cold-path operation (not on the hot runtime path).
- `blake3` is optimized for throughput (multiple GB/s).
- Maximum step count is bounded by `MAX_STEP_COUNT` (u16 limit).
- Worst case: ~65k steps hashed. Total time < 1ms even on modest hardware.

---

## H-009: Same Digest for ForEach/Collect/Aggregate/Repeat (Scope Limitation)

**Category**: Refinement / Correctness
**Severity**: HIGH (but explicitly out of scope)
**Status**: Known defect, not addressed

**Description**: The same nested-step-blindness defect exists for `for_each`, `collect`, `aggregate`, and `repeat` primitives. Their `body` fields contain sub-steps that are invisible to `canonical_digest`. This bead focuses only on `Together`.

**Impact**: Changing for_each body steps does not change the digest. A future bead must fix this.

**Mitigation**: Document the scope limitation in this bead. The `together` fix establishes the pattern (recursive `digest_sub_step` traversal) that can be reused for other primitives.

---

## H-010: Test Harness Vacuity

**Category**: Refinement / Correctness
**Severity**: MEDIUM
**Status**: Needs verification

**Description**: After implementing the fix, tests must prove they are non-vacuous — i.e., they must verify that the new hash inputs actually enter the hasher and affect the output. A test that constructs two identical workflows and asserts digest equality is vacuous for proving sensitivity.

**Mitigation**: Proptests must generate pairs of workflows that differ ONLY in together branch details and assert digest inequality. Coverage tool (tarpaulin) must confirm the new hasher.update() lines are exercised.

---

## Hazard Risk Matrix

| Hazard | Severity | Likelihood | Residual Risk | Status |
|--------|----------|------------|---------------|--------|
| H-001: Recursion depth | LOW | LOW | LOW | Mitigated |
| H-002: Branch ordering | MEDIUM | LOW (post-fix) | LOW | Needs fix |
| H-003: Canonical name drift | HIGH | CERTAIN (when fixed) | MEDIUM | Needs fix |
| H-004: Empty branches | LOW | LOW | LOW | Mitigated |
| H-005: Cross-primitive collision | LOW | NEGLIGIBLE | LOW | Mitigated |
| H-006: Incomplete fields | HIGH | MEDIUM | MEDIUM | Needs fix |
| H-007: Dead code activation | LOW | LOW | LOW | Mitigated |
| H-008: Performance | LOW | LOW | LOW | Mitigated |
| H-009: Scope limitation | HIGH | CERTAIN | HIGH | Deferred |
| H-010: Test vacuity | MEDIUM | MEDIUM | MEDIUM | Needs verification |
