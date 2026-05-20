# Codebase Map — vb-e4mt

**Bead**: vb-e4mt — bdd: Resource bounds and budget enforcement acceptance scenarios
**State**: 2 (explore)
**Source checkout**: /home/lewis/src/velvet-ballistics
**Isolated workspace**: /home/lewis/src/vb-e4mt-workspace
**Generated**: 2026-05-19

---

## 1. Scope Summary

This bead covers BDD acceptance scenarios for **resource bounds and budget enforcement** in the velvet-ballastics workflow engine. Focus areas:

- Bounded queues, frame pools, expression stacks
- Retry budgets and fanout limits
- IPC payloads, blobs, journal batches
- ValueStore arena caps
- Step budget per tick enforcement
- Whole-workflow boundedness analysis

---

## 2. Core Budget Types (vb_core)

### `/home/lewis/src/velvet-ballistics/crates/vb_core/src/budget.rs`

Primary budget computation and policy enforcement module.

| Type | Purpose |
|------|---------|
| `WholeWorkflowBudget` | Computed budget for entire workflow via IR walk |
| `BoundednessPolicy` | Policy limits that computed budget must satisfy |
| `BudgetError` | Budget validation failures (9 variants) |
| `AggregateResourceBudget` | Aggregate budget for runtime admission |
| `AggregateResourceCapacity` | Shard-local aggregate admission capacity |
| `AggregateResourceUsage` | Active shard aggregate usage snapshot |
| `AggregateReservation` | Exact budget reservation for a run |
| `AggregateBudgetError` | Aggregate resource-accounting failures (11 variants) |

**Key methods**:
- `WholeWorkflowBudget::compute()` — walks compiled IR and computes all budget dimensions
- `BoundednessPolicy::validate()` — validates computed budget against policy
- `AggregateResourceBudget::from_workflow()` — derives budget from CompiledWorkflow
- `AggregateResourceUsage::try_add_budget()` / `try_subtract_budget()` — accounting with overflow detection
- `AggregateResourceUsage::fits_within()` — capacity check
- `validate_step_ceilings()` — validates hard limits on step budgets

### `/home/lewis/src/velvet-ballistics/crates/vb_core/src/workflow/mod.rs`

- `ResourceContract` — per-workflow static limits (line ~200+)
- `WorkflowError::BudgetPolicyExceeded` — error variant for policy violations

### `/home/lewis/src/velvet-ballistics/crates/vb_core/src/engine/step.rs`

- `EngineSignal::StepBudgetExhausted` — runtime signal when step budget depleted

### `/home/lewis/src/velvet-ballistics/crates/vb_core/src/validation.rs`

- `BudgetError` integration into validation errors
- `WorkflowError::BudgetPolicyExceeded` mapping

---

## 3. Proof Kernel (vb_proof_kernels)

### `/home/lewis/src/velvet-ballistics/crates/vb_proof_kernels/src/resource_budget.rs`

Pure sequential Rust kernel for resource budget verification. Suitable for Verus/Aeneas extraction.

| Type | Purpose |
|------|---------|
| `Budget` | Proof kernel budget (steps, actions, parallel, retries, etc.) |
| `Policy` | Policy with max limits |

**Composition functions**:
- `sequential_compose()` — additive composition
- `branch_compose()` — max composition
- `loop_compose()` — multiplicative composition

**Saturation**: Uses `saturating_add` and `saturating_mul` for overflow safety.

---

## 4. Verification / Validation (vb_validate)

### `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/gates.rs`

- `validate_gate_07_expression_stack_depth()` — Gate 7 boundedness check
- `MAX_EXPR_STACK_DEPTH = 64` — Protocol max expression stack depth

### `/home/lewis/src/velvet-ballistics/crates/vb_ui/src/verify/resources.rs`

Resource verification utilities (UI layer).

---

## 5. Runtime (vb_runtime)

### `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/types.rs`

- `FramePool` — bounded frame pool management
- `FramePoolKey = (u16, u16)` — shard and tier identification

### `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/engine/types.rs`

- `RuntimeSignal::StepBudgetExhausted` — runtime-level signal

### `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/engine/signal.rs`

- Signal mapping between core and runtime signals

### `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`

- Step budget enforcement in lifecycle

---

## 6. Codegen (vb_codegen)

### `/home/lewis/src/velvet-ballistics/crates/vb_codegen/src/lib.rs`

Generated drive functions include:
- `step_budget_remaining` field initialization
- `DriveError::StepBudgetExhausted` error variant
- `checked_sub` for budget decrement

### `/home/lewis/src/velvet-ballistics/crates/vb_codegen/src/tests.rs`

- `drive_function_has_no_step_budget_enforcement()` — property test
- `post_budget_exhaustion_workflow()` — test workflow
- `post_007_step_budget_exhausted_error_preserved()` — error preservation test

---

## 7. CLI (vb_cli)

### `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/app_impl.rs`

- `VerifyError::BudgetPolicy` — CLI error variant
- Budget metadata reporting in verify command

### `/home/lewis/src/velvet-ballistics/crates/vb_cli/tests/cli_verify_integration.rs`

- `bdd_full_profile_fails_closed_on_budget_violation()`
- `bdd_standard_profile_warns_not_fails_on_budget()`
- `integration_full_profile_runs_budget_gates()`

---

## 8. Integration Tests (workspace_tests)

### `/home/lewis/src/velvet-ballistics/crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs`

Comprehensive budget error coverage:
- 15+ test scenarios covering all BudgetError variants
- `integration_policy_returns_total_slots_exceeded`
- `integration_policy_returns_nesting_depth_exceeded`
- `integration_policy_returns_parallel_exceeded`
- `integration_budget_returns_total_steps_exceeded`
- `integration_collect_overflow_returns_total_steps_exceeded`
- `integration_repeat_overflow_returns_total_steps_exceeded`
- `integration_together_overflow_returns_fanout_exceeded`
- `integration_nested_loops_returns_nesting_depth_exceeded`
- Error path assertions with actual/limit field checking

### `/home/lewis/src/velvet-ballistics/crates/workspace_tests/tests/vb_fzx7_budget_arithmetic.rs`

Budget arithmetic correctness tests.

### `/home/lewis/src/velvet-ballistics/crates/workspace_tests/tests/bdd_validation_tests.rs`

BDD validation tests.

---

## 9. Master Document Reference (velvet-ballistics-MASTER.md)

**Section 64: Whole-Workflow Boundedness Analysis**
- Static dataflow analysis on compiled IR
- `WholeWorkflowBudget` struct (12 dimensions)
- `BoundednessPolicy` struct (6 absolute limits)
- Validation: ResourceContract <= BoundednessPolicy
- Dataflow propagation rules (leaf, sequential, nested, conditional, parallel)

**Key limits**:
- `max_total_steps: 1_000_000`
- `max_total_slots: 65_535`
- `max_fanout: 64`
- `max_nesting_depth: 8`
- `absolute_max_action_tickets: 100_000`
- `absolute_max_parallel: 256`
- `absolute_max_run_time_seconds: 2_592_000` (30 days)
- `absolute_max_result_bytes: 262_144`
- `absolute_max_steps_executable: 1_000_000`

**Boundedness rules** (reject if):
1. `for_each` without declared max
2. `collect` without pages/items/time limit
3. `repeat` without times/time limit
4. `try_again` without max_attempts
5. `wait` without timeout
6. `ask` without timeout
7. `together` exceeding policy
8. Nested fanout exceeding policy
9. `finish` with unknown result size

---

## 10. Risk Tags

| Tag | Description |
|-----|-------------|
| `temporal` | Step budget exhaustion during execution |
| `concurrency` | Frame pool and parallel in-flight limits |
| `arithmetic` | Overflow/underflow in budget computation |
| `validation` | Policy validation at admission time |
| `persistence` | Journal batch byte limits |
| `public_api` | CLI verify command exposes budget errors |

---

## 11. Required Verifier Modes

| Mode | Relevance |
|------|-----------|
| `proptest` | Property-based tests for budget arithmetic |
| `kani` | Bounded panic-freedom for budget computation |
| `miri` | Undefined behavior detection in budget ops |

---

## 12. Open Questions / Unknowns

1. **GAP-1**: BudgetError currently lacks `primitive`, `node_index`, `structural_path` fields (BLOCK_LOCAL per vb_qi37_2_4)
2. Full evidence gate coverage for BoundednessPolicy validation is documented but not fully evidenced
3. Expression stack depth enforcement (Gate 7) - test coverage completeness unknown

---

## 13. Related Beads

| Bead | Relationship |
|------|-------------|
| `vb-hxm0` | Executable behavior catalog (parent) |
| `vb-qi37-2-4` | Budget error variant coverage |
| `vb-oewy` | BDD full suite runner (blocked by this) |
