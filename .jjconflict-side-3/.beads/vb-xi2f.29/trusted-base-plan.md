# Trusted Base Plan: vb-xi2f.29

**Bead**: vb-xi2f.29 — Digest Covers Together Semantics
**Planner invocation**: p4-plan-vb-xi2f.29-001

## Trust Markers

| ID | Category | Description | Boundary | Status |
|---|---|---|---|---|
| TB-xi2f29-001 | external-dependency | `vb_yaml::ast::StepPrimitive` and `vb_yaml::ast::TogetherBranch` type definitions are correct and match the AST specification. | vb_yaml → vb_compile boundary | trusted |
| TB-xi2f29-002 | external-dependency | `vb_yaml::ast::StepPrimitive::Together` variant has `branches: Vec<TogetherBranch>` with correct field types. | vb_yaml AST types | trusted |
| TB-xi2f29-003 | external-dependency | `blake3::Hasher` is a correct cryptographic hash function. Determinism and collision resistance are assumed. | blake3 crate boundary | trusted |
| TB-xi2f29-004 | implementation-trust | The `canonical_primitive_name` function at part_05.rs:98-114 is already correct for Together (line 105 returns `"together"`). Source fix for this defect is complete. | part_05.rs:105 | trusted |
| TB-xi2f29-005 | bounded-state | `MAX_LANGUAGE_NESTING_DEPTH = 8` from `vb_core/src/limits.rs:63` provides the concrete recursion bound. Validation enforces this before `canonical_digest` is called. | vb_core::limits → vb_compile digest | trusted |
| TB-xi2f29-006 | bounded-state | Recursion in `digest_sub_step` is bounded by `MAX_LANGUAGE_NESTING_DEPTH`. No unbounded recursion or arbitrary-depth trees exist in valid workflows. | digest_sub_step implementation | trusted |

## Assumptions

| ID | Type | Description | Bound | Status |
|---|---|---|---|---|
| ASM-xi2f29-001 | structural | `canonical_digest` is called after YAML parsing and validation. Invalid or malformed ASTs are rejected before reaching digest. | PRE-001 per contract | active |
| ASM-xi2f29-002 | correctness | `canonical_primitive_name` match is exhaustive. The wildcard `_ => "unknown"` arm handles any future variants safely. | All 12 known variants + wildcard | after-fix |
| ASM-xi2f29-003 | determinism | `Vec::iter()` order is deterministic and matches insertion order. Branch hashing order matches source declaration order. | Stable Rust collection iteration | active |
| ASM-xi2f29-004 | determinism | `String::as_bytes()` returns the same bytes for the same string value. No platform-dependent encoding. | UTF-8 string encoding | active |
| ASM-xi2f29-005 | scope | Only `StepPrimitive::Together` arm changes in `digest_step_primitive`. All other primitive arms are unchanged. | C-07 regression invariant | active |
| ASM-xi2f29-006 | scope | The dead code in `compile/mod.rs` is not compiled into the crate binary (not declared in `lib.rs`). It does not affect digest behavior. | compile/mod.rs dead code | active |
| ASM-xi2f29-007 | approach | `digest_sub_step` recurses only on `StepAst.primitive`, not on `StepAst.name`, `condition`, `with`, `retry`, `on_error`, or `then`. These fields are out of scope per contract non-goals. | New digest_sub_step function | active |

## Model Limitations

| ID | Limitation | Impact | Mitigation |
|---|---|---|---|
| LIM-xi2f29-001 | `for_each`, `collect`, `aggregate`, `repeat` also have nested-step blindness. Same root cause as together. | Not fixed in this bead. Future beads needed. | Explicitly out of scope per contract non-goals. |
| LIM-xi2f29-002 | `Aggregate` canonical name returns `"reduce"` (line 107) which may break existing aggregate digests. | Existing workflows with Aggregate primitives will have different digests after fix. | Out of scope per contract. Aggregate fix is a separate bead. |
| LIM-xi2f29-003 | Kani bounded verification uses `kani::unwind(N)` for recursion. If `MAX_LANGUAGE_NESTING_DEPTH` increases beyond Kani's capacity, this proof needs updating. | Proof becomes stale if limit changes. | Trusted-base row TB-xi2f29-006 tracks the bound. |
| LIM-xi2f29-004 | Proptest generates workflows through `compile_source` path which involves full YAML parsing and compilation. If compile pipeline changes independently, test failures could be false positives/negatives for digest. | Test sensitivity to unrelated pipeline changes. | Obligations PO-007 gates existing test regression independently. |
| LIM-xi2f29-005 | This bead does not add `StepAst.name`, `condition`, `with`, `retry`, `on_error`, or `then` fields to the digest. Workflows that differ only in these fields will produce identical digests. | Digest is not fully identity-complete for StepAst. | Explicit non-goal per contract. Full identity completeness is a separate bead. |

## Trusted Surface Summary

The following surfaces are trusted without further proof in this bead:

1. **blake3 crate**: Cryptographic hash correctness. External dependency.
2. **vb_yaml AST types**: Type definitions for `StepPrimitive`, `StepAst`, `TogetherBranch`. Source of truth for structure.
3. **vb_core limits**: `MAX_LANGUAGE_NESTING_DEPTH = 8`. Recursion bound.
4. **Existing `canonical_primitive_name`**: Already returns `"together"` at line 105 (source fix complete).
5. **Rust std**: `Vec::iter()` determinism, `String::as_bytes()` encoding, `u16::to_le_bytes()` endianness.
6. **compile_source pipeline**: Handles validation, compilation, and digest computation together correctly.

## Compensating Evidence Requirements

| Trusted Item | Compensating Evidence |
|---|---|
| TB-xi2f29-003 (blake3) | Existing proptest suite passes for all primitives. No blake3 version change. |
| TB-xi2f29-004 (name fix) | Kani harness PO-xi2f29-001 regression gate. Unit test PO-xi2f29-015. |
| TB-xi2f29-005 (MAX_LANGUAGE_NESTING_DEPTH) | Kani harness PO-xi2f29-009 with unwind(10). |
| ASM-xi2f29-005 (only Together arm changed) | Proptest PO-xi2f29-007 regression gate for all other primitives. |
