# Error Taxonomy: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** error-taxonomy/v1

## 1. Error Categories

### EC-1: Digest Insensitivity Error (DESIGN — the bug being fixed)
**Category:** Semantic / Integrity
**Severity:** HIGH
**Domain operation:** `digest_step_primitive(hasher, Wait{..})`
**Error:** The `catch-all` arm (`other =>`) hashes only `canonical_primitive_name(Wait{..})`, which returns `"wait"`. The `event` and `timeout` fields are silently ignored.
**Consequence:** Two workflows with different wait semantics produce identical `WorkflowDigest` values. The digest does not reflect the semantic content of the workflow.
**Root cause:** Missing match arm for `StepPrimitive::Wait` in `digest_step_primitive`.
**Fix:** Add an exhaustive `Wait { event, timeout }` match arm that hashes both fields with discriminators.

### EC-2: Duplicate Implementation Error (MAINTENANCE)
**Category:** Architecture / Drift
**Severity:** MEDIUM
**Domain operation:** Any change to `canonical_digest`, `digest_step_primitive`, or `canonical_primitive_name`.
**Error:** Identical logic exists in two files:
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (cold-path compiler)
- `crates/vb_compile/src/compile/mod.rs` (warm-path compiler)
**Consequence:** Any fix applied to only one copy causes the two compiler paths to diverge, producing different digests for the same workflow source.
**Root cause:** Code duplication without a shared module.
**Fix:** Apply the same fix to BOTH copies. File a follow-up bead for deduplication.

### EC-3: Missing Test Coverage Error (QUALITY)
**Category:** Test gap
**Severity:** HIGH
**Domain operation:** Digest sensitivity verification.
**Error:** No test verifies that different wait conditions produce different `canonical_digest` values.
- `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` only tests stability (same → same), not sensitivity (different → different).
- `compiled_digest_is_deterministic` only tests `compute_compiled_digest`, not `canonical_digest`.
**Consequence:** The EC-1 bug was never detected by existing tests. Future regressions in wait digest coverage would also go undetected.
**Fix:** Add proptest or unit tests verifying different wait configurations produce different digests.

### EC-4: Digest Algorithm Inconsistency Error (ARCHITECTURE)
**Category:** Design / Spec
**Severity:** MEDIUM
**Domain operation:** Digest comparison.
**Error:** `canonical_digest` and `compute_compiled_digest` produce different hash values for the same logical workflow.
**Consequence:** Two different "digests" floating through the system. The engine uses the `WorkflowParts.digest` populated by `canonical_digest`, while artifact storage uses `compute_compiled_digest`. They are not interchangeable.
**Fix:** Not in this bead scope. Document the inconsistency. Consider unification in a future bead.

## 2. Railway Error Variants (Existing, Unchanged)

These are domain errors that exist in the compilation pipeline and are not introduced by this bead. They remain valid and unchanged:

| Variant | Crate | Trigger | Handled by |
|---------|-------|---------|-----------|
| `CompileError::StepFieldShape` | vb_compile | Invalid wait field combination (both None) | `validate_wait_shape` at validation boundary |
| `CompileError::UnsupportedStepPrimitive` | vb_compile | Unknown step primitive type | `lower_canonical_step` or `compile_workflow` |
| `CompileError::SlotIndexOutOfRange` | vb_compile | Slot expression resolves to out-of-range index | `slot_from_text` |
| `CompileError::EmptySteps` | vb_compile | Workflow has zero steps | `compile_source` |
| `CompileError::NonStringKey` | vb_compile | YAML map has non-string key | `non_string_key_error` |

## 3. Error Propagation Map

```
YAML Source
    │
    ▼
[validation] ─────► CompileError::StepFieldShape (reject invalid wait)
    │                       │
    │ OK                    │ Err → termination
    ▼
[canonical_digest] ── (no errors — pure, panics on allocation failure only)
    │
    ▼
[lower_canonical_wait] ──► CompileErrors (slot resolution failure)
    │
    │ OK
    ▼
[digest_step_primitive] ── (CURRENT BUG: silently drops wait fields)
    │                            │
    ▼                            ▼
  Digest computed         INCORRECT digest (before fix)
  (after fix: correct)
```

## 4. New Error Considerations (Post-Fix)

| Error | Category | Handling |
|-------|----------|----------|
| Cold-path / warm-path digest mismatch | Integrity | Not detected at compile time. Runtime integrity check would catch it. |
| Hash ordering divergence between two copies | Implementation | Prevented by identical fix in both copies. |
| Breaking existing persisted digests | Compatibility | Expected — any persisted `WorkflowDigest` from before the fix will differ from post-fix digests for workflows containing wait steps. Requires recompilation and re-persistence. |

## 5. Error Severity Matrix

| Error ID | Pre-Fix | Post-Fix | Behavior Affecting |
|----------|---------|----------|--------------------|
| EC-1: Digest Insensitivity | HIGH (active) | RESOLVED | YES — two workflows with different wait semantics treated as identical |
| EC-2: Duplicate Implementation | MEDIUM (latent) | PARTIALLY RESOLVED (both copies fixed, but duplication remains) | YES — if copies diverge |
| EC-3: Missing Test Coverage | HIGH (latent) | REQUIRES TEST BEAD | YES — regression risk |
| EC-4: Digest Algorithm Inconsistency | MEDIUM (latent) | UNCHANGED | YES — but out of scope |
