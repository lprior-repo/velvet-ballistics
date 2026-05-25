# Contract: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Scope

This bead changes exactly one emission site in the compiler lowering pipeline and one dependency declaration:

1. **`crates/vb_compile/src/mod_compile_lowering/part_01.rs` line 57**: Replace `Ok(CompiledWorkflow::from_parts_unchecked(parts))` with `Ok(CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![CompileError::Workflow(e)]))?)`.
2. **`crates/vb_compile/Cargo.toml` line 13**: Remove `features = ["test-util"]` from the `vb_core` dependency.

## Preconditions

- `CompiledWorkflow::try_from_parts(parts)` exists in `vb_core` and validates all structural invariants.
- `CompileError::Workflow(#[from] WorkflowError)` exists and provides the error mapping.
- `vb_compile` does not need `test-util` for production code (only tests/benches might).

## Postconditions

- All compiler-emitted `CompiledWorkflow` instances have passed `validate_parts` and `validate_budget`.
- `from_parts_unchecked` is not reachable from `vb_compile` production code.
- The public API (`compile_workflow`, `compile_workflow_with_contracts`, `YamlCompiler::compile`) continues to return `Result<CompiledWorkflow, CompileErrors>`.

## Invariants Preserved

- `CompiledWorkflow` structural invariants are enforced at compile emission time.
- `WorkflowParts` remains the untrusted interchange format.
- `test-util` feature remains available for test crates and benchmark code.

## Invariants Introduced

- **Emission validation invariant**: Every `CompiledWorkflow` produced by `compile_source` has been validated by `try_from_parts`.

## API Compatibility

- No public API signature changes.
- Error variants: `CompileErrors` may now contain `CompileError::Workflow(...)` variants that were previously unreachable. This is a compatible expansion of possible error outputs.

## Verification Targets

- Proof seeds emitted in `proof-seeds.jsonl`.
- Traceability in `traceability-matrix.jsonl`.
