# Test Plan: vb-qi37.8 — Shared Validation Pipeline

## Summary

| Category | Count | Rationale |
|----------|-------|-----------|
| Behaviors identified | 47 | From contract + invariants |
| Unit tests | 38 | Gate logic, error variants, invariants |
| BDD scenarios | 62 | Public API behaviors, error variants |
| Proptest invariants | 8 | Determinism, purity, bijection properties |
| Fuzz targets | 2 | validate, validate_with_contracts |
| Kani harnesses | 20 | Bounded model checking for all gates |
| Miri checks | 14 | UB detection for all gates |
| Integration tests | 11 | Call sites R16-R21, 6 sites |
| TLA+/Lean proofs | 4 | Deferred G13/G15 temporal properties |
| Mutation checkpoints | 12 | Critical gate logic paths |

**Trophy Allocation**: ~35% unit, ~55% integration, ~5% e2e, ~5% static

---

## 1. Behavior Inventory

### Pipeline Behaviors

1. "ValidationPipeline accepts WorkflowParts and returns ValidationResult"
2. "ValidationPipeline::all_gates() enables all 9 gates"
3. "ValidationPipeline::no_gates() disables all gates"
4. "validate() does not modify input parts"
5. "validate() does not retain references after return"
6. "validate_with_contracts() performs G12 bijection check"
7. "Validation is deterministic: same input yields same output"
8. "validate() returns Ok(()) iff all enabled gates pass"
9. "validate_with_contracts() returns Ok(()) iff G7-G11,G13-G15 AND G12 pass"
10. "ValidationError contains step_idx of failing node when applicable"
11. "ValidationError code is in VALIDATION_ERROR_CODES"

### Gate G7 — Expression Stack Depth

12. "validate_gate_07 rejects expression stack depth > 64"
13. "validate_gate_07 accepts expression stack depth <= 64"
14. "validate_gate_07 computes stack depth without overflow"

### Gate G8 — Accessor Path Segments

15. "validate_gate_08 rejects unresolved accessor path segments"
16. "validate_gate_08 accepts resolved accessor path segments"
17. "validate_gate_08 performs symbol lookup without UB"

### Gate G9 — Slot References

18. "validate_gate_09 rejects output SlotIdx >= slot_count"
19. "validate_gate_09 rejects error_slot SlotIdx >= slot_count"
20. "validate_gate_09 accepts all slot references within bounds"
21. "validate_gate_09 rejects entry StepIdx >= nodes.len()"
22. "validate_gate_09 rejects next StepIdx >= nodes.len()"
23. "validate_gate_09 rejects on_error StepIdx >= nodes.len()"
24. "validate_gate_09 performs slot index operations without UB"

### Gate G10 — Node-Kind Structural Constraints

25. "validate_gate_10 accepts ForEachStart with matching ForEachJoin"
26. "validate_gate_10 rejects ForEachStart without matching ForEachJoin"
27. "validate_gate_10 accepts TogetherStart with matching TogetherJoin"
28. "validate_gate_10 rejects TogetherStart without matching TogetherJoin"
29. "validate_gate_10 accepts ReduceStart with matching ReduceFinish"
30. "validate_gate_10 rejects ReduceStart without matching ReduceFinish"
31. "validate_gate_10 accepts CollectStart with matching CollectFinish"
32. "validate_gate_10 rejects CollectStart without matching CollectFinish"
33. "validate_gate_10 performs node kind matching without UB"

### Gate G11 — Loop Body Graph

34. "validate_gate_11 accepts ForEach body subgraph leading to ForEachJoin"
35. "validate_gate_11 rejects ForEach body subgraph not leading to ForEachJoin"
36. "validate_gate_11 accepts Together body subgraph leading to TogetherJoin"
37. "validate_gate_11 rejects Together body subgraph not leading to TogetherJoin"
38. "validate_gate_11 performs graph traversal without UB"

### Gate G12 — Action Contract Bijection

39. "validate_gate_12 accepts when Do nodes surject onto ActionContracts"
40. "validate_gate_12 rejects when Do nodes do not cover all ActionContracts"
41. "validate_gate_12 rejects when ActionContracts do not cover all Do nodes"
42. "validate_gate_12 performs bijection check deterministically"

### Gate G13 — Slot Cycle Detection

43. "validate_gate_13 accepts slot graph with no cycles"
44. "validate_gate_13 rejects slot graph with circular dependencies"
45. "validate_gate_13 cycle detection terminates within slot_count iterations"
46. "validate_gate_13 performs cycle detection without UB"

### Gate G14 — Slot Type Compatibility

47. "validate_gate_14 accepts multi-writer slots with compatible types"
48. "validate_gate_14 rejects multi-writer slots with incompatible types"
49. "validate_gate_14 performs type checks without UB"

### Gate G15 — Non-determinism Separation

50. "validate_gate_15 accepts workflow with ND nodes separated by suspension"
51. "validate_gate_15 rejects workflow with adjacent ND nodes"
52. "validate_gate_15 performs graph search for suspension without UB"

### Error Handling

53. "All 37 ValidationError variants are constructible"
54. "Validation returns specific error codes, not generic errors"
55. "Validation does not panic on malformed input"
56. "Validation does not unwrap or expect in pipeline"

### Integration Call Sites

57. "vb_compile::compile.rs:30 calls validate_with_contracts"
58. "vb_compile::api_compilation.rs:51 calls validate_with_contracts"
59. "vb_compile::schema.rs:651 calls validate"
60. "vb_compile::types.rs:155 calls validate"
61. "velvet_ballistics::commands_verify.rs:76 calls validate"
62. "fuzz::lib.rs:40,60 calls validate_with_contracts"

---

## 2. Trophy Allocation

| Layer | Count | Tests | Rationale |
|-------|-------|-------|-----------|
| Static Analysis | 5 | clippy(no_unwrap), compile(gates exported), type_check | Catch entire classes of bugs at compile time |
| Unit / Calc | 38 | Gate logic tests, error variant tests, invariant tests | Pure functions, exhaustive boundary coverage |
| Integration | 11 | Call site tests, compilation tests, CLI verify | Real dependencies, component boundaries |
| E2E | 2 | Fuzz run, full compilation pipeline | End-to-end workflow validation |

**Rationale**: Validation pipeline is a pure computation layer. Unit tests dominate for gate logic. Integration tests verify call site correctness. Fuzz provides unbounded input coverage.

---

## 3. BDD Scenarios

### Behavior 1: validate() accepts WorkflowParts and returns ValidationResult

```
Scenario: validate accepts valid WorkflowParts
Given: a valid WorkflowParts with all nodes within bounds
When: validate(parts) is called
Then: returns Ok(()) with no modifications to parts

Scenario: validate rejects invalid WorkflowParts
Given: a WorkflowParts with slot_count = 0 but non-empty slot references
When: validate(parts) is called
Then: returns Err(ValidationError) with GATE_09 code and step_idx

Scenario: validate returns ValidationResult not Option
Given: any WorkflowParts input
When: validate(parts) is called
Then: return type is ValidationResult<()>, not Option<()>
```

### Behavior 6: validate_with_contracts() performs G12 bijection check

```
Scenario: validate_with_contracts accepts correct bijection
Given: WorkflowParts with N Do nodes and N ActionContracts with matching names
When: validate_with_contracts(parts, contracts) is called
Then: returns Ok(()) when all other gates pass

Scenario: validate_with_contracts rejects missing Do node
Given: WorkflowParts with Do node named "action_foo" but contracts has no "action_foo"
When: validate_with_contracts(parts, contracts) is called
Then: returns Err(ValidationError) with GATE_12 code

Scenario: validate_with_contracts rejects orphan ActionContract
Given: ActionContract named "action_bar" but no Do node with "action_bar"
When: validate_with_contracts(parts, contracts) is called
Then: returns Err(ValidationError) with GATE_12 code
```

### Behavior 7: Validation is deterministic

```
Scenario: validate is deterministic
Given: a valid WorkflowParts
When: validate(parts) is called twice
Then: both calls return identical Ok(()) results

Scenario: validate is deterministic on errors
Given: an invalid WorkflowParts that fails G9
When: validate(parts) is called twice
Then: both calls return Err with identical error code and step_idx
```

### Behavior 12: validate_gate_07 rejects expression stack depth > 64

```
Scenario: G7 rejects stack depth 65
Given: WorkflowParts with expression requiring depth 65
When: validate_gate_07_expression_stack_depth is called
Then: returns Err(ValidationError) with GATE_07 code

Scenario: G7 accepts stack depth 64
Given: WorkflowParts with expression requiring depth exactly 64
When: validate_gate_07_expression_stack_depth is called
Then: returns Ok(())

Scenario: G7 rejects stack depth overflow
Given: WorkflowParts with deeply nested expressions
When: stack depth computation would overflow usize
Then: returns Err(GATE_07) not panic
```

### Behavior 15: validate_gate_08 rejects unresolved accessor path segments

```
Scenario: G8 rejects unresolved symbol
Given: WorkflowParts with accessor path ["unknown_symbol", "field"]
When: validate_gate_08_accessor_path_segments is called
Then: returns Err(ValidationError) with GATE_08 code

Scenario: G8 accepts resolved symbols
Given: WorkflowParts with accessor paths resolving to declared symbols
When: validate_gate_08_accessor_path_segments is called
Then: returns Ok(())
```

### Behavior 18-23: validate_gate_09 slot reference bounds

```
Scenario: G9 rejects output SlotIdx out of bounds
Given: WorkflowParts with node.output = Some(slot_count)
When: validate_gate_09_slot_references is called
Then: returns Err(ValidationError) with GATE_09 code and node step_idx

Scenario: G9 accepts valid output SlotIdx
Given: WorkflowParts with node.output = Some(slot_count - 1)
When: validate_gate_09_slot_references is called
Then: returns Ok(())

Scenario: G9 rejects entry out of bounds
Given: WorkflowParts with parts.entry >= parts.nodes.len()
When: validate_gate_09_slot_references is called
Then: returns Err(ValidationError) with GATE_09 code

Scenario: G9 rejects next StepIdx out of bounds
Given: WorkflowParts with node.next = Some(nodes.len())
When: validate_gate_09_slot_references is called
Then: returns Err(ValidationError) with GATE_09 code and step_idx
```

### Behavior 25-32: validate_gate_10 node-kind structural constraints

```
Scenario: G10 accepts ForEach with matching Join
Given: WorkflowParts with ForEachStart at idx 5 and ForEachJoin at idx 10
When: validate_gate_10_node_kind_specific is called
Then: returns Ok(()) for this constraint

Scenario: G10 rejects orphaned ForEachStart
Given: WorkflowParts with ForEachStart but no ForEachJoin
When: validate_gate_10_node_kind_specific is called
Then: returns Err(ValidationError) with GATE_10 code and ForEachStart step_idx

Scenario: G10 rejects orphaned TogetherStart
Given: WorkflowParts with TogetherStart but no TogetherJoin
When: validate_gate_10_node_kind_specific is called
Then: returns Err(ValidationError) with GATE_10 code

Scenario: G10 rejects orphaned ReduceStart
Given: WorkflowParts with ReduceStart but no ReduceFinish
When: validate_gate_10_node_kind_specific is called
Then: returns Err(ValidationError) with GATE_10 code

Scenario: G10 rejects orphaned CollectStart
Given: WorkflowParts with CollectStart but no CollectFinish
When: validate_gate_10_node_kind_specific is called
Then: returns Err(ValidationError) with GATE_10 code
```

### Behavior 34-37: validate_gate_11 loop body graph

```
Scenario: G11 accepts ForEach with well-formed body
Given: ForEachStart at idx 3, body nodes 4-7, ForEachJoin at idx 8
When: validate_gate_11_loop_body_graph is called
Then: returns Ok(()) for this constraint

Scenario: G11 rejects ForEach with malformed body
Given: ForEachStart body does not lead to ForEachJoin
When: validate_gate_11_loop_body_graph is called
Then: returns Err(ValidationError) with GATE_11 code

Scenario: G11 accepts Together with well-formed body
Given: TogetherStart at idx 3, body nodes 4-7, TogetherJoin at idx 8
When: validate_gate_11_loop_body_graph is called
Then: returns Ok(()) for this constraint
```

### Behavior 39-41: validate_gate_12 action contract bijection

```
Scenario: G12 accepts complete bijection
Given: Do nodes at [1, 2, 3] with names ["a", "b", "c"] and contracts ["a", "b", "c"]
When: validate_gate_12_action_contract_completeness is called
Then: returns Ok(())

Scenario: G12 rejects missing Do node for contract
Given: contracts have name "d" but no Do node with name "d"
When: validate_gate_12_action_contract_completeness is called
Then: returns Err(GATE_12) indicating orphan contract

Scenario: G12 rejects missing contract for Do node
Given: Do node at idx 5 has name "e" but no contract with name "e"
When: validate_gate_12_action_contract_completeness is called
Then: returns Err(GATE_12) indicating missing contract
```

### Behavior 43-45: validate_gate_13 slot cycle detection

```
Scenario: G13 accepts acyclic slot graph
Given: slot 0 writes to slot 1, slot 1 writes to slot 2
When: validate_gate_13_no_slot_cycles is called
Then: returns Ok(())

Scenario: G13 rejects direct cycle
Given: slot 0 writes to slot 1 and slot 1 writes to slot 0
When: validate_gate_13_no_slot_cycles is called
Then: returns Err(GATE_13) with cycle description

Scenario: G13 rejects transitive cycle
Given: slot 0 → slot 1 → slot 2 → slot 0
When: validate_gate_13_no_slot_cycles is called
Then: returns Err(GATE_13) with cycle description

Scenario: G13 terminates within slot_count iterations
Given: slot_count = N
When: cycle detection runs
Then: terminates after at most N iterations
```

### Behavior 47-48: validate_gate_14 slot type compatibility

```
Scenario: G14 accepts compatible multi-writer types
Given: slot 0 written by node A (type String) and node B (type String)
When: validate_gate_14_slot_type_consistency is called
Then: returns Ok(())

Scenario: G14 rejects incompatible multi-writer types
Given: slot 0 written by node A (type String) and node B (type Number)
When: validate_gate_14_slot_type_consistency is called
Then: returns Err(GATE_14) indicating type mismatch
```

### Behavior 50-51: validate_gate_15 non-determinism separation

```
Scenario: G15 accepts separated ND nodes
Given: ND node at idx 3, suspension at idx 5, ND node at idx 7
When: validate_gate_15_determinism_proof is called
Then: returns Ok(())

Scenario: G15 rejects adjacent ND nodes
Given: ND node at idx 3 and ND node at idx 4 with no suspension between
When: validate_gate_15_determinism_proof is called
Then: returns Err(GATE_15) indicating missing suspension
```

### Behavior 54-56: Error handling requirements

```
Scenario: All 37 error variants constructible
Given: ValidationError enum with 37 variants
When: each variant is constructed with required fields
Then: all 37 compile successfully

Scenario: Validation does not panic on malformed input
Given: WorkflowParts with deliberately corrupt data
When: validate(parts) is called
Then: returns Err, never panics

Scenario: Validation has no unwrap in pipeline
Given: the validation pipeline source code
When: clippy is run with unwrap warnings enabled
Then: zero unwrap/expect findings in pipeline code
```

---

## 4. Proptest Invariants

### P1: validate determinism

```
Function: validate
Invariant: For any well-formed WorkflowParts, validate(parts) produces identical results across 1000 invocations
Strategy: (parts in well_formed_strategy())
Anti-invariant: None — determinism should always hold
```

### P2: validate_with_contracts bijection completeness

```
Function: validate_gate_12
Invariant: validate_gate_12 returns Ok iff Do_nodes.map(name) == contracts.map(name)
Strategy: (nodes in nodes_strategy(), contracts in contracts_strategy()) where bijection holds
Anti-invariant: orphan contract → must return Error
```

### P3: Expression stack depth monotonicity

```
Function: ExprStackDepth
Invariant: ExprStackDepth(expr) = 1 + max(ExprStackDepth(children)) for non-leaf
Strategy: (expr in arbitrary_expression_tree())
Anti-invariant: None — must always hold
```

### P4: Slot index monotonicity

```
Function: validate_gate_09
Invariant: All node slot references remain within [0, slot_count) after validation
Strategy: (parts in arbitrary_workflow_parts())
Anti-invariant: parts with out-of-bounds slot refs → Error
```

### P5: Node kind matching completeness

```
Function: validate_gate_10
Invariant: ForEachStart.count() == ForEachJoin.count() in any valid parts
Strategy: (nodes in arbitrary_nodes_with_matching_pairs())
Anti-invariant: mismatch in start/join counts → Error
```

### P6: Loop body graph well-formedness

```
Function: validate_gate_11
Invariant: ForEach body subgraph is connected and terminates at ForEachJoin
Strategy: (nodes in arbitrary_loop_nodes())
Anti-invariant: disconnected loop body → Error
```

### P7: Slot cycle absence

```
Function: validate_gate_13
Invariant: Slot dependency graph is acyclic (no path from slot to itself)
Strategy: (nodes in arbitrary_nodes(), slot_count in 1..100u16)
Anti-invariant: cyclic slot dependencies → Error
```

### P8: ND node separation

```
Function: validate_gate_15
Invariant: Between any two ND nodes, there exists a suspension point (Yield/Suspend node)
Strategy: (nodes in arbitrary_workflow_nodes())
Anti-invariant: adjacent ND nodes without suspension → Error
```

---

## 5. Fuzz Targets

### F1: validate with arbitrary WorkflowParts

```
Target: fuzz_validate
Input: arbitrary bytes → WorkflowParts deserialization
Risk: panic on malformed IR, stack overflow on deep expressions, OOM on large graphs
Corpus seeds:
  - minimal valid workflow (1 node)
  - deep expression tree (depth 64)
  - max slot_count boundary
  - all 14 node variants present
  - empty workflow parts (should Error)
Minimize to: crash, panic, UB detection
```

### F2: validate_with_contracts with arbitrary WorkflowParts and contracts

```
Target: fuzz_validate_with_contracts
Input: arbitrary bytes → (WorkflowParts, Vec<ActionContract>) deserialization
Risk: panic on malformed contracts, incorrect bijection logic, type confusion
Corpus seeds:
  - valid parts + matching contracts (bijection holds)
  - parts with orphan Do nodes
  - parts with orphan contracts
  - empty contracts list
  - duplicate contract names
Minimize to: crash, panic, UB detection, incorrect validation result
```

---

## 6. Kani Harnesses

### K1: G7 Expression Stack Depth Bounded by 64

```
Property: For all expressions in parts.expressions, ExprStackDepth(expr) <= 64
Bound: expression tree depth <= 64
Input: parts.expressions (bounded by 64)
Assumption: parts.well_typed()
Strategy: Induction on expression tree depth
PO: PO-001
```

### K2: G7 Stack Depth No Overflow

```
Property: Stack depth computation does not overflow usize
Bound: expression tree depth
Input: parts.expressions
Assumption: none
Strategy: Miri + Kani overflow checking
PO: PO-002
```

### K3: G8 Symbol Lookup Total

```
Property: SymbolLookup(segment, symbols) != None for all valid segments
Bound: symbols_count
Input: parts.accessors, parts.symbols_count
Assumption: parts.accessors.well_formed()
Strategy: Bounded traversal of accessor path segments
PO: PO-003
```

### K4: G9 Slot Reference Bounds

```
Property: For all nodes, node.output < slot_count when Some
Bound: slot_count (<= 65535)
Input: parts.nodes, parts.slot_count
Assumption: slot_count fits in u16
Strategy: Bounded traversal of all nodes
PO: PO-005
```

### K5: G9 Error Slot Bounds

```
Property: For all nodes, node.error_slot < slot_count when Some
Bound: slot_count
Input: parts.nodes, parts.slot_count
Assumption: slot_count fits in u16
Strategy: Bounded traversal
PO: PO-006
```

### K6: G10 ForEachStart Matching ForEachJoin

```
Property: For all ForEachStart nodes, exists matching ForEachJoin
Bound: nodes.len()
Input: parts.nodes
Assumption: nodes.len() finite
Strategy: Find matching node in array
PO: PO-008
```

### K7: G10 TogetherStart Matching TogetherJoin

```
Property: For all TogetherStart nodes, exists matching TogetherJoin
Bound: nodes.len()
Input: parts.nodes
Assumption: nodes.len() finite
Strategy: Find matching node in array
PO: PO-009
```

### K8: G10 ReduceStart Matching ReduceFinish

```
Property: For all ReduceStart nodes, exists matching ReduceFinish
Bound: nodes.len()
Input: parts.nodes
Assumption: nodes.len() finite
Strategy: Find matching node in array
PO: PO-010
```

### K9: G10 CollectStart Matching CollectFinish

```
Property: For all CollectStart nodes, exists matching CollectFinish
Bound: nodes.len()
Input: parts.nodes
Assumption: nodes.len() finite
Strategy: Find matching node in array
PO: PO-011
```

### K10: G11 ForEach Body Graph Well-Formed

```
Property: ForEachStart body subgraph leads to ForEachJoin
Bound: nodes.len()
Input: parts.nodes
Assumption: ForEachStart exists
Strategy: Graph traversal from start to join
PO: PO-013
```

### K11: G12 Do to ActionContract Surjection

```
Property: Every Do node has a corresponding ActionContract
Bound: action_contracts.len()
Input: parts.nodes, action_contracts
Assumption: action_contracts.len() > 0
Strategy: Bounded lookup of action_name in contracts
PO: PO-016
```

### K12: G12 ActionContract to Do Injection

```
Property: Every ActionContract corresponds to a Do node
Bound: nodes.len()
Input: parts.nodes, action_contracts
Assumption: nodes.len() > 0
Strategy: Bounded lookup of action_name in nodes
PO: PO-017
```

### K13: G13 Slot Dependency Graph Acyclic

```
Property: No circular dependencies in slot graph
Bound: slot_count iterations
Input: parts.nodes, parts.slot_count
Assumption: slot_count finite
Strategy: slot_count bounded cycle detection
PO: PO-019
```

### K14: G14 Multi-writer Slots Compatible Types

```
Property: For all slots with multiple writers, types are compatible
Bound: slot_count
Input: parts.nodes, parts.slot_count
Assumption: slot_count finite
Strategy: Pairwise type compatibility check
PO: PO-022
```

### K15: G15 ND Nodes Separated by Suspension

```
Property: For all ND node pairs, exists suspension point between them
Bound: nodes.len()
Input: parts.nodes
Assumption: finite workflow
Strategy: Graph search for suspension between ND nodes
PO: PO-024
```

### K16: Pipeline Composition Soundness

```
Property: validate_with_contracts returns Ok implies all gates pass
Bound: 9 gates
Input: parts, action_contracts
Assumption: validate_with_contracts returns Ok
Strategy: Conjunction of all gate postconditions
PO: PO-030
```

### K17: Node Output Bounds (INV-6)

```
Property: For any CompiledNode, output SlotIdx < slot_count when Some
Bound: slot_count
Input: parts.nodes, parts.slot_count
Assumption: none
Strategy: Bounded check per node
PO: PO-005
```

### K18: Node Next Bounds (INV-7)

```
Property: For any CompiledNode, next StepIdx < nodes.len()
Bound: nodes.len()
Input: parts.nodes
Assumption: none
Strategy: Bounded check per node
PO: PO-005
```

### K19: Node On-Error Bounds (INV-8)

```
Property: For any CompiledNode, on_error StepIdx < nodes.len() when Some
Bound: nodes.len()
Input: parts.nodes
Assumption: none
Strategy: Bounded check per node
PO: PO-005
```

### K20: Entry Bounds (INV-9)

```
Property: entry StepIdx < nodes.len()
Bound: nodes.len()
Input: parts.nodes, parts.entry
Assumption: none
Strategy: Single bounded check
PO: PO-005
```

---

## 7. Miri UB Checks

All gates require Miri execution to verify no undefined behavior:

| Gate | PO | Property | Execution Order |
|------|----|----------|-----------------|
| G7 | PO-002 | Stack depth no overflow | 1 |
| G8 | PO-004 | Accessor path no UB | 1 |
| G9 | PO-007 | Slot reference no UB | 1 |
| G10 | PO-012 | Node kind structural no UB | 1 |
| G11 | PO-015 | Loop body graph no UB | 1 |
| G13 | PO-021 | Slot cycle detection no UB | 1 |
| G14 | PO-023 | Slot type consistency no UB | 1 |
| G15 | PO-027 | Determinism proof no UB | 1 |
| Pipeline | PO-029 | Pipeline no side effects | 1 |

---

## 8. TLA+/Lean Deferred Proofs

### T1: G13 Slot Cycle Detection Terminates (Deferred)

```
Property: Cycle detection algorithm terminates within slot_count iterations
Verifier: TLA+
Bound: slot_count
Assumption: finite slot_count
Deferral Condition: Run after PO-019 (Kani) passes
PO: PO-020
```

### T2: G15 Determinism Temporal Property (Deferred)

```
Property: ND nodes separated implies valid workflow
Verifier: TLA+
Bound: nodes.len()
Assumption: finite workflow
Deferral Condition: Run after PO-024 (Kani) passes
PO: PO-025
```

### T3: G15 ND Nodes Separated Formal Proof (Deferred)

```
Property: NDNodesSeparated holds iff validation passes
Verifier: Lean
Bound: theorem
Assumption: parts.well_typed()
Deferral Condition: Run after T2 (TLA+) passes
PO: PO-026
```

### T4: G13 Cycle Detection Formal Proof (Deferred)

```
Property: G13_NoCycle invariant holds for all slot graphs
Verifier: Lean
Bound: theorem
Assumption: slot_count finite
Deferral Condition: Run after T1 (TLA+) passes
```

---

## 9. Mutation Testing Checkpoints

Critical mutations that must be caught:

| Function | Mutation | Must be caught by |
|----------|----------|-------------------|
| validate_gate_07 | Change > to >= | test_expression_stack_depth_bound |
| validate_gate_08 | Skip unresolved symbol check | test_accessor_path_resolution |
| validate_gate_09 | Change >= to > | test_slot_reference_bounds |
| validate_gate_10 | Skip ForEachJoin matching | test_foreach_matching_join |
| validate_gate_11 | Skip body graph traversal | test_foreach_body_well_formed |
| validate_gate_12 | Skip injection check | test_contract_to_do_injection |
| validate_gate_13 | Skip cycle detection | test_no_slot_cycles |
| validate_gate_14 | Skip type compatibility | test_slot_type_compatibility |
| validate_gate_15 | Skip suspension check | test_nd_nodes_separated |
| validate | Add unwrap() on gate result | test_no_unwrap_in_pipeline |
| validate | Add mutative step | test_validate_immutable |

**Threshold**: 90% mutation kill rate minimum

---

## 10. Integration Test Matrix

| Call Site | Requirement | Test | Expected Outcome |
|-----------|-------------|------|------------------|
| compile.rs:30 | R16 | test_compile_calls_validate_with_contracts | validate_with_contracts invoked |
| api_compilation.rs:51 | R17 | test_api_compilation_calls_validate_with_contracts | validate_with_contracts invoked |
| schema.rs:651 | R18 | test_schema_calls_validate | validate invoked |
| types.rs:155 | R19 | test_types_calls_validate | validate invoked |
| commands_verify.rs:76 | R20 | test_verify_command_calls_validate | validate invoked |
| fuzz/lib.rs:40,60 | R21 | test_fuzz_calls_validate_with_contracts | validate_with_contracts invoked |
| vb_compile full pipeline | AC9 | cargo test -p vb_compile | All tests pass |
| vb_validate unit | AC1 | cargo test -p vb_validate | All 9 gate tests pass |
| Fuzz target | AC10 | cargo fuzz run | No crashes, no UB |

---

## 11. Error Variant Coverage

All 37 ValidationError variants must be explicitly constructible:

| Variant Category | Count | Test |
|------------------|-------|------|
| G7 expression stack | 1 | test_all_error_variants_constructible |
| G8 accessor path | 1 | test_all_error_variants_constructible |
| G9 slot bounds | 4 | test_all_error_variants_constructible |
| G10 node kind | 5 | test_all_error_variants_constructible |
| G11 loop body | 2 | test_all_error_variants_constructible |
| G12 bijection | 2 | test_all_error_variants_constructible |
| G13 slot cycles | 1 | test_all_error_variants_constructible |
| G14 type compatibility | 1 | test_all_error_variants_constructible |
| G15 determinism | 1 | test_all_error_variants_constructible |
| Pipeline internal | 19 | test_all_error_variants_constructible |

---

## 12. Open Questions

1. **Lean proof environment**: Is Lean installed in CI? PO-026 depends on lean proof kernel.
2. **Fuzz corpus**: Should we seed fuzz corpus with real workflows from production?
3. **Mutation testing threshold**: Is 90% acceptable, or should we target 95%?
4. **TLA+ model bounds**: Confirm slot_count max bound for TLC model checker.
5. **Integration test isolation**: Should integration tests use real vb_compile or test doubles?

---

## 13. Execution Order

Per PO- execution_order field:

| Order | Lane | POs |
|-------|------|-----|
| 1 | Miri | PO-002,004,007,012,015,021,023,027,029 |
| 2 | Proptest | PO-018,028 |
| 3 | Kani | PO-001,003,005,006,008,009,010,011,013,014,016,017,019,022,024,030 |
| 4 | TLA+ (deferred) | PO-020,025 |
| 5 | Lean (deferred) | PO-026 |
| 6 | Integration | PO-031,032,033,034,035 |
| 7 | Fuzz | PO-036 |

---

## 14. Acceptance Criteria Mapping

| AC | Criterion | Test Layer | Evidence |
|----|-----------|------------|----------|
| AC1 | All 9 gates compile and pass unit tests | Unit | cargo test -p vb_validate |
| AC2 | validate() rejects malformed WorkflowParts | Unit + Fuzz | Unit tests + fuzz_validate |
| AC3 | validate_with_contracts() checks G12 bijection | Unit + Kani | test_action_contract_bijection + Kani |
| AC4 | ValidationPipeline::all_gates() enables all gates | Unit | test_all_gates_enabled |
| AC5 | ValidationPipeline::no_gates() disables all gates | Unit | test_no_gates_enabled |
| AC6 | Validation is deterministic | Proptest | PO-028 |
| AC7 | No panic on any input | Miri + Fuzz | PO-002 etc + fuzz |
| AC8 | Compilation succeeds --all-features | Static | cargo build --release |
| AC9 | Integration with vb_compile succeeds | Integration | cargo test -p vb_compile |
| AC10 | Fuzz harness exercises validate_with_contracts | Fuzz | cargo fuzz run |
