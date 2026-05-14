# Codebase Map: vb-qi37.8 — Shared Validation Pipeline

## Bead Overview
- **Bead ID**: vb-qi37.8
- **Title**: validate/compile: Prove and complete shared validation pipeline
- **State**: 2 (Explore + Scope)
- **Release Plan**: YES (critical bead)

## Architecture

### Shared Validation Pipeline Location
- **Primary Module**: `crates/vb_validate/src/shared.rs`
- **Gate Implementations**: `crates/vb_validate/src/gates.rs`
- **Public API Entry Points**:
  - `vb_validate::shared::validate(parts: &WorkflowParts)`
  - `vb_validate::shared::validate_with_contracts(parts: &WorkflowParts, action_contracts: &[ActionContract])`
  - `ValidationPipeline` struct with configurable gate enables/disables

### Validation Pipeline Architecture

```
WorkflowParts (untrusted IR from compiler boundary)
    │
    ▼
vb_validate::shared::validate()
    │
    ├── Gate 7: validate_gate_07_expression_stack_depth
    │   └── Bounds expression stack depth (max 64)
    │
    ├── Gate 8: validate_gate_08_accessor_path_segments
    │   └── Validates accessor paths resolve to valid symbols
    │
    ├── Gate 9: validate_gate_09_slot_references
    │   └── All slot references within declared slot_count
    │
    ├── Gate 10: validate_gate_10_node_kind_specific
    │   └── Node-kind-specific structural constraints
    │
    ├── Gate 11: validate_gate_11_loop_body_graph
    │   └── ForEach/Together body subgraphs well-formed
    │
    ├── Gate 13: validate_gate_13_no_slot_cycles
    │   └── No circular slot dependencies
    │
    ├── Gate 14: validate_gate_14_slot_type_consistency
    │   └── Slots with multiple writers have compatible types
    │
    ├── Gate 15: validate_gate_15_determinism_proof
    │   └── Non-deterministic nodes properly separated by suspension points
    │
    └── Gate 12: validate_gate_12_action_contract_completeness (requires contracts)
        └── Bijection between Do nodes and ActionContracts
```

### Crates Touched

| Crate | Role |
|-------|------|
| `crates/vb_validate` | Core validation pipeline implementation |
| `crates/vb_compile` | Calls shared validation at compile boundary |
| `crates/vb_core` | Provides WorkflowParts, CompiledNode, ActionContract types |
| `crates/velvet_ballastics` | CLI commands that invoke validation |
| `fuzz` | Fuzz harnesses that exercise the validation pipeline |

### Public APIs

#### vb_validate::shared
```rust
// Entry points
pub fn validate(parts: &WorkflowParts) -> ValidationResult<()>
pub fn validate_with_contracts(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> ValidationResult<()>

// Configuration
pub struct ValidationPipeline {
    pub gate_07_expression_stack: bool,
    pub gate_08_accessor_paths: bool,
    pub gate_09_slot_references: bool,
    pub gate_10_node_kind_specific: bool,
    pub gate_11_loop_body_graph: bool,
    pub gate_12_action_contracts: bool,
    pub gate_13_no_slot_cycles: bool,
    pub gate_14_slot_type_consistency: bool,
    pub gate_15_determinism_proof: bool,
}

impl ValidationPipeline {
    pub const fn all_gates() -> Self
    pub const fn no_gates() -> Self
    pub fn validate(&self, parts: &WorkflowParts) -> ValidationResult<()>
    pub fn validate_with_contracts(&self, parts: &WorkflowParts, action_contracts: &[ActionContract]) -> ValidationResult<()>
}
```

#### Individual Gate Exports
```rust
pub use gates::validate_gate_07_expression_stack_depth;
pub use gates::validate_gate_08_accessor_path_segments;
pub use gates::validate_gate_09_slot_references;
pub use gates::validate_gate_10_node_kind_specific;
pub use gates::validate_gate_11_loop_body_graph;
pub use gates::validate_gate_12_action_contract_completeness;
pub use gates::validate_gate_13_no_slot_cycles;
pub use gates::validate_gate_14_slot_type_consistency;
pub use gates::validate_gate_15_determinism_proof;
```

### Key Types (from vb_core)

#### WorkflowParts
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

#### CompiledNode
```rust
pub struct CompiledNode {
    pub id: StepIdx,
    pub output: Option<SlotIdx>,
    pub next: Option<StepIdx>,
    pub on_error: Option<StepIdx>,
    pub error_slot: Option<SlotIdx>,
    pub kind: CompiledNodeKind,
}
```

#### CompiledNodeKind Variants
- Nop, SetConst, Copy, EvalExpr, BuildObject, BuildList
- Do, Choose, ChooseSlot
- ForEachStart, ForEachNext, ForEachJoin
- TogetherStart, TogetherBranch, TogetherJoin
- CollectStart, CollectPage, CollectNext, CollectFinish
- ReduceStart, ReduceNext, ReduceFinish
- RepeatStart, RepeatAttempt, RepeatCheck, RepeatFinish
- WaitUntil, WaitEvent, Ask, AskResume
- RetryCheck, ErrorHandler, Jump, Finish

### Call Sites

| File | Usage |
|------|-------|
| `crates/vb_compile/src/compile.rs:30` | `validate_with_contracts(&parts, contracts)` |
| `crates/vb_compile/src/api_compilation.rs:51` | `validate_with_contracts(&parts, contracts)` |
| `crates/vb_compile/src/schema.rs:651` | `validate(&parts)` |
| `crates/vb_compile/src/types.rs:155` | `validate(&parts)` |
| `crates/vb_compile/src/lib.rs:163,221,280` | `validate` and `validate_with_contracts` |
| `crates/velvet_ballastics/src/commands_verify.rs:76` | `validate(&parts)` |
| `fuzz/src/lib.rs:40,60` | `validate_with_contracts` for fuzzing |

### Validation Error Variants (37 total)
- DUPLICATE_KEY, FORBIDDEN_YAML_FEATURE, UNKNOWN_TOP_LEVEL_FIELD
- UNKNOWN_STEP_FIELD, MISSING_REQUIRED_FIELD, INVALID_VERSION
- INVALID_ID, RESERVED_ID, DUPLICATE_ID
- MULTIPLE_STEP_PRIMITIVES, MISSING_STEP_PRIMITIVE
- UNKNOWN_REFERENCE, FUTURE_REFERENCE, SECRET_NOT_DECLARED
- DIRECT_RUNTIME_REFERENCE, INVALID_THEN_TARGET, CONTROL_FLOW_CYCLE
- UNREACHABLE_STEP, INVALID_CHOOSE, INVALID_FOR_EACH
- INVALID_TOGETHER, INVALID_COLLECT, INVALID_REDUCE, INVALID_REPEAT
- INVALID_WAIT, INVALID_ASK, INVALID_FINISH, INVALID_RETRY, INVALID_ON_ERROR
- SECRET_RESULT_LEAK, TYPE_MISMATCH, PAYLOAD_TOO_LARGE
- LIMIT_REQUIRED, LIMIT_EXCEEDED, UNSUPPORTED_TRIGGER, HTTP_TRIGGER_OUT_OF_CORE
- Expression stack/accessor/slot/loop/determinism specific errors (Gates 7-15)

### Tests

| File | Coverage |
|------|----------|
| `crates/vb_validate/tests/red_phase_validation.rs` | Gate 7, 8, 9 tests |
| `crates/vb_validate/tests/capability_contract_schema.rs` | Gate 12 action contracts |
| `crates/vb_validate/tests/idempotency_contract_red.rs` | Idempotency contract validation |
| `crates/vb_validate/tests/gate_08_accessor_parity.rs` | Accessor gate parity |
| `crates/vb_validate/benches/capability_schema.rs` | Benchmark validation |

### Dependencies

```
vb_validate/Cargo.toml:
  - vb_core (path = "../vb_core")
  - thiserror (workspace)

vb_compile/Cargo.toml:
  - vb_validate (path = "../vb_validate")
  - vb_core (path = "../vb_core")
  - vb_codegen (path = "../vb_codegen")
```

### Risk Tags
- `DRIFT-5`: Validation deduplication between vb_compile and vb_validate
- `SECTION-63`: Plan verifier gates for compiled workflow IR
- `SECTION-16`: Master contract error codes
- `COLD-PATH`: Validation runs only at compile time, not runtime

### Required Verifier Modes

| Gate | Verifier | Mode |
|------|----------|------|
| Gate 7-15 | Miri | For detecting UB in pointer handling |
| Gate 7-15 | Kani | For bounded model checking of slot references |
| Gate 12 | Proptest | For contract completeness checking |
| Gate 15 | TLA+ | For determinism proof temporal properties |

### Related Beads
- vb-qi37.4.3/4.4: Earlier validation pipeline work
- vb-qi37.16.2/16.5: Lifecycle journal storage contracts
- vb-qi37.7.3: Kani validation proofs