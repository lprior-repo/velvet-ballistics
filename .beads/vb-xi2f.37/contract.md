# Contract Specification - vb-xi2f.37

## Context
- **Bead ID**: vb-xi2f.37
- **Title**: P0: accept canonical reduce primitive name
- **Scope**: reduce primitive in YAML parse/validate/compile pipeline
- **Domain terms**: StepPrimitive, reduce, is_primitive, reject_unknown_step_fields, parse_step_primitive, canonical_primitive_name
- **Assumptions**: YAML is cold authoring only; runtime never interprets YAML directly

## Key Findings from rust-contract

### Required Changes
1. Add "reduce" to `is_primitive()` in `vb_yaml/src/ast/parse_steps.rs`
2. Add "reduce" to `reject_unknown_step_fields()` in `vb_yaml/src/ast/parse_steps.rs`
3. Add "reduce" case to `parse_step_primitive()` in `vb_yaml/src/ast/parse_steps.rs`
4. Add `StepPrimitive::Reduce` variant in `vb_yaml/src/ast/types.rs`
5. `canonical_primitive_name()` needs `Reduce -> "reduce"` mapping

## Contract Clauses

### CC-001: Parsing - reduce primitive is recognized
- **When**: YAML step contains `reduce:` key
- **Then**: `parse_step_primitive()` returns `StepPrimitive::Reduce` variant
- **Verifier**: Kani, unit tests

### CC-002: Validation - reduce field is allowed
- **When**: `reject_unknown_step_fields()` is called on a reduce step
- **Then**: It does NOT reject "reduce" as unknown
- **Verifier**: unit tests

### CC-003: Type - Reduce variant exists in StepPrimitive enum
- **When**: Rust code references `StepPrimitive::Reduce`
- **Then**: The variant is defined with correct fields
- **Verifier**: code inspection, Verus

### CC-004: Canonical name - reduce maps to "reduce"
- **When**: `canonical_primitive_name()` is called with `StepPrimitive::Reduce`
- **Then**: Returns "reduce"
- **Verifier**: unit tests

## Error Taxonomy
- YamlError::UnknownField - when reduce is not in primitive list
- YamlError::FieldShape - malformed reduce step

## Proof Seeds (12 total)
1. Unit test: is_primitive("reduce") returns true
2. Unit test: reject_unknown_step_fields accepts reduce step
3. Unit test: parse_step_primitive handles reduce
4. Unit test: canonical_primitive_name(Reduce) == "reduce"
5. Integration test: full YAML parse with reduce step
6. Integration test: reduce rejected before changes (negative)
7. Kani harness: parse_step_primitive no panic on reduce
8. Kani harness: is_primitive bounds check
9. Code inspection: Reduce variant fields match spec
10. Code inspection: all primitive match arms handle reduce
11. Code inspection: no other primitive name collisions
12. Fuzz target: random reduce step generation

## Non-goals
- Runtime behavior of reduce (compile-only change)
- Cross-pipeline effects beyond parse/validate
- Changes to other primitives
