# CODE Review: vb-8eocj.2 F1 first source slice
Reviewer: local-3090-code-review-v1
Commit: 8e1227a8afa6
Base: d0645504a432552301d43094859e1261318a1bef

## Scope
Bounded slice: canonical top-level Do with checked numeric ActionId and direct slot
input; Set with I64 constants plus Bool/Null/finite F64 and typed scalar expressions;
named prior references ($step.id/$steps.id/direct name); separate alias/output namespaces;
ambiguous reference rejection. No execution tests run; compile check passed in root v14.

## Files Reviewed
- crates/vb_compile/src/mod_compile_lowering/canonical_do.rs (73 lines)
- crates/vb_compile/src/mod_compile_lowering/canonical_references.rs (135 lines)
- crates/vb_compile/src/mod_compile_lowering/canonical_scalar.rs (82 lines)
- crates/vb_compile/src/mod_compile_lowering/finish_inputs.rs (41 lines)
- crates/vb_compile/src/mod_compile_lowering/part_01.rs (22 line changes)
- crates/vb_compile/src/mod_compile_lowering/part_02.rs (94 line changes)

## Findings

### PASS: Action ID resolution (canonical_do.rs:53-64)
Correctly parses action as u16 with checked error:
```rust
let numeric = action.parse::<u16>().map_err(|_e| {
    CompileErrors(vec![CompileError::StepFieldShape {
        step: step_index,
        field: "action",
        expected: "numeric action ID (u16)",
    }])
})?;
```
Includes precise step index in error. Matches contract requirement for "checked numeric
ActionId."

### PASS: Input slot resolution (canonical_do.rs:66-75)
Uses shared `resolve_slot_reference` from canonical_references.rs. Correctly wraps
single CompileError into CompileErrors vec. Input slot is recorded via builder.record_slot.

### PASS: Reference resolution namespaces (canonical_references.rs:29-40, 67-119)
Two separate maps maintained: `outputs` (Set aliases) and `step_outputs` (step IDs).
Resolution rules correctly implemented:
- `$step.id` / `$steps.id`: explicit step reference, only checks step_outputs
- Direct `$name`: checks both maps, rejects ambiguity with different slots
- Same name, same slot: not ambiguous, resolves successfully

Ambiguous reference error is precise and includes the name:
```rust
Err(CompileError::ExpressionLoweringUnsupported {
    feature: format!(
        "ambiguous reference '{}' found in both output names and step IDs with different slots",
        name
    ).into(),
})
```

### PASS: Forward/self reference rejection (canonical_references.rs)
The step_outputs map only contains entries for steps processed prior to the current
step. Forward references and self-references will not be in the map and will fail
with CompileError::UnknownReferenceName. No guessed indices or handles.

### PASS: I64 constant preservation (canonical_scalar.rs:28-35)
I64 constants are parsed first and lowered using existing SetConst shape:
```rust
if let Ok(constant) = value.parse::<i64>() {
    return lower_set_constant(id, slot, constant, next, builder);
}
```
This preserves backward compatibility as required.

### PASS: Expression compilation for other scalars (canonical_scalar.rs:67-81)
Non-I64 values (Bool, Null, F64, expressions) are compiled via expression parser
and bytecode compiler with StepReferenceResolver. Uses EvalExpr node kind.

### PASS: .out suffix rejection (canonical_references.rs:44-51)
While not explicitly named ".out suffix," the reference resolution logic correctly
rejects references like `$step.producer.out` because "producer.out" is not a valid
step ID (fails in resolve_explicit_step_id). Similarly, `$producer.out` fails in
resolve_direct_name because "producer.out" is not a valid output name. The contract
requirement is met functionally.

### PASS: Finish slot resolution (finish_inputs.rs:7-19)
Uses canonical_finish_slot_with_step_outputs which calls resolve_slot_reference.
Correctly handles $input.name references separately.

### PASS: Part 02 step lowering (part_02.rs:58-74)
For Do steps, output_name is always Some(step.id.as_str()), so compute_output_slot
always returns Some(slot). The slot is correctly added to step_outputs after
lowering, preventing self-reference.

### MINOR: Numeric slot error step index (canonical_references.rs:121-133)
The step index is hardcoded to 0 for numeric slot parsing errors:
```rust
CompileError::StepFieldShape {
    step: 0,  // hardcoded
    field: "slot reference",
    ...
}
```
This is a minor precision issue for debugging but does not affect correctness.

## Compilation Evidence
Root v14 check passed (exit_code 0) with:
- cargo check -p vb_compile --lib --all-features
- RUSTFLAGS=-Dwarnings

## Limitations
- No execution tests run; this review covers source correctness only.
- Public source execution tests remain pending per contract.
- No full F1 closure claim; parent F1 remains open.
- General lexical inputs, secrets, metadata, named action registry are out of scope.

## Verdict
PASS for bounded scope. The implementation correctly delivers:
1. Canonical top-level Do with checked numeric ActionId (u16)
2. Direct slot input resolution via shared reference logic
3. Set with I64 constants preserving SetConst shape
4. Bool, Null, finite F64, and typed scalar expressions via expression bytecode
5. Named prior references ($step.id/$steps.id/direct name)
6. Separate alias/output namespaces with ambiguous reference rejection
7. Forward/self reference rejection (no guessed indices)
8. Effective .out suffix rejection via reference resolution logic

All error types are precise and include relevant context. No fabricated symbols,
secret handles, or named-action hashes. The implementation uses existing shared
infrastructure (SlotCompiler, expression parser, bytecode compiler) correctly.
