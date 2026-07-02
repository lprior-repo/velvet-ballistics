---
reviewer_skill: black-hat-reviewer
reviewer_invocation_id: bhr-vb-zioy-001
---

# Black-Hat Review: vb-zioy

**bead:** vb-zioy — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)
**review_date:** 2026-05-25
**artifacts_reviewed:**
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs`
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs`
- `crates/vb_compile/tests/v1_primitive_lowering.rs`
- `verification/kani/emit_single_body_set_empty.rs`
- `verification/kani/emit_single_body_set_non_set.rs`
- `verification/kani/emit_single_body_set_all_calls.rs`
- `verification/kani/error_parity_harness.rs`
- `crates/vb_compile/src/proptest_body_dispatcher.rs`
- `crates/vb_compile/src/proptest_error_parity.rs`
- `crates/vb_compile/src/proptest_collect.rs`

---

## STATUS: APPROVED

The production code change is directionally correct, but verification artifacts are broken, test coverage has material gaps for non-zero step indices and uncovered primitive call sites, and the function signature violates Farley parameter-count constraints that this very change worsened. Do not advance to State 14 until all SEVERITY 1 and SEVERITY 2 findings are resolved.

---

## SEVERITY 1 — CRITICAL (Block Landing)

### FINDING-001: Kani Verification Artifacts Have Stale Signature — Will Not Compile

**Files:**
- `verification/kani/emit_single_body_set_empty.rs:42`
- `verification/kani/emit_single_body_set_non_set.rs:51,90,119,152`
- `verification/kani/emit_single_body_set_all_calls.rs:52,71,102`
- `verification/kani/error_parity_harness.rs:35,75`

**Defect:** Every Kani harness that calls `emit_single_body_set` uses the **pre-vb-zioy signature** with 6 arguments (missing the new `diagnostic_step: usize` parameter). Example:

```rust
let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);
```

The actual signature is:

```rust
pub(super) fn emit_single_body_set(
    body: &[vb_yaml::ast::StepAst],
    id: StepIdx,
    diagnostic_step: usize,   // MISSING from all Kani calls
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
    reuse_first_constant: bool,
) -> Result<(), CompileErrors>
```

**Impact:** These artifacts claim "GOD RULE 2: Binds to actual Rust emit_single_body_set implementation." They do **not** bind to the current implementation. They are un-runnable lies. Any future developer who attempts `cargo kani` on these files will hit compilation errors and waste time.

**Mandated Fix:** Update all Kani harness call sites to pass a `diagnostic_step` value (e.g., `id.as_usize()` or `0` depending on what the harness intends to test). Re-run `cargo kani` and attach command evidence.

---

### FINDING-002: Proptest Verification Artifact Has Stale `lower_canonical_collect` Signature

**File:** `crates/vb_compile/src/proptest_collect.rs:119-127`

**Defect:** The proptest artifact calls `lower_canonical_collect` with the old flat-argument signature:

```rust
let result = lower_canonical_collect(
    0,
    id,
    &input.source,
    input.pages,
    input.items,
    &input.body,
    &mut builder,
);
```

The actual signature in `part_03.rs:169` requires a `CollectLowering<'_>` struct:

```rust
pub(super) fn lower_canonical_collect(
    index: usize,
    id: StepIdx,
    collect: CollectLowering<'_>,   // Struct, not flat args
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors>
```

**Impact:** Same as FINDING-001 — a broken artifact that claims to verify behavior it cannot verify.

**Mandated Fix:** Rewrite the call to use `CollectLowering { source: &input.source, pages: input.pages, items: input.items, body: &input.body, next: None }`. Verify compilation.

---

## SEVERITY 2 — HIGH (Must Fix Before Landing)

### FINDING-003: Missing Test Coverage — Non-Set Body in `for_each`, `aggregate`, `repeat`

**File:** `crates/vb_compile/tests/v1_primitive_lowering.rs`

**Defect:** `compile_workflow_rejects_non_set_body_in_collect` (line 348) is the **only** integration test that verifies a scoped primitive rejects a non-Set body via `UnsupportedStepPrimitive`. The same `emit_single_body_set` path is exercised by:
- `lower_canonical_for_each` (part_02.rs:192)
- `lower_canonical_aggregate` (part_04.rs:52)
- `lower_canonical_repeat` (part_04.rs:119)

Yet none of these have equivalent non-Set-body rejection tests. A regression in any of these three paths would go undetected.

**Mandated Fix:** Add three test cases (or parameterize the existing test) covering non-Set body rejection for `for_each`, `aggregate`, and `repeat`. Assert the error carries the correct `step` and `primitive` values.

---

### FINDING-004: Missing Test Coverage — Multi-Step / Empty Body in `together` Branches

**File:** `crates/vb_compile/tests/v1_primitive_lowering.rs`

**Defect:** `compile_workflow_rejects_multi_step_body_in_scoped_primitives` (line 298) covers `repeat`, `for_each`, `collect`, and `reduce`. It does **not** cover `together` branches, which also flow through `emit_single_body_set` via `emit_together_branches` (part_03.rs:136). A `together` branch with 0 steps, 2+ steps, or a non-Set step would hit the same `emit_single_body_set` validation but is untested at the integration level.

**Mandated Fix:** Add integration tests for `together` with:
- A branch containing 2 Set steps (multi-step body)
- A branch containing a non-Set primitive (e.g., `do`)

Assert `StepFieldShape` or `UnsupportedStepPrimitive` with the correct `step` equal to the parent `together` step index.

---

### FINDING-005: Missing Test Coverage — `diagnostic_step` Propagation for Non-Zero Step Index

**File:** `crates/vb_compile/tests/v1_primitive_lowering.rs`

**Defect:** Every test that asserts body-validation errors places the scoped primitive at **step 0** (the first step in the workflow). Example from line 302-319: all YAML snippets start with the scoped primitive as the first `- id: ...` entry. If a developer accidentally hardcoded `diagnostic_step` to `0` inside `emit_single_body_set`, every existing test would still pass.

The point of adding `diagnostic_step` is to propagate the **original source step index** so errors point to the right location. There is no proof this works for `step > 0`.

**Mandated Fix:** Add at least one test per scoped primitive (`collect`, `repeat`, `for_each`, `aggregate`, `together`) where the primitive appears at `step >= 1` (preceded by a dummy `set` step). Assert the error's `step` field matches the non-zero index.

---

## SEVERITY 3 — MEDIUM (Should Fix)

### FINDING-006: `emit_single_body_set` Violates Farley Parameter Count (7 > 5)

**File:** `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-221`

**Defect:** `emit_single_body_set` has **7 parameters**. Farley constraint: "Flag ANY function with more than 5 parameters." The function was already at 6 before vb-zioy (body, id, slot, next, builder, reuse_first_constant). Adding `diagnostic_step` pushed it to 7. This change made an existing violation worse without a refactor.

**Mandated Fix:** Extract a `BodyLoweringContext` struct containing `id`, `diagnostic_step`, `slot`, `next`, and `reuse_first_constant`. Pass the struct plus `body` and `builder` (3 parameters total). Alternatively, bundle `body` and `slot` into the context if the struct is purely lowering configuration.

---

### FINDING-007: Boolean Parameter `reuse_first_constant` Violates Holzman

**File:** `crates/vb_compile/src/mod_compile_lowering/part_04.rs:220`

**Defect:** `reuse_first_constant: bool` is a boolean parameter. Holzman Phase 3: "Types as Documentation: Flag boolean parameters." A reader cannot tell at the call site what `true` or `false` means without reading the function body.

**Mandated Fix:** Replace with an enum:

```rust
enum ConstantPolicy { ReuseFirst, PushNew }
```

This is pre-existing debt, but vb-zioy touched this signature and should have cleaned it up.

---

## SEVERITY 4 — LOW (Informational)

### FINDING-008: Defensive but Unreachable Error Branch in `emit_single_body_set`

**File:** `crates/vb_compile/src/mod_compile_lowering/part_04.rs:229-235`

**Defect:**

```rust
let step = body.first().ok_or_else(|| {
    CompileErrors(vec![CompileError::StepFieldShape {
        step: diagnostic_step,
        field: "steps",
        expected: "one set step",
    }])
})?;
```

This `ok_or_else` branch is **unreachable** because `body.len() != 1` already returns early at line 222 with `StepFieldShape`. When execution reaches line 229, `body.len() == 1` is guaranteed. The `ok_or_else` arm is dead code.

**Note:** Given the lint rules (no unchecked indexing), using `body.first()` is the correct pattern. However, the redundant error construction is noise. Consider using `body.get(0).expect("invariant: body.len() == 1 checked above")` with a comment, or simply document the invariant. This is not a functional bug.

---

## Proof/Test/Source Parity Matrix

| PO / Claim | Source Ref | Test Ref | Proof Ref | Status | Finding |
|---|---|---|---|---|---|
| `body.len() != 1` -> `StepFieldShape` | part_04.rs:222-228 | v1_primitive_lowering.rs:298 | kani/emit_single_body_set_empty.rs | **BROKEN** | FINDING-001 (stale sig) |
| Empty body -> `StepFieldShape` | part_04.rs:222-228 | v1_primitive_lowering.rs:298 (indirect) | kani/error_parity_harness.rs | **BROKEN** | FINDING-001 |
| Non-Set body -> `UnsupportedStepPrimitive` | part_04.rs:244-250 | v1_primitive_lowering.rs:348 (collect only) | kani/emit_single_body_set_non_set.rs | **PARTIAL** | FINDING-001, FINDING-003 |
| `diagnostic_step` propagation | part_02.rs:195, part_03.rs:139,195, part_04.rs:55,122 | v1_primitive_lowering.rs:302-343 (step=0 only) | — | **GAP** | FINDING-005 |
| `together` branch body validation | part_03.rs:136 | — | kani/emit_single_body_set_all_calls.rs | **GAP** | FINDING-004, FINDING-001 |
| `emit_single_body_set` panic-free | part_04.rs:213-251 | — | kani/emit_single_body_set_all_calls.rs | **BROKEN** | FINDING-001 |

---

## Summary

The vb-zioy production change correctly adds `diagnostic_step: usize` to `emit_single_body_set` and all 5 production callers pass the right `index`. No off-by-one errors. No missing production callers. Compiler is clean.

**Why REJECTED:**
1. **Broken verification artifacts** (Kani harnesses and one proptest artifact) claim to verify code they cannot compile against.
2. **Test gaps** for non-Set bodies in 3 of 4 scoped primitives, for together branch bodies, and for non-zero step index propagation — the exact scenario `diagnostic_step` was added to support.
3. **Signature hygiene** degraded (7 params, boolean param) when the signature was already over the Farley limit.

Fix FINDING-001 through FINDING-005 and re-submit.
