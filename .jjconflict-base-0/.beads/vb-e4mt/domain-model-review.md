# Domain Model Review — vb-e4mt

**Bead**: vb-e4mt — bdd: Resource bounds and budget enforcement acceptance scenarios
**State**: 3 (contract)
**Source checkout**: /home/lewis/src/velvet-ballistics

---

## Domain Type Inventory

### Budget Computation Types (vb_core::budget)

#### `WholeWorkflowBudget`
- **Purpose**: Computed worst-case budget certificate for an entire workflow, derived by walking the compiled IR.
- **Dimensions (15)**:
  - `max_total_steps: u64` — sum of all step budgets across all branches
  - `max_total_slots: u64` — maximum slot count across all paths
  - `max_fanout: u16` — maximum concurrent branches (fanout)
  - `max_nesting_depth: u16` — maximum loop nesting depth
  - `max_steps_executable: u32` — maximum executable step count per workflow admission
  - `max_action_tickets: u32` — maximum Do nodes in the workflow
  - `max_parallel_in_flight: u16` — maximum parallel in-flight actions
  - `max_retries_per_action: u16` — maximum retries per action
  - `max_gather_pages: u32` — maximum gather pages across all CollectStart nodes
  - `max_gather_items: u32` — maximum gather items across all CollectStart nodes
  - `max_for_each_iterations: u32` — maximum for-each loop iterations
  - `max_together_branches: u16` — maximum together branches in any TogetherStart
  - `max_repeat_attempts: u16` — maximum repeat attempts in any RepeatStart
  - `max_run_time_seconds: u64` — maximum run time in seconds (Phase 0: max_total_steps)
  - `max_result_bytes: u32` — maximum result bytes
  - `max_total_slots_written: u32` — maximum total slots written
- **Construction**: `WholeWorkflowBudget::compute(nodes, entry, contract)` — walks IR from entry
- **Key invariant**: All dimensions are finite (no `u64::MAX` from overflow)

#### `BoundednessPolicy`
- **Purpose**: Global absolute safety ceiling applied across all workflows.
- **Fields (8)**:
  - `max_total_steps: u64 = 1_000_000`
  - `max_total_slots: u64 = 65_535`
  - `max_fanout: u16 = 64`
  - `max_nesting_depth: u16 = 8`
  - `absolute_max_action_tickets: u32 = 100_000`
  - `absolute_max_parallel: u16 = 256`
  - `absolute_max_run_time_seconds: u64 = 2_592_000`
  - `absolute_max_result_bytes: u32 = 262_144`
  - `absolute_max_steps_executable: u32 = 1_000_000`
- **Validation**: `BoundednessPolicy::validate(budget)` returns `Ok(())` iff all 8 checks pass
- **Relationship**: `WholeWorkflowBudget <= BoundednessPolicy` must hold at admission

#### `BudgetError`
- **Purpose**: Workflow-level budget computation or policy validation failures.
- **Variants (9)**: `TotalStepsExceeded`, `TotalSlotsExceeded`, `FanoutExceeded`, `NestingDepthExceeded`, `ParallelExceeded`, `ActionTicketsExceeded`, `RunTimeExceeded`, `ResultBytesExceeded`, `StepsExecutableExceeded`
- **Fields**: Each variant carries `actual` and `limit` of the corresponding type
- **Exhaustiveness**: `#[non_exhaustive]` — new variants require semver bump

### Aggregate Runtime Types (vb_core::budget)

#### `AggregateResourceBudget`
- **Purpose**: Whole-run budget required for runtime shard admission.
- **Fields (14)**: mirrors `WholeWorkflowBudget` but as admission request + `max_queue_depth`, `max_journal_batch_bytes`, `max_step_budget_per_tick`, `max_transitions_per_tick`

#### `AggregateResourceUsage`
- **Purpose**: Active shard aggregate usage snapshot; tracks current consumption.
- **Fields (12)**: `u64` counters for steps, action_tickets, parallel, gather_pages, gather_items, result_bytes, total_slots_written, active_runs, queue_depth, journal_batch_bytes, step_budget_per_tick, transitions_per_tick
- **Key methods**:
  - `try_add_budget` — add requested budget; `Overflow` on arithmetic failure
  - `try_sub_budget` — release budget; `Underflow` on going negative
  - `fits_within(capacity)` — returns `true` iff all dimensions <= capacity

#### `AggregateResourceCapacity`
- **Purpose**: Shard-local aggregate admission capacity (per-shard ceiling).
- **Fields (12)**: `u64` ceilings mirroring `AggregateResourceUsage`

#### `AggregateReservation`
- **Purpose**: Exact budget reservation associated with a run.
- **Fields**: `run: RunId`, `requested: AggregateResourceBudget`

#### `AggregateBudgetError`
- **Purpose**: Runtime admission failures.
- **Variants (11)**: `WorkflowBudget`, `PolicyExceeded`, `CapacityExceeded`, `Overflow`, `Underflow`, `QueueDepthExceeded`, `JournalBatchBytesExceeded`, `StepBudgetPerTickExceeded`, `TransitionsPerTickExceeded`, `InsufficientCapacity`, `RunNotFound`
- **Note**: `WorkflowBudget` wraps `WorkflowError` in non-Kani builds; Kani narrows to a stub to avoid drop recursion

### Frame Pool Types (vb_runtime::shard)

#### `FramePool`
- **Purpose**: Bounded frame pool management for run execution frames.
- **Key**: `FramePoolKey = (u16, u16)` — (shard_id, tier)
- **Methods**:
  - `try_acquire(key)` — acquire a frame guard; returns `None` if pool exhausted
  - `release(key)` — return frame to pool
- **Bound**: Pool capacity is bounded at construction; key space is finite `(u16::MAX, u16::MAX)`

### Step Budget Types

#### `EngineSignal::StepBudgetExhausted` (vb_core::engine::step)
#### `RuntimeSignal::StepBudgetExhausted` (vb_runtime::engine::types)
- **Purpose**: Runtime signal when step budget per tick is depleted.
- **Behavior**: Workflow suspends; engine signals step budget exhaustion before any step executes beyond the ceiling.

### Expression Stack Types (vb_core::workflow)

#### `ExprProgram`
- **Purpose**: Compiled expression bytecode with metadata.
- **Fields**: `ops: Box<[ExprOp]>`, `max_stack: u8`
- **Construction**: `ExprProgram::try_from_ops` computes exact `max_stack` from ops; `ExprProgram::try_from_parts` validates declared `max_stack` matches computed

#### `ExprOp`
- **Purpose**: Postfix expression bytecode operation.
- **Variants (28+)**: `LoadSlot`, `LoadConst`, `LoadAccessor`, `Eq`, `NotEq`, `Gt`, `Gte`, `Lt`, `Lte`, `And`, `Or`, `Not`, `Add`, `Sub`, `Mul`, `Div`, `Contains`, `StartsWith`, `EndsWith`, `Has`, `Exists`, `Length`, `Empty`, `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`

### Validation Gate Types (vb_validate::gates)

#### `validate_gate_07_expression_stack_depth`
- **Purpose**: Gate 7 boundedness check — validates every expression program's `max_stack` fits within protocol limit.
- **Limit**: `MAX_EXPR_STACK_DEPTH = 64`
- **Checks**:
  1. `parts.resource_contract.max_expr_stack <= 64`
  2. For each expression: `expr.max_stack <= parts.resource_contract.max_expr_stack`
  3. Declared `max_stack` matches recomputation from opcode stream

---

## Relationship Diagram

```
CompiledWorkflow
    |
    +-- resource_contract: ResourceContract (author caps)
    |       |
    |       +-- max_steps, max_slots, max_fanout, ...
    |
    +-- WholeWorkflowBudget::compute() --> WholeWorkflowBudget
            |
            +-- BoundednessPolicy::validate() --> Ok or BudgetError
            |
            +-- AggregateResourceBudget::from_workflow() --> AggregateResourceBudget
                    |
                    +-- admit_run_with_budget() --> AggregateReservation
                            |
                            +-- AggregateResourceUsage::try_add_budget() (success or Overflow/CapacityExceeded)

FramePool
    +-- try_acquire(key) --> FrameGuard or None
    +-- release(key)

ExprProgram
    +-- try_from_ops() --> ExprProgram { max_stack: u8 }
    +-- try_from_parts() --> validates declared max_stack matches computed
```

---

## Key Invariants

1. **Bounded IR walk**: `WholeWorkflowBudget::compute` terminates on finite `CompiledNode` slice with bounded loop depth.
2. **Saturation safety**: All arithmetic in budget composition uses `saturating_add` / `saturating_mul`; no panic from overflow.
3. **Exhaustive error mapping**: Every out-of-bounds condition maps to exactly one `BudgetError` variant.
4. **Aggregate non-overflow**: `AggregateResourceUsage` counters never exceed `AggregateResourceCapacity` for an active shard.
5. **Frame pool boundedness**: `FramePool` key space is `(u16, u16)` — finite; `try_acquire` returns `None` when exhausted.
6. **Expression stack boundedness**: `MAX_EXPR_STACK_DEPTH = 64` is the protocol hard limit; Gate 7 enforces it.

---

## PARITY / DRIFT Notes

- **PARITY-001**: `BoundednessPolicy::DEFAULT.max_total_slots` (65,535) and `ResourceContract::DEFAULT.max_slots` (1,024) have different values. Contract stance: global safety ceiling and per-workflow default are distinct; accepted contracts must satisfy both.
- **PARITY-002**: `compiled_workflow.rs` has a separate `ResourceContract` shape missing some fields; active/legacy status unresolved (deferred from vb-qi37.2).
- **DRIFT-3**: Historically, bounds were per-primitive with no dataflow analysis; defaults were effectively unbounded. Phase 37/45 resolved this with `WholeWorkflowBudget` static analysis.
