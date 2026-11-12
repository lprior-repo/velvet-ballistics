# Domain Model: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Ubiquitous Language

| Term | Definition |
|------|-----------|
| **Workflow Source** | Cold YAML authoring AST (`vb_yaml::ast::WorkflowSource`) produced by the parser. |
| **Compiled Workflow** | Immutable, structurally validated runtime IR (`CompiledWorkflow`) consumed by the runtime engine. |
| **Workflow Parts** | Untrusted bag-of-parts (`WorkflowParts`) emitted by the compiler lowering phase before validation. |
| **Compiler Emission** | The act of producing a `CompiledWorkflow` from `WorkflowParts` at the end of the lowering pipeline. |
| **Structural Validation** | Invariant checks on numeric references, reachability, loop nesting, edge direction, and resource contracts. |
| **Unchecked Construction** | Building a `CompiledWorkflow` directly from parts without validation (test-only). |
| **Checked Construction** | Building a `CompiledWorkflow` via `try_from_parts`, which runs `validate_parts` and `validate_budget`. |

## Entities

### CompiledWorkflow (Aggregate Root)
- Identity: Digest (`WorkflowDigest`) derived from source YAML.
- Invariants: All numeric indices are in-bounds; all nodes reachable from entry; no backward edges except loop back-edges; loops properly nested; resource contract covers actual usage.
- Lifecycle: Created only through checked construction or deserialized from a previously validated artifact.

### WorkflowParts (Value Object)
- Raw material for `CompiledWorkflow`.
- **Untrusted by default**: any compiler bug or corrupted deserialization can violate invariants.
- Must pass through `try_from_parts` before crossing into the runtime boundary.

### YamlCompiler (Entity / Facade)
- Orchestrates parsing → AST validation → lowering → emission.
- Public entry points: `compile()`, `compile_workflow()`, `compile_workflow_with_contracts()`.

## Value Objects

- **StepIdx**, **SlotIdx**, **ConstIdx**, **ExprIdx**, **AccessorIdx**: Newtyped `u16`/`u32` indices with checked construction.
- **WorkflowDigest**: Blake3 hash of canonical source.
- **ResourceContract**: Bounded admission policy carried with the compiled artifact.

## Commands

1. **CompileWorkflow** — `YamlCompiler::compile(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors>`
2. **CompileWithContracts** — `compile_workflow_with_contracts(source, contracts)`
3. **LowerSource** — `compile_source(source: &WorkflowSource) -> Result<CompiledWorkflow, CompileErrors>` (internal)

## Events

- `WorkflowCompiled` — successful emission of validated `CompiledWorkflow`.
- `CompilationFailed` — one or more `CompileError`s collected during pipeline.
- `WorkflowValidationFailed` — `WorkflowError` surfaced from `try_from_parts` during emission.

## Policies

- **No Unchecked Emission**: Production compiler code must never call `from_parts_unchecked`.
- **Test-Util Feature Firewall**: `from_parts_unchecked` is gated behind `#[cfg(feature = "test-util")]` and must not be reachable from production dependency graphs.
- **Compiler Bugs Are Compile Errors**: Any structurally invalid IR produced by the compiler must be caught at emission and reported as `CompileError::Workflow(...)`.
