# Proof Strategy: vb-xi2f.36 — `together` Primitive Acceptance

## Bead Summary

| Field | Value |
|-------|-------|
| Bead ID | vb-xi2f.36 |
| Title | P0: accept canonical together primitive name |
| Priority | P0 |
| Contract clause | P1–P8, E1–E2, BC1, INV1 |
| Risk profile | YAML input validation, parser-boundary, compile-boundary, backward-compatibility |

---

## Proof Objective

Prove that the workflow pipeline correctly recognizes `together` as a valid step primitive in place of `parallel`, and that all diagnostic messages use the canonical name `together`.

**Core changes required:**

1. **vb_yaml/src/ast/parse_steps.rs**: Add `"together"` to `is_primitive()` matches and add `"together"` arm in `parse_step_primitive()` match
2. **vb_validate/src/schema.rs**: Add `"together"` to `STEP_PRIMITIVES` and `ALLOWED_STEP_FIELDS` arrays
3. **vb_validate/src/schema_fields.rs**: Add `"together"` to `STEP_PRIMITIVES` and `ALLOWED_STEP_FIELDS` arrays
4. **vb_validate/src/validation.rs** (if separate): Add `"together"` to `STEP_PRIMITIVES` and `ALLOWED_STEP_FIELDS` arrays
5. **vb_compile/src/mod_compile_lowering/part_09.rs**: Add `"together" => Some(Self::Parallel)` to `StepPrimitive::from_field()` and change `as_str()` to return `"together"` for `Self::Parallel`

**Backward compatibility**: Keep `"parallel"` in `is_primitive()` during transition window per BC1.

---

## Risk Classification

| Risk Tag | Description | Affected Functions | Severity |
|----------|-------------|-------------------|----------|
| `parser-boundary` | YAML input rejected at parse layer | `is_primitive()`, `parse_step_primitive()` | HIGH |
| `yaml-input` | Malformed YAML causes wrong error | `parse_parallel()` | HIGH |
| `validation-boundary` | Schema rejects valid `together` | `STEP_PRIMITIVES` arrays | HIGH |
| `compile-boundary` | Lowering fails or wrong string | `StepPrimitive::from_field()`, `as_str()` | HIGH |
| `backward-compatibility` | Existing `parallel` YAML breaks | `is_primitive()` | MEDIUM |
| `illegal-state` | Empty branches allowed by construction | `parse_parallel()` | MEDIUM |
| `type-invariant` | `branches.len() >= 1` not enforced | `StepPrimitive::Together` construction | MEDIUM |

---

## Verification Lane Decisions

### Lane Selection Rationale

| Lane | Tools | Justification |
|------|-------|---------------|
| **Kani** | `cargo kani` | Bounded model checking for panic-freedom on all parse/compile paths; exhausts YAML input space for the 10 proof seeds |
| **Verus** | `cargo verus` | Pure function properties on `is_primitive()`, `from_field()`, `as_str()`; type invariant proofs for `Together` |
| **Proptest** | `cargo test` | Grammar-based input generation for `parse_step_primitive()` with `together`; error contract verification |
| **Miri** | `cargo miri test` | Detect UB in YAML parsing of `together` (no unsafe expected but defensive) |

### Lanes NOT Selected

| Lane | Reason |
|------|--------|
| TLA+ | State machine is trivial (single primitive parse); bounded checkers sufficient |
| Flux | No dependent types required; pure refinements via Verus |
| Loom | No concurrency in parse/compile pipeline for this bead |

---

## Proof Obligations

### Parse Layer (vb_yaml)

| ID | Obligation | Method | Artifact |
|----|-----------|--------|----------|
| PO-01 | `is_primitive("together") == true` | Kani harness with arbitrary string | `kani/harnesses/is_primitive_together.rs` |
| PO-02 | `parse_step_primitive()` matches `"together"` to `parse_parallel()` | Kani harness on YAML with `together` key | `kani/harnesses/parse_step_together.rs` |
| PO-03 | `parse_parallel()` returns `StepPrimitive::Together { branches }` with non-empty | Kani + Verus | `kani/harnesses/parse_parallel_together.rs`, `verus/parse_parallel.rs` |
| PO-04 | Empty `together: {}` returns `YamlError::FieldShape` | Proptest | `proptest/parse_together_errors.rs` |
| PO-05 | `together: { branches: [] }` returns shape/budget error | Proptest | `proptest/parse_together_errors.rs` |

### Validation Layer (vb_validate)

| ID | Obligation | Method | Artifact |
|----|-----------|--------|----------|
| PO-06 | `validate_workflow_schema()` accepts `together` as step primitive | Kani | `kani/harnesses/validate_together.rs` |
| PO-07 | All three `STEP_PRIMITIVES` arrays contain `"together"` | Source inspection + Kani | `kani/harnesses/step_primitives_array.rs` |

### Compile Layer (vb_compile)

| ID | Obligation | Method | Artifact |
|----|-----------|--------|----------|
| PO-08 | `StepPrimitive::from_field("together") == Some(Parallel)` | Kani + Verus | `kani/harnesses/from_field_together.rs`, `verus/from_field.rs` |
| PO-09 | `StepPrimitive::as_str(Parallel) == "together"` | Verus pure fn proof | `verus/as_str.rs` |
| PO-10 | `lower_together()` produces `CompiledNodeKind::TogetherStart` | Kani | `kani/harnesses/lower_together.rs` |

### Backward Compatibility

| ID | Obligation | Method | Artifact |
|----|-----------|--------|----------|
| PO-11 | `is_primitive("parallel") == true` (alias still works) | Kani | `kani/harnesses/is_primitive_parallel.rs` |

### Invariants

| ID | Obligation | Method | Artifact |
|----|-----------|--------|----------|
| PO-12 | `Together { branches }` has `branches.len() >= 1` after parse | Verus invariant proof | `verus/together_invariant.rs` |

---

## Trusted Base

The following functions are **trusted** (assumed correct, not verified in this bead):

| Function | Location | Reason Trusted |
|----------|----------|----------------|
| `lookup()` | vb_yaml/src/ast/parse.rs | Preexisting; tested in vb_yaml tests |
| `mapping()` | vb_yaml/src/ast/parse.rs | Preexisting; tested |
| `require_str_in()` | vb_yaml/src/ast/parse.rs | Preexisting; tested |
| `sequence()` | vb_yaml/src/ast/parse.rs | Preexisting; tested |
| `reject_unknown_fields()` | vb_yaml/src/ast/parse.rs | Preexisting; tested |
| `lower_together()` branch count limit check | vb_compile/src/compile/mod.rs | Preexisting; covered by budget tests |
| `CompiledNodeKind::TogetherStart` | vb_core | Preexisting; structural tests exist |

---

## Exit Criteria

All 12 proof obligations must have evidence (Kani report, Verus proof, or Proptest pass) before this bead is considered proof-complete.

Evidence artifacts:
- `verification/kani/*.rs` harnesses with passing `cargo kani` output
- `verification/verus/*.rs` with `cargo verifast` clean output
- `verification/proptest/*.rs` with `cargo test` pass
- Raw command evidence logged in `proof-obligations.planned.jsonl`
