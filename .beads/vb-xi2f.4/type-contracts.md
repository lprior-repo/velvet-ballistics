# Type Contracts: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Core Types

### `CompiledWorkflow`
```rust
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    nodes: Box<[CompiledNode]>,
    expressions: Box<[ExprProgram]>,
    accessors: Box<[AccessorProgram]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
    symbols_count: u32,
    entry: StepIdx,
    resource_contract: ResourceContract,
    step_names: Box<[Box<str>]>,
}
```
- **Smart constructor**: `try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError>`
- **Illegal-state prevention**: All fields are private; no public mutation.
- **Test-only bypass**: `from_parts_unchecked` is `#[cfg(feature = "test-util")]`.

### `WorkflowParts`
```rust
pub struct WorkflowParts {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub nodes: Box<[CompiledNode]>,
    pub expressions: Box<[ExprProgram]>,
    pub accessors: Box<[AccessorProgram]>,
    pub constants: Box<[ConstValue]>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Box<[Box<str>]>,
}
```
- **Public fields intentional**: this is the untrusted interchange format.
- **Must not be accepted by runtime directly**: always bridged through `try_from_parts`.

### `CompileErrors`
```rust
pub struct CompileErrors(pub Vec<CompileError>);
```
- **Railway error**: accumulates diagnostics across pipeline stages.
- **Must absorb `WorkflowError`**: `CompileError::Workflow(#[from] WorkflowError)`.

## Typestates

### Compiler Pipeline State Machine
1. **RawBytes** — untrusted `&[u8]` YAML source.
2. **Parsed** — `WorkflowSource` (syntactically valid YAML, schema-checked).
3. **ValidatedAst** — `ast::WorkflowAst` (referentially valid, type-taint clean).
4. **LoweredParts** — `WorkflowParts` (numeric IR, still unvalidated).
5. **Compiled** — `CompiledWorkflow` (structurally validated, ready for runtime).

**Transitions**:
- RawBytes → Parsed: fallible (UTF-8, YAML parse, strict-profile limits).
- Parsed → ValidatedAst: fallible (schema, references, control flow, type taint).
- ValidatedAst → LoweredParts: fallible (lowering limits, index overflow).
- LoweredParts → Compiled: **must use `try_from_parts`** (previously broken via `from_parts_unchecked`).

## Invariant Summary

| Invariant | Enforced By | Failure Mode |
|-----------|-------------|--------------|
| `nodes` non-empty | `validate_parts` | `WorkflowError::EmptyNodes` |
| `entry` in bounds | `validate_entry` | `WorkflowError::EntryOutOfBounds` |
| Node id matches index | `validate_node_id` | `WorkflowError::NodeIdMismatch` |
| Slot refs in bounds | `validate_slot` | `WorkflowError::SlotOutOfBounds` |
| Step refs in bounds | `validate_step` | `WorkflowError::StepOutOfBounds` |
| Const refs in bounds | `validate_const` | `WorkflowError::ConstOutOfBounds` |
| Expr refs in bounds | `validate_expr` | `WorkflowError::Expression(...)` |
| Accessor refs in bounds | `validate_accessor` | `WorkflowError::Expression(...)` |
| Branch tables non-empty | `validate_branch_route` | `WorkflowError::EmptyBranchTable` |
| All nodes reachable | `validate_reachability` | `WorkflowError::UnreachableNode` |
| Forward edges only | `validate_forward_edges` | `WorkflowError::BackwardEdge` |
| Proper loop nesting | `validate_forward_edges` | `WorkflowError::ImproperLoopNesting` |
| Resource contract covers usage | `validate_resource_contract` | `WorkflowError::ResourceContractExceeded` |
| Whole-workflow budget ok | `validate_budget` | `WorkflowError::BudgetPolicyExceeded` |
| Accessor path depth bounded | `validate_accessor_paths` | `WorkflowError::AccessorPathTooDeep` |
| Symbol ids in bounds | `validate_symbol` | `WorkflowError::SymbolOutOfBounds` |

## Parser-at-Boundary

- `WorkflowParts` is deserialized from postcard bytes at storage/runtime boundaries.
- Deserialized parts **must** pass through `try_from_parts` before use.
- No `CompiledWorkflow` deserialization exists; this is intentional (force validation).

## Feature-Gate Contract

- `test-util` on `vb_core` exposes `from_parts_unchecked`.
- `vb_compile` **must not** enable `test-util` in `[dependencies]` (only `[dev-dependencies]` if needed).
- After this bead, `vb_compile/Cargo.toml` removes `features = ["test-util"]` from `vb_core` dependency.
