# Contract: vb-qi37.8 — Shared Validation Pipeline

## Bead Information
- **bead_id**: vb-qi37.8
- **title**: validate/compile: Prove and complete shared validation pipeline
- **state**: 3 (Contract + Proof Obligations)
- **dispatch_manifest**: delegate_agent=rust-contract, isolated_workdir=/home/lewis/src/vb-qi37-ws

---

## 1. Requirements

### 1.1 Core Validation Pipeline

| Req ID | Requirement | Source | Priority |
|--------|-------------|--------|----------|
| R1 | `validate(parts: &WorkflowParts) -> ValidationResult<()>` must accept untrusted IR and return `ValidationResult` | delivery-scope | MUST |
| R2 | `validate_with_contracts(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> ValidationResult<()>` must perform Gate 12 completeness check | delivery-scope | MUST |
| R3 | `ValidationPipeline` struct must support configurable gate enable/disable | delivery-scope | MUST |
| R4 | All 9 gates (7-15) must be exported individually via `pub use gates::*` | codebase-map.md | MUST |

### 1.2 Gate Requirements

| Gate | Req ID | Requirement | Bounded |
|------|--------|-------------|---------|
| G7 | R7-1 | Expression stack depth ≤ 64 | Yes (max 64) |
| G8 | R8-1 | Accessor path segments resolve to valid symbols | Yes (symbols_count bound) |
| G9 | R9-1 | All slot references within `slot_count` bounds | Yes (u16 bound) |
| G10 | R10-1 | Node-kind-specific structural constraints satisfied | Yes (14 node variants) |
| G11 | R11-1 | ForEach/Together body subgraphs well-formed | Yes (finite unroll) |
| G12 | R12-1 | Bijection between Do nodes and ActionContracts | Yes (action_contracts.len()) |
| G13 | R13-1 | No circular slot dependencies | Yes (slot_count iterations) |
| G14 | R14-1 | Multi-writer slots have compatible types | Yes (type finite) |
| G15 | R15-1 | Non-deterministic nodes separated by suspension points | Yes (finite graph) |

### 1.3 Integration Requirements

| Req ID | Requirement | Call Site |
|--------|-------------|-----------|
| R16 | `vb_compile::compile.rs:30` calls `validate_with_contracts` | compile.rs:30 |
| R17 | `vb_compile::api_compilation.rs:51` calls `validate_with_contracts` | api_compilation.rs:51 |
| R18 | `vb_compile::schema.rs:651` calls `validate` | schema.rs:651 |
| R19 | `vb_compile::types.rs:155` calls `validate` | types.rs:155 |
| R20 | `velvet_ballistics::commands_verify.rs:76` calls `validate` | commands_verify.rs:76 |
| R21 | `fuzz::lib.rs:40,60` calls `validate_with_contracts` | fuzz/lib.rs |

### 1.4 Error Handling Requirements

| Req ID | Requirement |
|--------|-------------|
| R22 | Validation must return `ValidationError` variants (37 total) |
| R23 | Validation must be fallible (no unwrap/expect in pipeline) |
| R24 | Validation must not panic on malformed input |

---

## 2. Assumptions

### 2.1 Input Assumptions

| A ID | Assumption | Rationale |
|------|-----------|-----------|
| A1 | `WorkflowParts` is well-formed at type level | Borrowed from vb_core |
| A2 | `WorkflowParts.digest` is pre-computed by caller | Compiler computes before validation |
| A3 | `CompiledNode` variants are exhaustive (14 kinds) | Enum in vb_core |
| A4 | `slot_count` fits in `u16` | Workspace constraint |
| A5 | `symbols_count` fits in `u32` | Workspace constraint |

### 2.2 Environmental Assumptions

| A ID | Assumption | Rationale |
|------|-----------|-----------|
| A6 | No concurrent modification of `WorkflowParts` during validation | Caller ensures single-threaded |
| A7 | Validation runs at compile time, not runtime | Cold path per codebase-map |
| A8 | Kani bounded model checking uses slot_count as bound | Enables finite verification |

---

## 3. Invariants

### 3.1 Validation Pipeline Invariants

```
INV-1: ValidationResult is either Ok(()) or Error(ValidationError)
INV-2: validate() and validate_with_contracts() are pure (no side effects)
INV-3: ValidationPipeline::all_gates() enables all 9 gates
INV-4: ValidationPipeline::no_gates() disables all gates
INV-5: Gate ordering is deterministic (G7→G8→...→G15)
```

### 3.2 Structural Invariants

```
INV-6: For any CompiledNode, output SlotIdx < slot_count when Some
INV-7: For any CompiledNode, next StepIdx < nodes.len()
INV-8: For any CompiledNode, on_error StepIdx < nodes.len() when Some
INV-9: entry StepIdx < nodes.len()
INV-10: ForEachStart/ TogetherStart have matching Join nodes
INV-11: ReduceStart has matching ReduceFinish
INV-12: CollectStart has matching CollectFinish
```

### 3.3 Resource Contract Invariants

```
INV-13: slot_count > 0 implies at least one slot declared
INV-14: symbols_count > 0 implies at least one symbol declared
INV-15: step_names.len() == nodes.len()
```

---

## 4. Contract Clauses

### 4.1 Preconditions

```
PRE-1: parts.nodes.len() > 0
PRE-2: parts.entry < parts.nodes.len()
PRE-3: parts.slot_count > 0 implies parts.expressions.len() >= 0
PRE-4: parts.is_consistent() // internal consistency check
```

### 4.2 Postconditions

```
POST-1: validate() returns Ok(()) iff all enabled gates pass
POST-2: validate() returns Error(e) implies e.code ∈ VALIDATION_ERROR_CODES
POST-3: validate_with_contracts() returns Ok(()) iff G7-G11,G13-G15 pass AND G12 passes
POST-4: Returned ValidationError contains step_idx of failing node when applicable
POST-5: Validation is deterministic: same input → same output
```

### 4.3 Frame Conditions

```
FRAME-1: validate() does not modify parts
FRAME-2: validate() does not retain references to parts after return
FRAME-3: ValidationPipeline::validate() does not modify internal state
```

### 4.4 Aborts (Explicit Failure Modes)

```
ABORT-1: parts violates structural invariant → Error with relevant code
ABORT-2: Expression stack overflow (>64) → Error(G7)
ABORT-3: Accessor path resolution failure → Error(G8)
ABORT-4: Slot index out of bounds → Error(G9)
ABORT-5: Node-kind constraint violation → Error(G10)
ABORT-6: Loop body graph malformed → Error(G11)
ABORT-7: Action contract bijection fails → Error(G12)
ABORT-8: Slot cycle detected → Error(G13)
ABORT-9: Slot type incompatibility → Error(G14)
ABORT-10: Non-determinism without suspension → Error(G15)
```

---

## 5. Acceptance Criteria

| AC ID | Criterion | Verification Method |
|-------|-----------|---------------------|
| AC1 | All 9 gates compile and pass their unit tests | cargo test -p vb_validate |
| AC2 | validate() rejects malformed WorkflowParts | Unit test coverage |
| AC3 | validate_with_contracts() checks G12 bijection | Integration test |
| AC4 | ValidationPipeline::all_gates() enables all gates | Unit test |
| AC5 | ValidationPipeline::no_gates() disables all gates | Unit test |
| AC6 | Validation is deterministic | Property test (proptest) |
| AC7 | No panic on any input | Miri + Kani bounded checking |
| AC8 | Compilation succeeds with --all-features | cargo build --release |
| AC9 | Integration with vb_compile succeeds | cargo test -p vb_compile |
| AC10 | Fuzz harness exercises validate_with_contracts | cargo fuzz run |

---

## 6. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| DRIFT-5: Validation deduplication bypass | Low | High | Ensure vb_compile always calls vb_validate |
| SECTION-63: Gate implementation gaps | Medium | High | Kani bounded model checking |
| COLD-PATH: Validation not run at runtime | N/A | N/A | Design assumption |
