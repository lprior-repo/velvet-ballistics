# Workflow Model: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## States

```
[RawSource] --(parse)--> [ParsedSource] --(validate)--> [ValidatedAst]
    |                        |                           |
    |                        |                           |
    v                        v                           v
[CompileFailed]        [CompileFailed]            [CompileFailed]
                            |
                            v
                    [LoweredParts] --(try_from_parts)--> [CompiledWorkflow]
                            |                                   |
                            |                                   |
                            v                                   v
                    [CompileFailed]                    [ReadyForRuntime]
                                                            |
                                                            v
                                                    [SerializedArtifact]
                                                            |
                                                            v
                                                    [DeserializedParts]
                                                            |
                                                            v
                                                    [CompiledWorkflow]
                                                            |
                                                            v
                                                      [Admitted]
```

## State Definitions

- **RawSource**: Unvalidated `&[u8]` YAML bytes.
- **ParsedSource**: `WorkflowSource` from `vb_yaml::parse_workflow_source`.
- **ValidatedAst**: `ast::WorkflowAst` passing schema, reference, control-flow, and taint checks.
- **LoweredParts**: `WorkflowParts` emitted by `SlotCompiler` in `mod_compile_lowering::compile_source`.
- **CompiledWorkflow**: Validated `CompiledWorkflow` from `try_from_parts`.
- **ReadyForRuntime**: Validated artifact ready for admission and execution.
- **SerializedArtifact**: Postcard bytes from `emit_compiled_artifact`.
- **DeserializedParts**: `WorkflowParts` from `postcard::from_bytes`.
- **Admitted**: Workflow accepted by runtime admission gate.

## Guards

| Transition | Guard |
|------------|-------|
| RawSource → ParsedSource | UTF-8, YAML parse, single document, strict profile limits. |
| ParsedSource → ValidatedAst | Schema shape, no duplicate keys, valid references, acyclic data flow, valid control flow. |
| ValidatedAst → LoweredParts | Lowering must not overflow step/slot/expression indices. |
| LoweredParts → CompiledWorkflow | **`try_from_parts` must succeed** — all structural invariants hold. |
| CompiledWorkflow → SerializedArtifact | Postcard serialization must succeed. |
| SerializedArtifact → DeserializedParts | Postcard deserialization must succeed. |
| DeserializedParts → CompiledWorkflow | **`try_from_parts` must succeed** — re-validation after storage round-trip. |
| CompiledWorkflow → Admitted | Runtime admission checks resource contract against deployment policy. |

## Terminal States

- **CompileFailed**: Non-empty `CompileErrors`; pipeline halts.
- **Admitted**: Workflow is running or queued for execution.

## Hazardous Transition (Pre-Fix)

- **LoweredParts → CompiledWorkflow via `from_parts_unchecked`**:
  - Skips all structural validation.
  - A compiler bug producing out-of-bounds indices or unreachable nodes would emit a corrupt `CompiledWorkflow`.
  - Runtime would encounter the corruption during execution (panic or undefined behavior in unsafe-free code = logic error / data corruption).

## Fixed Transition (Post-Fix)

- **LoweredParts → CompiledWorkflow via `try_from_parts`**:
  - Maps `WorkflowError` to `CompileError::Workflow`.
  - Surfaces structural bugs as compile-time diagnostics instead of runtime failures.
