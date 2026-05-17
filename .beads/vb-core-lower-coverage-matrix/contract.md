# Contract Specification — vb-core-lower-coverage-matrix

## Context
- **Feature**: v1 YAML construct coverage parity matrix
- **Goal**: Prove every v1 YAML construct is accepted/rejected consistently across vb_yaml, vb_validate, and vb_compile
- **Excludes**: codegen/generated mode

## Domain Terms
- **vb_yaml**: Cold-path YAML parser and profile enforcement crate
- **vb_validate**: Cold-path workflow validation crate
- **vb_compile**: Cold-path YAML compiler boundary crate
- **v1 construct**: Any YAML field, primitive, or trigger defined in `velvet-ballastics/v1` schema
- **Parity**: Same input produces same accept/reject classification across all three crates

## Assumptions
1. The v1 schema is defined by the `WorkflowSource` AST in `vb_yaml/src/ast/types.rs`
2. `vb_compile` and `vb_validate` share reference validation via `vb_validate::references::validate_single_reference`
3. Unsupported primitives (save, do, choose) are intentionally excluded from compilation
4. Top-level declarations (inputs, result) are intentionally rejected at compile time

## Open Questions
1. Are `vars` declarations validated by vb_validate?
2. Are `secrets` declarations validated by vb_validate?
3. Are `examples` validated or silently ignored?
4. Is the `with` connector field validated?
5. Is the `then` next-step label validated?

---

## Preconditions

### PRE-001: Valid v1 YAML Source
Every input to the parity matrix must be a valid `WorkflowSource` AST parseable by `vb_yaml::parse_workflow_source`.

### PRE-002: Non-Empty Steps
The `steps` array must contain at least one step for compilation to proceed.

---

## Postconditions

### POST-001: Construct Classification Parity
For every v1 construct C and input YAML Y containing C:
- If vb_yaml accepts Y (no profile error), then vb_validate and vb_compile classification must match vb_yaml's acceptance
- If vb_yaml rejects Y (profile error), then vb_validate and vb_compile must also reject Y

### POST-002: Primitive Shape Invariants
For each supported primitive P in {for_each, together, collect, reduce, repeat, wait, ask}:
- `compile_workflow(P)` produces exactly the node kind sequence and slot count defined in `v1_primitive_lowering.rs::PRIMITIVE_CASES`

### POST-003: Unsupported Primitive Rejection
For each unsupported primitive U in {save, do, choose}:
- `compile_workflow(U)` returns `CompileError::UnsupportedStepPrimitive { primitive: U }`

### POST-004: Top-Level Rejection Parity
For each unsupported top-level declaration D in {inputs, result}:
- vb_yaml accepts D (parses as AST)
- vb_compile rejects D with `CompileError::UnsupportedTopLevelDeclaration` or `CompileError::UnsupportedTopLevelResult`

---

## Invariants

### INV-001: Node ID Density
All compiled workflows have dense, zero-indexed node IDs: `node[i].id == i` for all valid i.

### INV-002: Slot Reference Bounds
All slot references in compiled nodes are strictly less than `slot_count`.

### INV-003: Target Range
All step targets (next, body, done, join, etc.) reference nodes within the compiled workflow's node count.

### INV-004: Primitive Shape Determinism
Equal YAML source compiles to byte-identical `WorkflowDigest`.

---

## Error Taxonomy

### YAML Profile Errors (vb_yaml)
- `DuplicateKey` - duplicate mapping key
- `ForbiddenFeature` - anchor, alias, merge key, custom tag, binary scalar
- `MultipleDocuments` - more than one YAML document
- `AmbiguousBoolean` - YAML 1.1 yes/no/on/off

### Compile Errors (vb_compile)
- `EmptySteps` - steps array is empty
- `UnsupportedTopLevelDeclaration` - inputs, vars, secrets, examples
- `UnsupportedTopLevelResult` - result declaration
- `UnsupportedStepControlField` - step name field
- `DuplicateStepId` - duplicate step IDs
- `DuplicateOutputName` - duplicate output variable names
- `UnknownOutputName` - reference to undefined output
- `StepFieldShape` - invalid field value shape
- `SlotIndexOutOfRange` - slot index exceeds u16::MAX
- `UnsupportedStepPrimitive` - save, do, choose primitives
- `CanonicalYaml` - YAML profile error propagated
- `PrimitiveLoweringLimitExceeded` - primitive parameter exceeds limit

---

## Contract Signatures

```rust
// vb_yaml public API
fn parse_yaml_events(text: &str) -> YamlResult<Vec<YamlEvent>>;
fn parse_workflow_source(text: &str) -> YamlResult<WorkflowSource>;
fn validate_yaml_profile(text: &str) -> YamlResult<()>;

// vb_compile public API
fn compile_source(source: &WorkflowSource) -> CompileResult<CompiledWorkflow>;
fn compile_workflow(yaml: &[u8]) -> CompileResult<CompiledWorkflow>;
struct YamlCompiler { fn compile(self, yaml: &[u8]) -> CompileResult<CompiledWorkflow>; }

// vb_validate public API
fn validate_single_reference(...) -> ValidationResult<()>;
```

---

## Verus-Owned Clauses

### INV-001, INV-002, INV-003
Proven by existing `verification/verus/v1_primitive_lowering.rs`:
- `proof_construct_plan_valid`
- `proof_lowering_plan_targets_in_range`
- `proof_lowering_plan_slot_count_covers_references`
- `proof_lowering_plan_checks_bounds_before_casts`

---

## TLA+-Owned Clauses

This bead is NOT about temporal/workflow behavior. It is about static acceptance/rejection parity across parsing, validation, and compilation. No TLA+ model is required.

**Non-applicability rationale**: The coverage matrix proves compiler behavior on discrete inputs, not event-driven state transitions. TLA+ is inappropriate.

---

## Non-Goals
- Runtime behavior
- Codegen mode
- UI mode
- Concurrent/parallel execution paths
- Storage/persistence
- Network I/O
