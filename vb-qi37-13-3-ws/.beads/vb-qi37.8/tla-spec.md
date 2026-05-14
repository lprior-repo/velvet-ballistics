# TLA+ Specification: vb-qi37.8 — Shared Validation Pipeline

## Bead Information
- **bead_id**: vb-qi37.8
- **title**: validate/compile: Prove and complete shared validation pipeline
- **state**: 3 (Contract + Proof Obligations)

---

## 1. Module Overview

```tla
MODULE ValidationPipeline /\ TEMPORAL
\* TLA+ temporal model for shared validation pipeline
\* Captures gate ordering, determinism, and termination properties
```

---

## 2. Constants and Variables

### 2.1 Constants

| Constant | Type | Description |
|----------|------|-------------|
| `MaxNodes` | Nat | Maximum number of workflow nodes |
| `MaxSlots` | Nat | Maximum number of slots (u16 bound) |
| `MaxSymbols` | Nat | Maximum symbols (u32 bound) |
| `MaxExprStack` | 64 | Expression stack depth bound |
| `NodeKinds` | {"Nop","SetConst","Copy","EvalExpr","BuildObject","BuildList","Do","Choose","ChooseSlot","ForEachStart","ForEachNext","ForEachJoin","TogetherStart","TogetherBranch","TogetherJoin","CollectStart","CollectPage","CollectNext","CollectFinish","ReduceStart","ReduceNext","ReduceFinish","RepeatStart","RepeatAttempt","RepeatCheck","RepeatFinish","WaitUntil","WaitEvent","Ask","AskResume","RetryCheck","ErrorHandler","Jump","Finish"} | Enumeration of node variants |

### 2.2 Variables

| Variable | Type | Description |
|----------|------|-------------|
| `workflowParts` | Record | Current WorkflowParts under validation |
| `gateResults` | [GateId -> {"pass","fail","skip"}] | Per-gate validation results |
| `currentGate` | GateId | Gate currently executing (7-15) |
| `errorLog` | Seq(ValidationError) | Sequence of errors encountered |
| `validationState` | {"idle","running","complete","failed"} | Pipeline state |

---

## 3. Gate State Machine

```tla
(* Gate execution state machine *)
GateStateMachine ==
    /\ validationState = "idle"
    /\ currentGate \in {7,8,9,10,11,12,13,14,15}
    /\ validationState' = "running"
    /\ currentGate' = currentGate

(* Gate transition rules *)
GateTransition ==
    /\ validationState = "running"
    /\ IF gateResults[currentGate] = "fail"
       THEN /\ validationState' = "failed"
            /\ errorLog' = Append(errorLog, MakeError(currentGate))
       ELSE /\ currentGate' = NextGate(currentGate)
            /\ IF currentGate' = NIL
               THEN /\ validationState' = "complete"
                    /\ gateResults' = [g \in DOMAIN gateResults |-> "pass"]
               ELSE validationState' = "running"
            /\ UNCHANGED errorLog

NextGate(g) ==
    CASE g = 7 -> 8
      [] g = 8 -> 9
      [] g = 9 -> 10
      [] g = 10 -> 11
      [] g = 11 -> 12
      [] g = 12 -> 13
      [] g = 13 -> 14
      [] g = 14 -> 15
      [] g = 15 -> NIL
      [] OTHER -> NIL
```

---

## 4. Temporal Properties

### 4.1 Liveness Properties

```tla
(* Pipeline eventually completes or fails for any finite input *)
ValidationLiveness ==
    WF__vars(validationState = "idle")
    => <>(validationState = "complete" \/ validationState = "failed")

(* Every enabled gate eventually reaches a terminal state *)
GateLiveness ==
    \A g \in {7,8,9,10,11,12,13,14,15}:
        gateResults[g] \in {"pass","fail","skip"}
```

### 4.2 Safety Properties

```tla
(* Gate ordering: gates execute in strict sequence *)
GateOrderingSafety ==
    validationState = "running"
    => LET completed == {g \in DOMAIN gateResults: gateResults[g] \in {"pass","fail"}}
       IN \A g1,g2 \in completed:
            g1 < g2 => gateResults[g1] # "skip" => gateResults[g2] \in {"pass","fail"}

(* No gate executes after failure *)
NoPostFailureGate ==
    validationState = "failed"
    => \A g \in DOMAIN gateResults:
            g > currentGate => gateResults[g] = "skip"

(* Determinism: same input yields same output *)
ValidationDeterminism ==
    \A s1,s2 \in States:
        s1.workflowParts = s2.workflowParts
        => s1.validationState = s2.validationState
        /\ s1.errorLog = s2.errorLog
```

---

## 5. Gate Formal Specifications

### 5.1 Gate 7: Expression Stack Depth

```tla
(* G7: Expression stack depth bounded by MaxExprStack (64) *)
G7_Pre ==
    /\ workflowParts # NIL
    /\ workflowParts.expressions # <<>>

G7_Inv ==
    \A expr \in DOMAIN workflowParts.expressions:
        StackDepth(worklowParts.expressions[expr]) <= MaxExprStack

G7_Post ==
    gateResults[7] = "pass"
    => G7_Inv
    /\ gateResults[7] = "fail"
    => \E expr \in DOMAIN workflowParts.expressions:
            StackDepth(worklowParts.expressions[expr]) > MaxExprStack
```

### 5.2 Gate 8: Accessor Path Segments

```tla
(* G8: Accessor paths resolve to valid symbols *)
G8_Inv ==
    \A accessor \in DOMAIN workflowParts.accessors:
        \A segment \in DOMAIN accessor.path:
            SymbolLookup(accessor.path[segment], workflowParts.symbols)
            # "undefined"

G8_Post ==
    gateResults[8] = "pass"
    => G8_Inv
```

### 5.3 Gate 9: Slot References

```tla
(* G9: All slot references within slot_count bounds *)
G9_Inv ==
    \A node \in DOMAIN workflowParts.nodes:
        /\ node.output # NIL => node.output < workflowParts.slot_count
        /\ node.error_slot # NIL => node.error_slot < workflowParts.slot_count
        /\ \A ref \in SlotRefs(node): ref < workflowParts.slot_count
```

### 5.4 Gate 10: Node Kind Specific

```tla
(* G10: Node-kind-specific structural constraints *)
G10_Inv ==
    \A node \in DOMAIN workflowParts.nodes:
        CASE node.kind OF
          "ForEachStart" -> \E join \in DOMAIN workflowParts.nodes:
                              workflowParts.nodes[join].kind = "ForEachJoin"
          "TogetherStart" -> \E join \in DOMAIN workflowParts.nodes:
                               workflowParts.nodes[join].kind = "TogetherJoin"
          "ReduceStart" -> \E finish \in DOMAIN workflowParts.nodes:
                             workflowParts.nodes[finish].kind = "ReduceFinish"
          "CollectStart" -> \E finish \in DOMAIN workflowParts.nodes:
                              workflowParts.nodes[finish].kind = "CollectFinish"
          OTHER -> TRUE
        END
```

### 5.5 Gate 11: Loop Body Graph

```tla
(* G11: ForEach/Together body subgraphs are well-formed *)
G11_WellFormed ==
    \A node \in DOMAIN workflowParts.nodes:
        node.kind \in {"ForEachStart","TogetherStart"}
        => IsWellFormedLoopBody(node.body, workflowParts.nodes)
```

### 5.6 Gate 12: Action Contract Completeness

```tla
(* G12: Bijection between Do nodes and ActionContracts *)
G12_Bijection ==
    \A doNode \in {n \in DOMAIN workflowParts.nodes: n.kind = "Do"}:
        \E contract \in DOMAIN actionContracts:
            contract.action_name = doNode.action_name
    /\ \A contract \in DOMAIN actionContracts:
        \E doNode \in {n \in DOMAIN workflowParts.nodes: n.kind = "Do"}:
            contract.action_name = doNode.action_name
```

### 5.7 Gate 13: No Slot Cycles

```tla
(* G13: No circular slot dependencies *)
G13_NoCycle ==
    \A slot \in 0..workflowParts.slot_count-1:
        ~HasCycle(slot, SlotDependencyGraph(workflowParts.nodes))
```

### 5.8 Gate 14: Slot Type Consistency

```tla
(* G14: Multi-writer slots have compatible types *)
G14_TypeConsistent ==
    \A slot \in 0..workflowParts.slot_count-1:
        LET writers == {n \in DOMAIN workflowParts.nodes: n.output = slot}
        IN \A w1,w2 \in writers:
            CompatibleTypes(NodeType(w1), NodeType(w2))
```

### 5.9 Gate 15: Determinism Proof

```tla
(* G15: Non-deterministic nodes separated by suspension points *)
G15_Separated ==
    \A nd1, nd2 \in NonDeterministicNodes(workflowParts.nodes):
        \E suspension \in Suspensions(workflowParts.nodes):
            PathBetween(suspension, nd1, nd2)
```

---

## 6. Model Constraints

```tla
(* Bounded model: finite state space for model checking *)
CONSTRAINT
    /\ workflowParts.nodes \in [1..MaxNodes -> Node]
    /\ workflowParts.slot_count \in 0..MaxSlots
    /\ workflowParts.symbols_count \in 0..MaxSymbols
    /\ Len(errorLog) < 100  \* Reasonable error bound
```

---

## 7. TLC Configuration

```tla
(* Invariants to check *)
INVARIANTS
    GateOrderingSafety
    NoPostFailureGate
    G7_Inv
    G8_Inv
    G9_Inv
    G10_Inv
    G11_WellFormed
    G13_NoCycle
    G14_TypeConsistent

(* Temporal properties *)
TEMPORAL_PROPERTIES
    ValidationLiveness
    GateLiveness
    ValidationDeterminism
```

---

## 8. Apalache Annotations (Optional)

```tla
\* @type: Int => Bool;
StackDepth(expr) == ...

\* @type: (Set(Node), SlotIdx) => Bool;
HasCycle(slot, graph) == ...

\* @type: (Node, Seq(Node)) => Bool;
IsWellFormedLoopBody(node, nodes) == ...
```
