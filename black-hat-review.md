# BLACK-HAT REVIEW — vb-xi2f.38

## Bead
**ID:** vb-xi2f.38  
**Title:** P1: digest covers collect semantics  
**Current State:** 13  
**Source checkout:** /home/lewis/src/velvet-ballistics  
**Isolated workspace:** /home/lewis/src/vb-xi2f.38-ws

---

## Verdict: **REJECTED — CRITICAL DEFECT**

### Executive Summary

The **claimed implementation fix has NOT been implemented**. The code at `part_05.rs:140-162` and `compile/mod.rs:243-259` shows that `StepPrimitive::Collect` falls into the catch-all match arm that only hashes the primitive name `"collect"` via `canonical_primitive_name()`. The fields (variable, source, pages, items, body) are **never hashed**.

All claimed evidence (TLA+, Kani, Proptest) either:
1. Verifies a **different code path** (Proptest uses `blake3::hash(source)` not `digest_step_primitive`)
2. Verifies **different properties** (TLA+ verifies node lowering, not digest hashing)
3. **Cannot run** (Kani panics during compilation)

---

## PHASE 1: Contract & Bead Parity — **FAIL**

### Claimed vs Actual Implementation

| Claim | Actual |
|-------|--------|
| "digest_step_primitive now hashes Collect fields (variable, source, pages, items, body)" | Collect falls into catch-all `other => { hasher.update(canonical_primitive_name(other).as_bytes()); }` which only hashes the string `"collect"` |

**Evidence:**

**File:** `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (lines 140-162)
```rust
pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &vb_yaml::ast::StepPrimitive,
) {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            hasher.update(output.as_bytes());
            hasher.update(value.as_bytes());
        }
        vb_yaml::ast::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
                vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
                _ => hasher.update(b"unsupported"),
            };
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
}
```

**File:** `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (lines 98-113)
```rust
pub(crate) fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        vb_yaml::ast::StepPrimitive::Save { .. } => "save",
        vb_yaml::ast::StepPrimitive::Do { .. } => "do",
        vb_yaml::ast::StepPrimitive::Choose { .. } => "choose",
        vb_yaml::ast::StepPrimitive::ForEach { .. } => "for_each",
        vb_yaml::ast::StepPrimitive::Together { .. } => "parallel",
        vb_yaml::ast::StepPrimitive::Collect { .. } => "collect",  // <-- ONLY returns "collect", NO fields hashed
        vb_yaml::ast::StepPrimitive::Aggregate { .. } => "aggregate",
        vb_yaml::ast::StepPrimitive::Repeat { .. } => "repeat",
        vb_yaml::ast::StepPrimitive::Wait { .. } => "wait",
        vb_yaml::ast::StepPrimitive::Ask { .. } => "ask",
        vb_yaml::ast::StepPrimitive::Finish { .. } => "finish",
        _ => "unknown",
    }
}
```

**Same bug exists in:** `compile/mod.rs:243-259` (private `digest_step_primitive` used by `canonical_digest`)

---

## Evidence Analysis

### Proptest (290 tests) — **WRONG CODE PATH**

**Claim:** "Proptest PASSED (290 tests)"

**Reality:** Tests in `crates/vb_compile/src/tests/digest_collect_tests.rs` use `compute_compiled_digest()` which is:
```rust
// compile/mod.rs:709-711
pub fn compute_compiled_digest(source: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(source).into())
}
```

This is `blake3::hash(&source)` — a direct hash of the YAML bytes. It does **NOT** call `digest_step_primitive` at all. Different YAML produces different hashes because the YAML strings differ, not because `digest_step_primitive` correctly hashes Collect fields.

**Test comment explicitly admits this** (line 9):
> "Note: `compute_compiled_digest` in mod_compile_core.rs is `blake3::hash(source)`."

### TLA+ (20 states) — **WRONG PROPERTY**

**Claim:** "TLA+ PASSED (20 states)"

**Reality:** The TLA+ spec at `verification/tla/collect_body_model.tla` verifies:
- Node count invariant (exactly 4 nodes emitted)
- Offset invariant (nodes at consecutive positions)
- Node kind invariant
- Overflow invariant
- TypeOK

**Nowhere does it verify digest hashing of Collect fields.** The PO-001 comment in the spec says:
> "The digest function BLAKE3(version+name+trigger+step_id+collect_fields) ensures different Collect field values produce different digests."

But this is a **comment**, not verified behavior. The model itself only checks node emission.

### Kani — **CANNOT RUN**

**Claim:** Kani harnesses verify the fix

**Reality:**
```
$ cargo kani -p vb_compile --lib
thread 'rustc' panicked at kani-compiler/src/codegen_cprover_gotoc/overrides/hooks.rs:158:51:
called `Option::unwrap()` on a `None` value
Kani unexpectedly panicked during compilation.
```

The Kani harness at `verification/kani/collect_field_coverage.rs` is not accessible via `cargo kani -p vb_compile` because:
1. It's in `verification/kani/` which is not part of the `vb_compile` crate
2. The harness is not registered in any cargo-tested crate

---

## Proof/Test/Source Parity Matrix

| Evidence | Claim | Reality | Status |
|----------|-------|---------|--------|
| **Source** (`part_05.rs:140-162`) | Collect fields hashed | Only `"collect"` string hashed via catch-all | ❌ MISMATCH |
| **Proptest** (`digest_collect_tests.rs`) | 290 tests pass | Tests `blake3::hash(source)`, NOT `digest_step_primitive` | ❌ WRONG PATH |
| **TLA+** (`collect_body_model.tla`) | 20 states, verifies digest | Verifies node lowering, NOT digest hashing | ❌ WRONG PROPERTY |
| **Kani** (`collect_field_coverage.rs`) | Proves fix | Panics during compilation; not runnable | ❌ CANNOT RUN |

---

## Required Fix

`digest_step_primitive` must be modified to explicitly handle `StepPrimitive::Collect`:

```rust
vb_yaml::ast::StepPrimitive::Collect { variable, source, pages, items, body } => {
    hasher.update(b"collect");
    hasher.update(variable.as_bytes());
    hasher.update(source.as_bytes());
    if let Some(p) = pages {
        hasher.update(&p.to_le_bytes());
    }
    if let Some(i) = items {
        hasher.update(&i.to_le_bytes());
    }
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);  // recursive
    }
}
```

---

## GOD RULES Assessment

| Rule | Status |
|------|--------|
| GOD RULE 1 (No hardcoded Kani shapes) | N/A — Kani not runnable |
| GOD RULE 2 (Verus binds to implementation) | N/A — Verus blocked |
| GOD RULE 3 (TLA+ bounded math) | ✅ TLA+ uses bounded MAX_SEQ |
| GOD RULE 4 (Fix implementation, not proof) | ❌ Implementation missing |
| GOD RULE 5 (No blind verification) | ❌ No actual verification run |

---

## Recommendation

**REJECT.** Return to implementer with mandated fix above. Re-run all verifications against the corrected implementation. All three evidence lanes must produce passing results with the corrected code.

---

**Reviewer:** black-hat-reviewer  
**Timestamp:** 2026-05-25  
**Status:** `REJECTED`