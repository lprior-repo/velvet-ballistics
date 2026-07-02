# TLA+ Temporal Model Plan — vb-e4mt

## Boundary

**Temporal/workflow behavior** (TLA+-owned):
- Workflow admission boundedness: a compiled workflow with `WholeWorkflowBudget` satisfying `BoundednessPolicy` is admitted or rejected before execution starts
- Aggregate resource accounting lifecycle: usage tracked per-shard; admit/release balanced; overflow/underflow impossible
- Step budget per-tick enforcement: engine suspends workflow before executing beyond step ceiling
- Frame pool lifecycle: finite frame acquisition/release per shard/tier

**Rust/core behavior excluded from TLA+** (Verus/Kani/proptest):
- Pure `WholeWorkflowBudget::compute` — IR walk with saturating arithmetic
- Pure `BoundednessPolicy::validate` — 8 exact bound checks
- Pure `AggregateResourceUsage::try_add_budget/try_sub_budget` — capacity accounting
- Expression stack depth validation (Gate 7)
- Frame pool implementation details
- Step budget `checked_sub`/`checked_add` behavior

**External systems abstracted**:
- Action execution side effects
- Persistence / journal writes
- Network I/O

---

## Non-applicability Rationale

This bead covers **BDD acceptance scenarios** for resource bounds enforcement. The core budget computation (`WholeWorkflowBudget::compute`, `BoundednessPolicy::validate`) is a **pure function** with no temporal/state-over-time behavior — it takes a compiled IR and returns a budget certificate or an error. The TLA+ model is relevant for:

1. The **workflow admission boundedness invariant**: a workflow is admitted only if its computed budget satisfies the global policy
2. The **aggregate resource lifecycle**: admit/release balanced, no overflow/underflow
3. The **step budget exhaustion**: per-tick enforcement with suspension semantics

These are state-machine properties. The pure Rust proof kernel (`vb_proof_kernels::resource_budget`) covers the sequential/branch/loop composition correctness. TLA+ covers the system-level lifecycle.

---

## TLA+-Owned Clauses

### TLA-WF-001: Workflow Admission Boundedness
- **Contract clause**: INV-001
- **Module**: `WorkflowBudgetSpec`
- **Property**: Safety — every admitted workflow has a `WholeWorkflowBudget` satisfying `BoundednessPolicy::DEFAULT`

### TLA-WF-002: Aggregate Resource Lifecycle Balance
- **Contract clause**: INV-002
- **Module**: `AggregateResourceSpec`
- **Property**: Safety — `AggregateResourceUsage` dimensions never exceed `AggregateResourceCapacity` after admit; Usage = sum of active Reservations + pending requests

### TLA-WF-003: Step Budget Exhaustion Signaling
- **Contract clause**: POST-006 / INV-005
- **Module**: `StepBudgetSpec`
- **Property**: Safety — `EngineSignal::StepBudgetExhausted` is raised before any step executes beyond per-tick ceiling

### TLA-WF-004: BudgetError Exhaustiveness
- **Contract clause**: INV-006
- **Module**: `WorkflowBudgetSpec`
- **Property**: Safety — every out-of-bounds condition maps to exactly one `BudgetError` variant

---

## Model Shape

### Module: WorkflowBudgetSpec

**Variables**:
- `workflowBudget`: `WholeWorkflowBudget` — current computed budget (or `None` if not computed)
- `policy`: `BoundednessPolicy` — fixed to `BoundednessPolicy::DEFAULT`
- `admitted`: `BOOLEAN` — whether workflow is admitted

**Init**:
```
workflowBudget = None
admitted = FALSE
```

**Actions**:
- `ComputeBudget(nodes, entry, contract)`: Compute `WholeWorkflowBudget` from IR
- `ValidateAgainstPolicy(budget)`: Call `BoundednessPolicy::validate`
- `AdmitWorkflow`: `admitted = TRUE` if validation passed
- `RejectWorkflow`: `admitted = FALSE` if validation failed

**Safety Invariant**:
```
InvAdmission: admitted => workflowBudget # None /\ Validate(policy, workflowBudget) = Ok
```

**State Constraints**:
- `workflowBudget.max_total_steps <= 1_000_000`
- `workflowBudget.max_fanout <= 64`
- etc. (all 8 policy dimensions)

### Module: AggregateResourceSpec

**Variables**:
- `usage`: `AggregateResourceUsage`
- `capacity`: `AggregateResourceCapacity` (fixed at Init)
- `reservations`: `[RunId -> AggregateResourceBudget]` (finite set of active runs)
- `pending`: `AggregateResourceBudget` (in-flight admission request)

**Init**:
```
usage = DefaultAggregateResourceUsage
reservations = {}
pending = ZeroBudget
```

**Actions**:
- `RequestAdmission(req)`: `pending := req`
- `AdmitRun(run, req)`: `reservations := reservations \cup {run}`; `usage := usage + req`
- `ReleaseRun(run)`: `usage := usage - reservations[run]`; `reservations := reservations \ {run}`
- `RejectAdmission(req)`: `pending := ZeroBudget` (request denied)

**Safety Invariants**:
```
InvNoOverflow: usage.dimensions <= capacity.dimensions
InvUsageMatchesReservations: usage = Sum_{r \in reservations} r
```

### Module: StepBudgetSpec

**Variables**:
- `stepBudget`: `u64` — remaining step budget for current tick
- `stepsExecuted`: `u64` — steps executed this tick
- `signal`: `STEP_BUDGET_OK | STEP_BUDGET_EXHAUSTED`

**Init**:
```
stepBudget = INITIAL_BUDGET
stepsExecuted = 0
signal = STEP_BUDGET_OK
```

**Actions**:
- `ConsumeSteps(n)`: when `n <= stepBudget`, `stepBudget := stepBudget - n`, `stepsExecuted := stepsExecuted + n`
- `ExhaustBudget`: when `stepBudget = 0`, `signal := STEP_BUDGET_EXHAUSTED`

**Safety Invariant**:
```
InvExhaustionBeforeSteps: signal = STEP_BUDGET_EXHAUSTED => stepsExecuted = INITIAL_BUDGET
```

---

## Properties

### WorkflowBudgetSpec
- **Safety**: `InvAdmission` — every admitted workflow's budget passes policy validation
- **Liveness**: A workflow with valid budget eventually reaches `AdmitWorkflow` or `RejectWorkflow`

### AggregateResourceSpec
- **Safety**: `InvNoOverflow` — usage never exceeds capacity
- **Safety**: `InvUsageMatchesReservations` — usage exactly matches sum of active reservations
- **Liveness**: Every admitted run eventually releases (no infinite reservation leak)

### StepBudgetSpec
- **Safety**: `InvExhaustionBeforeSteps` — exhaustion signal is raised before steps execute beyond budget
- **Liveness**: Step budget resets at each tick boundary

---

## Fairness

- Weak fairness on `AdmitRun` and `ReleaseRun` actions when enabled
- Weak fairness on `ConsumeSteps` action when steps remain

---

## Refinement to Rust/Runtime

| TLA+ Variable | Rust/Runtime Correspondence |
|----------------|---------------------------|
| `workflowBudget` | `WholeWorkflowBudget::compute()` output |
| `policy` | `BoundednessPolicy::DEFAULT` |
| `admitted` | `CompiledWorkflow` accepted by `validate_budget` |
| `usage` | `AggregateResourceUsage` |
| `capacity` | `AggregateResourceCapacity` |
| `reservations` | `AggregateReservation` per active `RunId` |
| `stepBudget` | `StepBudget::remaining()` |
| `signal` | `EngineSignal::StepBudgetExhausted` |

---

## Evidence Command

```bash
# TLC model check for WorkflowBudgetSpec
tlc -config specs/WorkflowBudgetSpec.cfg specs/WorkflowBudgetSpec.tla

# TLC model check for AggregateResourceSpec
tlc -config specs/AggregateResourceSpec.cfg specs/AggregateResourceSpec.tla

# TLC model check for StepBudgetSpec
tlc -config specs/StepBudgetSpec.cfg specs/StepBudgetSpec.tla
```

Note: Actual `.tla` / `.cfg` files are written by the proof-writer skill. This document defines the model intent and variable shapes.

---

## Waivers

- **WAIVER-TLA-001**: Pure budget computation (`WholeWorkflowBudget::compute`) is Verus-owned, not TLA+. Rationale: pure function with no temporal behavior; no state machine needed.
- **WAIVER-TLA-002**: Expression stack depth enforcement is Verus-owned. Rationale: pure validation function; no concurrent/distributed state.
- **WAIVER-TLA-003**: Frame pool acquire/release is runtime-managed; covered by integration tests and Miri for UB, not TLA+.
