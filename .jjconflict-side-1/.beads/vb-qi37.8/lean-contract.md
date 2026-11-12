# Lean Contract: vb-qi37.8 — Shared Validation Pipeline

## Bead Information
- **bead_id**: vb-qi37.8
- **title**: validate/compile: Prove and complete shared validation pipeline
- **state**: 3 (Contract + Proof Obligations)

---

## 1. Theorem Kernel Overview

This Lean specification serves as the theorem kernel for the validation pipeline. It captures:
- Core functional correctness lemmas
- Invariant preservation proofs
- Termination arguments for each gate
- Compositional reasoning about validation pipeline

---

## 2. Type Definitions

### 2.1 Core Types (Corresponding to vb_core)

```lean
-- Lean representation of WorkflowParts
structure WorkflowParts where
  name : String
  digest : WorkflowDigest
  nodes : Array CompiledNode
  expressions : Array ExprProgram
  accessors : Array AccessorProgram
  constants : Array ConstValue
  slot_count : Nat
  symbols_count : Nat
  entry : Nat
  resource_contract : ResourceContract
  step_names : Array String
deriving Repr

-- Node kind enumeration (14 variants mapped from CompiledNodeKind)
inductive NodeKind
  | Nop | SetConst | Copy | EvalExpr | BuildObject | BuildList
  | Do | Choose | ChooseSlot
  | ForEachStart | ForEachNext | ForEachJoin
  | TogetherStart | TogetherBranch | TogetherJoin
  | CollectStart | CollectPage | CollectNext | CollectFinish
  | ReduceStart | ReduceNext | ReduceFinish
  | RepeatStart | RepeatAttempt | RepeatCheck | RepeatFinish
  | WaitUntil | WaitEvent | Ask | AskResume
  | RetryCheck | ErrorHandler | Jump | Finish

-- CompiledNode structure
structure CompiledNode where
  id : Nat
  output : Option Nat  -- SlotIdx
  next : Option Nat    -- StepIdx
  on_error : Option Nat
  error_slot : Option Nat
  kind : NodeKind
deriving Repr

-- ValidationError variant
inductive ValidationError
  | DuplicateKey | UnknownTopLevelField | UnknownStepField
  | MissingRequiredField | InvalidVersion | InvalidId | ReservedId | DuplicateId
  | MultipleStepPrimitives | MissingStepPrimitive
  | UnknownReference | FutureReference | SecretNotDeclared
  | DirectRuntimeReference | InvalidThenTarget | ControlFlowCycle
  | UnreachableStep | InvalidChoose | InvalidForEach | InvalidTogether
  | InvalidCollect | InvalidReduce | InvalidRepeat | InvalidWait
  | InvalidAsk | InvalidFinish | InvalidRetry | InvalidOnError
  | SecretResultLeak | TypeMismatch | PayloadTooLarge
  | LimitRequired | LimitExceeded | UnsupportedTrigger | HttpTriggerOutOfCore
  | ExpressionStackDepthExceeded | AccessorPathInvalid | SlotRefOutOfBounds
  | LoopBodyMalformed | ActionContractIncomplete | SlotCycleDetected
  | SlotTypeInconsistent | DeterminismViolation
deriving Repr, DecidableEq

-- ValidationResult
inductive ValidationResult
  | Ok
  | Err ValidationError
```

---

## 3. Function Specifications

### 3.1 Gate 7: Expression Stack Depth

```lean
-- Max expression stack depth
def MaxExprStackDepth : Nat := 64

-- Stack depth computation
def ExprStackDepth (expr : ExprProgram) : Nat :=
  match expr with
  | .const _ => 1
  | .var _ => 1
  | .binop _ e1 e2 => 1 + max (ExprStackDepth e1) (ExprStackDepth e2)
  | .access _ path => 1 + path.length
  | .call _ args => 1 + args.foldl (fun acc e => max acc (ExprStackDepth e)) 0

-- G7: Expression stack depth bounded
theorem g7_expression_stack_depth
  (parts : WorkflowParts) :
  parts.expressions.forall (fun e => ExprStackDepth e ≤ MaxExprStackDepth)
  ↔ validate_gate_07 parts = .Ok
```

### 3.2 Gate 8: Accessor Path Segments

```lean
-- Symbol lookup
def SymbolLookup (segment : String) (symbols : Array String) : Option Nat :=
  symbols.findIdx (fun s => s == segment)

-- G8: All accessor paths resolve
theorem g8_accessor_paths_resolve
  (parts : WorkflowParts) :
  (∀ (acc : AccessorProgram) (seg : String),
      acc.path.contains seg → SymbolLookup seg parts.accessors ≠ none)
  ↔ validate_gate_08 parts = .Ok
```

### 3.3 Gate 9: Slot References

```lean
-- Slot reference bounds
def SlotRefValid (node : CompiledNode) (slot_count : Nat) : Bool :=
  match node.output with
  | some slot => slot < slot_count
  | none => true
  ∧ match node.error_slot with
    | some slot => slot < slot_count
    | none => true

-- G9: All slot references within bounds
theorem g9_slot_references
  (parts : WorkflowParts) :
  (∀ (n : CompiledNode), n ∈ parts.nodes → SlotRefValid n parts.slot_count)
  ↔ validate_gate_09 parts = .Ok
```

### 3.4 Gate 10: Node Kind Specific

```lean
-- Find matching join node
def HasMatchingJoin (start : CompiledNode) (nodes : Array CompiledNode) : Bool :=
  match start.kind with
  | .ForEachStart =>
    nodes.any (fun n => n.kind = .ForEachJoin)
  | .TogetherStart =>
    nodes.any (fun n => n.kind = .TogetherJoin)
  | .ReduceStart =>
    nodes.any (fun n => n.kind = .ReduceFinish)
  | .CollectStart =>
    nodes.any (fun n => n.kind = .CollectFinish)
  | _ => true

-- G10: Node-kind-specific structural constraints
theorem g10_node_kind_specific
  (parts : WorkflowParts) :
  (∀ (n : CompiledNode), n ∈ parts.nodes → HasMatchingJoin n parts.nodes)
  ↔ validate_gate_10 parts = .Ok
```

### 3.5 Gate 11: Loop Body Graph

```lean
-- Well-formed loop body
inductive LoopBodyWellFormed : CompiledNode → Array CompiledNode → Prop
  | for_each_wf (start : CompiledNode) (nodes : Array CompiledNode) :
      start.kind = .ForEachStart →
      (∃ join : CompiledNode, join.kind = .ForEachJoin ∧
         PathExists start join nodes) →
      LoopBodyWellFormed start nodes
  | together_wf (start : CompiledNode) (nodes : Array CompiledNode) :
      start.kind = .TogetherStart →
      (∃ join : CompiledNode, join.kind = .TogetherJoin ∧
         PathExists start join nodes) →
      LoopBodyWellFormed start nodes

-- G11: Loop body graph well-formed
theorem g11_loop_body_graph
  (parts : WorkflowParts) :
  (∀ (n : CompiledNode),
      n ∈ parts.nodes ∧ n.kind ∈ {.ForEachStart, .TogetherStart} →
        LoopBodyWellFormed n parts.nodes)
  ↔ validate_gate_11 parts = .Ok
```

### 3.6 Gate 12: Action Contract Completeness

```lean
-- ActionContract representation
structure ActionContract where
  action_name : String
  input_schema : Schema
  output_schema : Schema

-- Bijection between Do nodes and ActionContracts
def DoNodes := [n : CompiledNode | n.kind = .Do]

theorem g12_action_contract_bijection
  (parts : WorkflowParts) (contracts : Array ActionContract) :
  (∀ (do : CompiledNode), do ∈ DoNodes parts.nodes →
    ∃ (c : ActionContract), c.action_name = do.action_name) ∧
  (∀ (c : ActionContract), ∃ (do : CompiledNode),
    do ∈ DoNodes parts.nodes ∧ c.action_name = do.action_name)
  ↔ validate_gate_12 parts contracts = .Ok
```

### 3.7 Gate 13: No Slot Cycles

```lean
-- Slot dependency graph
def SlotDeps (node : CompiledNode) : Array Nat :=
  match node.kind with
  | .Do => match node.output with | some s => #[s] | none => #[] | _ => #[]
  | _ => []

-- No cycle in slot dependencies
def NoSlotCycles (parts : WorkflowParts) : Prop :=
  ∀ (slot : Nat) (trace : List Nat),
    slot ∈ trace → slot ∉ SlotDeps (parts.nodes[slot])

-- G13: No circular slot dependencies
theorem g13_no_slot_cycles
  (parts : WorkflowParts) :
  NoSlotCycles parts ↔ validate_gate_13 parts = .Ok
```

### 3.8 Gate 14: Slot Type Consistency

```lean
-- Type compatibility
inductive TypeCompatible : NodeKind → NodeKind → Prop
  | string_string : TypeCompatible .EvalExpr .EvalExpr
  | num_num : TypeCompatible .EvalExpr .EvalExpr
  | obj_obj : TypeCompatible .BuildObject .BuildObject
  -- ... (complete for all combinations)

-- Multi-writer slots have compatible types
def SlotTypesConsistent (parts : WorkflowParts) : Prop :=
  ∀ (slot : Nat) (w1 w2 : CompiledNode),
    w1.output = some slot ∧ w2.output = some slot →
      w1.kind = w2.kind ∨ TypeCompatible w1.kind w2.kind

-- G14: Slot type consistency
theorem g14_slot_type_consistent
  (parts : WorkflowParts) :
  SlotTypesConsistent parts ↔ validate_gate_14 parts = .Ok
```

### 3.9 Gate 15: Determinism Proof

```lean
-- Non-deterministic node kinds
def IsNonDeterministic : NodeKind → Bool
  | .Choose => true
  | .Ask => true
  | _ => false

-- Suspension points
def IsSuspensionPoint : CompiledNode → Bool
  | .WaitUntil => true
  | .WaitEvent => true
  | _ => false

-- Non-deterministic nodes separated
def NDNodesSeparated (parts : WorkflowParts) : Prop :=
  ∀ (nd1 nd2 : CompiledNode),
    nd1 ∈ parts.nodes ∧ IsNonDeterministic nd1.kind ∧
    nd2 ∈ parts.nodes ∧ IsNonDeterministic nd2.kind ∧ nd1 ≠ nd2 →
      ∃ (sp : CompiledNode),
        sp ∈ parts.nodes ∧ IsSuspensionPoint sp ∧
        PathBetween sp nd1 nd2

-- G15: Determinism proof
theorem g15_determinism_proof
  (parts : WorkflowParts) :
  NDNodesSeparated parts ↔ validate_gate_15 parts = .Ok
```

---

## 4. Pipeline Theorems

### 4.1 Pipeline Composition

```lean
-- Pipeline is composition of gates
def validate_pipeline (parts : WorkflowParts) (gates : Array GateId) : ValidationResult :=
  gates.foldl (fun acc g =>
    match acc with
    | .Err e => .Err e
    | .Ok => validate_gate g parts
  ) .Ok

-- Gate order is fixed: [7,8,9,10,11,12,13,14,15]
def GateOrder : Array GateId := #[7,8,9,10,11,12,13,14,15]

-- Main validation entry point
def validate (parts : WorkflowParts) : ValidationResult :=
  validate_pipeline parts GateOrder

-- With contracts
def validate_with_contracts (parts : WorkflowParts) (contracts : Array ActionContract) : ValidationResult :=
  let g12 := validate_gate 12 parts contracts
  match g12 with
  | .Err e => .Err e
  | .Ok => validate_pipeline parts GateOrder
```

### 4.2 Determinism Theorem

```lean
-- Validation is deterministic
theorem validate_deterministic
  (parts : WorkflowParts) (c1 c2 : Array ActionContract) :
  validate_with_contracts parts c1 = validate_with_contracts parts c2
```

### 4.3 No False Positives

```lean
-- If validation passes, all invariants hold
theorem no_false_positives
  (parts : WorkflowParts) (contracts : Array ActionContract) :
  validate_with_contracts parts contracts = .Ok →
    G7_Inv parts ∧ G8_Inv parts ∧ G9_Inv parts ∧ G10_Inv parts ∧
    G11_Inv parts ∧ G12_Inv parts contracts ∧
    G13_Inv parts ∧ G14_Inv parts ∧ G15_Inv parts
```

### 4.4 No False Negatives

```lean
-- If all invariants hold, validation passes
theorem no_false_negatives
  (parts : WorkflowParts) (contracts : Array ActionContract) :
    G7_Inv parts ∧ G8_Inv parts ∧ G9_Inv parts ∧ G10_Inv parts ∧
    G11_Inv parts ∧ G12_Inv parts contracts ∧
    G13_Inv parts ∧ G14_Inv parts ∧ G15_Inv parts →
  validate_with_contracts parts contracts = .Ok
```

---

## 5. Termination Arguments

```lean
-- Gate execution terminates
theorem gate_terminates (g : GateId) (parts : WorkflowParts) :
  ∃ (result : ValidationResult), validate_gate g parts = result

-- Pipeline terminates (all gates execute finitely)
theorem pipeline_terminates (parts : WorkflowParts) :
  ∃ (result : ValidationResult), validate parts = result

-- Validation with contracts terminates
theorem validate_with_contracts_terminates
  (parts : WorkflowParts) (contracts : Array ActionContract) :
  ∃ (result : ValidationResult),
    validate_with_contracts parts contracts = result
```

---

## 6. Soundness and Completeness

```lean
-- Soundness: validates correct workflows
theorem validate_sound
  (parts : WorkflowParts) (contracts : Array ActionContract) :
  validate_with_contracts parts contracts = .Ok →
    IsValidWorkflow parts ∧ IsValidContracts parts contracts

-- Completeness: accepts all correct workflows
theorem validate_complete
  (parts : WorkflowParts) (contracts : Array ActionContract) :
  IsValidWorkflow parts ∧ IsValidContracts parts contracts →
    validate_with_contracts parts contracts = .Ok
```
