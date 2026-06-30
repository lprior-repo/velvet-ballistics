# Boundary Map: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Layer Architecture

```
┌─────────────────────────────────────────┐
│  Shell: CLI / Server / Test Harness     │
│  (I/O, async, network, time)            │
├─────────────────────────────────────────┤
│  Imperative Shell: vb_compile facade    │
│  - YamlCompiler::compile()              │
│  - compile_workflow()                   │
│  - compile_workflow_with_contracts()    │
├─────────────────────────────────────────┤
│  Functional Core: vb_compile lowering   │
│  - mod_compile_lowering::compile_source │
│  - SlotCompiler (builds WorkflowParts)  │
├─────────────────────────────────────────┤
│  PURE VALIDATION BOUNDARY (THIS BEAD)   │
│  - CompiledWorkflow::try_from_parts     │
│  - validate_parts, validate_budget      │
├─────────────────────────────────────────┤
│  Runtime Core: vb_runtime / vb_engine   │
│  - Admission, execution, scheduling     │
├─────────────────────────────────────────┤
│  Storage Shell: vb_storage              │
│  - Serialize/deserialize WorkflowParts  │
│  - Fjall persistence, recovery          │
└─────────────────────────────────────────┘
```

## Boundary Definitions

### 1. Parser Boundary (YAML → AST)
- **Location**: `vb_yaml::parse_workflow_source`, `strict_yaml`
- **Input**: `&[u8]` raw YAML
- **Output**: `WorkflowSource`
- **Pure?** No — depends on `saphyr` parser (external dependency).
- **Error type**: `CompileError` (parse, profile, schema)

### 2. AST Validation Boundary (AST → Validated AST)
- **Location**: `references::validate_workflow_ast`, `type_taint::validate_workflow_ast`, `control_flow::validate_workflow_ast`
- **Input**: `WorkflowSource` / `WorkflowAst`
- **Output**: Validated AST
- **Pure?** Yes — deterministic, no I/O.
- **Error type**: `CompileError` (reference, taint, control flow)

### 3. Lowering Boundary (AST → WorkflowParts)
- **Location**: `mod_compile_lowering::compile_source`
- **Input**: `&WorkflowSource`
- **Output**: `WorkflowParts`
- **Pure?** Yes — deterministic translation.
- **Error type**: `CompileErrors` (lowering limits, index overflow)
- **PRE-FIX BUG**: Emission called `from_parts_unchecked`, bypassing the validation boundary.

### 4. Validation Boundary (WorkflowParts → CompiledWorkflow) — **CRITICAL**
- **Location**: `vb_core::CompiledWorkflow::try_from_parts`
- **Input**: `WorkflowParts` (untrusted)
- **Output**: `Result<CompiledWorkflow, WorkflowError>`
- **Pure?** Yes — fully deterministic, no I/O, no allocation failure paths.
- **Error type**: `WorkflowError`
- **Post-fix**: `compile_source` calls `try_from_parts` and maps `WorkflowError` → `CompileError::Workflow`.

### 5. Runtime Boundary (CompiledWorkflow → Execution)
- **Location**: `vb_runtime` admission, `RunFrame` construction
- **Input**: `CompiledWorkflow`
- **Output**: Admitted run or admission error
- **Pure?** No — I/O, timers, async, storage.
- **Error type**: `AdmissionError`, runtime `WorkflowError`

### 6. Storage Boundary (CompiledWorkflow → Bytes → CompiledWorkflow)
- **Location**: `emit_compiled_artifact`, `vb_storage` deserialization
- **Input**: `CompiledWorkflow` / postcard bytes
- **Output**: Postcard bytes / `WorkflowParts`
- **Pure?** Serialization is pure; deserialization + `try_from_parts` is the validation gate.

## Feature-Gate Boundary

- `test-util` feature on `vb_core` is a **trust boundary**.
- `from_parts_unchecked` lives behind this gate.
- **Policy**: Only test crates and benchmark code may depend on `vb_core/test-util`.
- **Violation**: `vb_compile/Cargo.toml` currently enables `test-util` in `[dependencies]`, making `from_parts_unchecked` available in production builds.

## Data Flow

```
[YAML bytes]
    │
    ▼
[vb_yaml parser] ───────┐
    │                    │
    ▼                    │
[WorkflowSource]         │
    │                    │
    ▼                    │
[AST validators]         │  (pure)
    │                    │
    ▼                    │
[WorkflowAst]            │
    │                    │
    ▼                    │
[Lowering: compile_source]
    │                    │  (pure)
    ▼                    │
[WorkflowParts]          │
    │                    │
    ▼                    │
[try_from_parts] ◄───────┘  (pure validation boundary)
    │
    ▼
[CompiledWorkflow]
    │
    ├──► [vb_runtime]    (imperative shell)
    │
    └──► [postcard] ──► [storage] ──► [deser + try_from_parts] ──► [runtime]
```
