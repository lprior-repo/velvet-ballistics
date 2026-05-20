# Contract Specification — vb-e4mt

## Context

- **Bead**: vb-e4mt — bdd: Resource bounds and budget enforcement acceptance scenarios
- **State**: 3 (contract)
- **Source checkout**: /home/lewis/src/velvet-ballistics
- **Isolated workspace**: /home/lewis/src/vb-e4mt-workspace
- **Parent**: vb-hjvq (release: Full E2E BDD acceptance suite)
- **Blocks**: vb-oewy (bdd: Full suite runner and evidence artifact contract)

## Domain Terms

| Term | Definition |
|------|-----------|
| `WholeWorkflowBudget` | Computed worst-case budget for an entire workflow, derived by IR walk (15 dimensions) |
| `BoundednessPolicy` | Global absolute safety ceiling; `WholeWorkflowBudget` must satisfy this before admission |
| `ResourceContract` | Per-workflow static limits declared by author; must satisfy both `BoundednessPolicy` and `ResourceContract` |
| `AggregateResourceBudget` | Whole-run budget required for runtime shard admission |
| `AggregateResourceUsage` | Active shard aggregate usage snapshot; tracks current consumption |
| `AggregateResourceCapacity` | Shard-local aggregate admission capacity |
| `AggregateReservation` | Exact budget reservation associated with a run |
| `BudgetError` | Workflow-level budget computation or policy validation failures (9 variants) |
| `AggregateBudgetError` | Runtime admission failures (11 variants) |
| `FramePool` | Bounded frame pool; `(u16, u16)` key = (shard_id, tier) |
| `StepBudget` | Per-tick step execution budget; enforced by `EngineSignal::StepBudgetExhausted` |
| `ExpressionStack` | Bounded evaluation stack; max depth = 64 per protocol |

## Assumptions

- A compiled/validated workflow artifact with a concrete `ResourceContract` arrives at admission
- `WholeWorkflowBudget::compute` walks a finite `CompiledNode` slice with bounded loop nest depth
- `BoundednessPolicy::DEFAULT` represents the global safety ceiling shared across all workflows
- Frame pool capacity is bounded by `(u16::MAX, u16::MAX)` per shard/tier
- Step budget is a hard per-tick ceiling, not a global wallet
- Aggregate admission uses `try_add_budget` / `try_sub_budget` with overflow detection

## Open Questions

- **OQ-001**: `GAP-1` — `BudgetError` lacks `primitive`, `node_index`, `structural_path` fields per vb_qi37_2_4 BLOCK_LOCAL spec. Resolution required before final evidence.
- **OQ-002**: Full evidence gate coverage for `BoundednessPolicy` validation completeness — documented but not fully evidenced in CI.
- **OQ-003**: Expression stack depth enforcement (Gate 7) — test coverage completeness unknown.

---

## Preconditions

- **PRE-001**: `WholeWorkflowBudget::compute` input (`nodes`, `entry`, `contract`) is a validated compiled workflow artifact with finite `ResourceContract`.
- **PRE-002**: Every `ResourceContract` numeric dimension is finite, nonzero where zero would mean unbounded runtime work, and no larger than `BoundednessPolicy::DEFAULT` or corresponding hard limit constants.
- **PRE-003**: Frame pool allocation request key `(shard_id, tier)` must be within `u16::MAX` for both components.
- **PRE-004**: Step budget per tick is a positive `u64` when a run is admitted.
- **PRE-005**: Expression program `max_stack` is within `MAX_EXPR_STACK_DEPTH = 64` at gate 7 validation time.

## Postconditions

- **POST-001**: `WholeWorkflowBudget::compute` returns a budget where every dimension is finite (no `u64::MAX`, no `u32::MAX` from overflow) and reflects exact IR worst-case.
- **POST-002**: `BoundednessPolicy::validate` returns `Ok(())` iff all 8 policy checks pass; returns the first failing `BudgetError` variant otherwise.
- **POST-003**: `AggregateResourceUsage::try_add_budget` returns `Ok(())` on success or `AggregateBudgetError::Overflow` / `AggregateBudgetError::CapacityExceeded` on failure; never panics.
- **POST-004**: `AggregateResourceUsage::fits_within` returns `true` iff all dimensions of `self` are <= corresponding dimensions of `capacity`.
- **POST-005**: Frame pool `FramePoolKey` lookups always return `None` for absent keys; `FramePool` never grows beyond its constructed capacity.
- **POST-006**: Step budget exhaustion raises `EngineSignal::StepBudgetExhausted` before any step executes beyond the budget ceiling.

## Invariants

- **INV-001**: For every accepted workflow, `WholeWorkflowBudget` dimensions are finite and satisfy `BoundednessPolicy::DEFAULT`.
- **INV-002**: `AggregateResourceUsage` dimensions never exceed `AggregateResourceCapacity` dimensions for an active shard after admission.
- **INV-003**: Frame pool capacity is bounded; pool key space is `(u16, u16)` — finite.
- **INV-004**: Expression stack depth never exceeds `MAX_EXPR_STACK_DEPTH = 64` for any accepted expression program.
- **INV-005**: Step budget per tick is monotonically non-increasing within a tick and reset at tick boundaries.
- **INV-006**: `BudgetError` variants are exhaustive: total_steps, total_slots, fanout, nesting_depth, parallel, action_tickets, run_time, result_bytes, steps_executable.

---

## Error Taxonomy

| Error Variant | Trigger |
|---------------|---------|
| `BudgetError::TotalStepsExceeded` | `budget.max_total_steps > policy.max_total_steps` |
| `BudgetError::TotalSlotsExceeded` | `budget.max_total_slots > policy.max_total_slots` |
| `BudgetError::FanoutExceeded` | `budget.max_fanout > policy.max_fanout` |
| `BudgetError::NestingDepthExceeded` | `budget.max_nesting_depth > policy.max_nesting_depth` |
| `BudgetError::ParallelExceeded` | `budget.max_parallel_in_flight > policy.absolute_max_parallel` |
| `BudgetError::ActionTicketsExceeded` | `budget.max_action_tickets > policy.absolute_max_action_tickets` |
| `BudgetError::RunTimeExceeded` | `budget.max_run_time_seconds > policy.absolute_max_run_time_seconds` |
| `BudgetError::ResultBytesExceeded` | `budget.max_result_bytes > policy.absolute_max_result_bytes` |
| `BudgetError::StepsExecutableExceeded` | `budget.max_steps_executable > policy.absolute_max_steps_executable` |
| `AggregateBudgetError::WorkflowBudget` | Wraps `WorkflowError` from budget validation at admission |
| `AggregateBudgetError::PolicyExceeded` | Requested aggregate exceeds shard capacity |
| `AggregateBudgetError::CapacityExceeded` | `requested > available` for a specific resource |
| `AggregateBudgetError::Overflow` | Arithmetic overflow in `try_add_budget` / `try_sub_budget` |
| `AggregateBudgetError::Underflow` | Arithmetic underflow in `try_sub_budget` |
| `ValidationError::ExpressionStackExceeded` | Gate 7: `expr.max_stack > contract_stack` or `contract_stack > 64` |

---

## Contract Signatures

```rust
// WholeWorkflowBudget computation
fn WholeWorkflowBudget::compute(
    nodes: &[CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<Self, WorkflowError>

// BoundednessPolicy validation
fn BoundednessPolicy::validate(&self, budget: &WholeWorkflowBudget) -> Result<(), BudgetError>

// Aggregate runtime admission
fn aggregate_budget_from_workflow(workflow: &CompiledWorkflow) -> Result<AggregateResourceBudget, AggregateBudgetError>
fn admit_run_with_budget(
    requested: AggregateResourceBudget,
    usage: &mut AggregateResourceUsage,
    capacity: AggregateResourceCapacity,
) -> Result<RunAdmission, AggregateBudgetError>

// Frame pool
fn FramePool::try_acquire(&self, key: FramePoolKey) -> Option<FrameGuard>
fn FramePool::release(&self, key: FramePoolKey) -> bool

// Step budget
fn try_consume_step_budget(budget: &mut StepBudget, n: u64) -> Result<(), StepBudgetExhausted>

// Expression stack
fn validate_gate_07_expression_stack_depth(parts: &WorkflowParts) -> ValidationResult<()>
```

---

## TLA+-Owned Clauses

- **INV-001**: Whole-workflow boundedness — temporal safety: the computed `WholeWorkflowBudget` is finite and satisfies `BoundednessPolicy` before any run is admitted. Model: `WorkflowBudgetSpec.tla`.
- **INV-006**: BudgetError exhaustiveness — state transition safety: every out-of-bounds condition maps to exactly one `BudgetError` variant. Covered by `WorkflowBudgetSpec.tla::BudgetErrorVariant` action.

## Verus-Owned Clauses

- **PRE-001 / POST-001**: `WholeWorkflowBudget::compute` — pure function; entry bounds, no panic, overflow-safe arithmetic via `saturating_add`/`saturating_mul`, finite output.
- **POST-002**: `BoundednessPolicy::validate` — pure function; each of 8 checks returns exact `BudgetError` variant.
- **POST-003 / POST-004**: `AggregateResourceUsage::try_add_budget` / `fits_within` — pure; overflow detection, capacity check.
- **INV-004**: `check_expr_stack_bound` — pure; bounded depth computation with exact `CoreError` on violation.

## Theorem-Owned Clauses

- **INV-001** (kernel projection): `vb_proof_kernels::resource_budget` — `sequential_compose`, `branch_compose`, `loop_compose` preserve policy bounds under saturation. Projected to Lean if Verus proof scope exceeds reasonable bead boundary.

## Non-goals

- Runtime I/O, async scheduling, network, or storage behavior during budget enforcement
- External FFI or non-Rust interop
- CLI or human-in-the-loop budget override workflows
- Performance benchmarking (deferred to `vb-fzx7` bead)

---

## Policy Limits Reference

| Limit | Value |
|-------|-------|
| `max_total_steps` | 1,000,000 |
| `max_total_slots` | 65,535 |
| `max_fanout` | 64 |
| `max_nesting_depth` | 8 |
| `absolute_max_action_tickets` | 100,000 |
| `absolute_max_parallel` | 256 |
| `absolute_max_run_time_seconds` | 2,592,000 (30 days) |
| `absolute_max_result_bytes` | 262,144 |
| `absolute_max_steps_executable` | 1,000,000 |
| `MAX_EXPR_STACK_DEPTH` | 64 |
