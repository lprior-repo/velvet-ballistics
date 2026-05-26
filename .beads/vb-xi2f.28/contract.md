# Contract — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Acceptance Criteria

This section defines the acceptance contract for bead vb-xi2f.28. All criteria must be satisfied for the bead to be considered complete.

### 1.1 Primary Acceptance: ForEach Digest Sensitivity

**AC-FE-01:** Changing `ForEach.input` (the input collection expression) in the YAML source MUST produce a different `canonical_digest` than the original source.

**AC-FE-02:** Changing `ForEach.at_once` (the max concurrency limit) in the YAML source MUST produce a different `canonical_digest` than the original source.

**AC-FE-03:** Changing `ForEach.variable` (the loop variable name) in the YAML source MUST produce a different `canonical_digest` than the original source.

**AC-FE-04:** Changing any text content of any body step in `ForEach.body` MUST produce a different `canonical_digest` than the original source.

### 1.2 Secondary Acceptance: Determinism Preserved

**AC-FE-05:** After the fix, `canonical_digest` remains deterministic: compiling the same identical YAML source multiple times MUST produce the same digest every time.

**AC-FE-06:** After the fix, both compilation paths (`compile/mod.rs` and `mod_compile_lowering/part_05.rs`) MUST produce identical digests for identical input.

### 1.3 Tertiary Acceptance: Non-Decomposition

**AC-FE-07:** After the fix, semantically equivalent ForEach inputs produce equivalent digests:
- `at_once: None` and `at_once: 1` (or absent `at_once` field) produce the same `canonical_digest` contribution (both canonicalize to limit=1 which hashes as `0u32`; however, `Some(1)` hashes as `1u32.to_le_bytes()` — this needs a domain decision).
  
  **Decision:** Per the lowering code (`part_02.rs:172`: `limit: at_once.unwrap_or(1)`), `None` and `Some(1)` are semantically equivalent. The digest should reflect this: both hash as `1u32.to_le_bytes()`. The canonical representation is the resolved limit value (1 when absent), not the Option wrapper.

  **CORRECTION:** After deeper analysis, the canonical representation should hash `at_once.unwrap_or(1)`, i.e., the *resolved* limit value. This ensures semantic equivalence. `at_once: None` and `at_once: Some(1)` both hash as `1u32.to_le_bytes()`.

- `ForEach { body: [step1] }` and `ForEach { body: [step1, step2] }` where step2 is a no-op: these produce different digests (because body contains different steps). This is correct — the body content differs.

### 1.4 Scope Boundaries

**AC-FE-08:** This bead addresses ForEach ONLY. The following primitives are explicitly OUT OF SCOPE and may retain the current catch-all hashing behavior:
- `Collect`, `Aggregate` (reduce), `Repeat`, `Together` (parallel), `Wait`, `Ask`, `Choose`, `Do`, `Save`

**AC-FE-09:** The `compute_compiled_digest` function (`mod_compile_core.rs:114`) is NOT in scope. It already correctly hashes the full serialized IR and is sensitive to ForEach field changes at the compiled-artifact level.

---

## 2. Behavioral Specification

### 2.1 What MUST Change

#### digest_step_primitive (both copies)

```rust
// BEFORE (current — gap):
fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &StepPrimitive) {
    match primitive {
        StepPrimitive::Set { output, value } => { /* full coverage */ }
        StepPrimitive::Finish { result } => { /* full coverage */ }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes()); // NAME ONLY
        }
    }
}

// AFTER (fix — contract):
fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &StepPrimitive) {
    match primitive {
        StepPrimitive::Set { output, value } => { /* full coverage — unchanged */ }
        StepPrimitive::Finish { result } => { /* full coverage — unchanged */ }
        StepPrimitive::ForEach { variable, input, at_once, body } => {  // NEW ARM
            hasher.update(b"for_each");
            hasher.update(b"variable:");
            hasher.update(variable.as_bytes());
            hasher.update(b"input:");
            hasher.update(input.as_bytes());
            hasher.update(b"at_once:");
            let limit = at_once.unwrap_or(1);
            hasher.update(&limit.to_le_bytes());
            hasher.update(b"body:");
            for step in body {
                hasher.update(step.id.as_bytes());
                digest_step_primitive(hasher, &step.primitive);
            }
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes()); // unchanged for remaining primitives
        }
    }
}
```

### 2.2 What MUST NOT Change

- The function signatures (`canonical_digest`, `digest_step_primitive`, `canonical_primitive_name`)
- The return type (`WorkflowDigest`)
- The existing Set and Finish hashing behavior
- The outer `canonical_digest` structure (version, name, trigger, step loop)
- The `lower_steps_to_ir` function or any lowering/compilation logic
- Any production code outside `crates/vb_compile/src/compile/mod.rs` and `crates/vb_compile/src/mod_compile_lowering/part_05.rs`

### 2.3 What Is Optional (Design Decisions)

| Decision | Choices | Recommended | Rationale |
|---|---|---|---|
| Field delimiter character | `b":"`, `b"\n"`, `b"\0"`, length-prefixed | `b":"` | Simple, unambiguous for YAML identifiers |
| at_once canonical form | Hash None as 0, Hash None as unwrap_or(1), Hash as Option tag+value | `at_once.unwrap_or(1)` | Matches lowering semantics; None→1 equivalence preserved |
| Body step ID prefix | Include step ID, Exclude step ID | Include step ID | Step ID is part of source; changing step IDs should change digest |
| ForEach arm placement | Before or after other arms in match | Before `other` catch-all | Explicit arm prevents fall-through; compiler will warn if unused |

---

## 3. Verification Requirements

### 3.1 Required Tests

| Test ID | Description | Type | Covers |
|---|---|---|---|
| **TST-FE-01** | `canonical_digest` differs when ForEach.input changes | Integration | AC-FE-01 |
| **TST-FE-02** | `canonical_digest` differs when ForEach.at_once changes | Integration | AC-FE-02 |
| **TST-FE-03** | `canonical_digest` differs when ForEach.variable changes | Integration | AC-FE-03 |
| **TST-FE-04** | `canonical_digest` differs when ForEach.body content changes | Integration | AC-FE-04 |
| **TST-FE-05** | `canonical_digest` is identical for identical ForEach sources | Integration | AC-FE-05 |
| **TST-FE-06** | Both compilation paths produce identical digests for same source | Cross-path integration | AC-FE-06 |
| **TST-FE-07** | Semantically equivalent ForEach (at_once=None vs Some(1)) produces same digest | Proptest | AC-FE-07 |
| **TST-FE-08** | Existing determinism tests still pass | Regression | AC-FE-05 |
| **TST-FE-09** | Non-ForEach primitives produce digests unchanged (Set, Finish) | Regression | AC-FE-08 |

### 3.2 Required Proofs

| Proof ID | Description | Verifier |
|---|---|---|
| **PRF-FE-01** | `canonical_digest` is deterministic (pure function) for all StepPrimitive variants | Kani / proptest |
| **PRF-FE-02** | ForEach field hashing is exhaustive (all 4 fields covered) | Kani / static analysis |
| **PRF-FE-03** | Both copies of `digest_step_primitive` produce identical behavior | Proptest |

---

## 4. Non-Requirements (Explicitly Out of Scope)

1. **Do NOT** consolidate the two `canonical_digest` copies into a single shared function. This is a separate refactoring bead.
2. **Do NOT** add field hashing for primitives other than ForEach (Collect, Repeat, Together, etc.).
3. **Do NOT** modify `lower_steps_to_ir`, `canonical_layout`, `lower_canonical_for_each`, or any lowering/IR logic.
4. **Do NOT** modify `WorkflowDigest`, `WorkflowParts`, `CompiledWorkflow`, or any `vb_core` types.
5. **Do NOT** change the format or serialization of the digest itself.
6. **Do NOT** add a version field to distinguish pre-fix from post-fix digests.

---

## 5. Contract Breach Conditions

The following conditions constitute a **breach of this contract** and require the bead to be re-opened:

1. Any ForEach field change does not produce a different `canonical_digest` (AC-FE-01 through AC-FE-04 violated)
2. `canonical_digest` becomes non-deterministic after the fix (AC-FE-05 violated)
3. Two compilation paths produce different digests (AC-FE-06 violated)
4. Existing Set/Finish digest behavior is altered (regression)
5. Any production code outside the two specified files is modified (scope violation)
6. Any out-of-scope primitive's field hashing is added (scope violation — save for future beads)
