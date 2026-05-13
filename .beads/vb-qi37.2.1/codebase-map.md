# Codebase Map — vb-qi37.2.1

## Bead Identity

| Field | Value |
|-------|-------|
| bead_id | vb-qi37.2.1 |
| title | runtime: Define aggregate resource budget model |
| phase | 2 |
| source_checkout | /home/lewis/src/Velvet-ballistics |
| isolated_workspace | /home/lewis/src/vb-qi37-2-1 |

## Scope Summary

Design and implement whole-workflow resource accounting that composes primitive costs across nested collect, reduce, repeat, together, waits, asks, and actions. `ResourceContract` has safe non-unbounded defaults; aggregate budgets computed before admission; nested composition cannot bypass caps.

Risk level: **critical** (resource safety)
Required verifier modes: **type-level resource boundedness proof**, **integration tests for budget enforcement**

---

## Key Crates and Files

### vb_core (pure core, no runtime dependencies)

| File | Key Types/Functions |
|------|---------------------|
| `crates/vb_core/src/budget.rs` | `WholeWorkflowBudget`, `BoundednessPolicy`, `AggregateResourceBudget`, `AggregateResourceCapacity`, `AggregateResourceUsage`, `AggregateReservation`, `AggregateBudgetError`, `validate_aggregate_budget()`, `validate_step_ceilings()` |
| `crates/vb_core/src/workflow/mod.rs` | `CompiledWorkflow`, `ResourceContract`, `WorkflowParts`, `CompiledNodeKind`, `WorkflowError` |
| `crates/vb_core/src/validation/resource.rs` | `validate_resource_contract()`, `validate_resource_counts()` (structural validation layer, no runtime deps) |
| `crates/vb_core/src/policy.rs` | `RuntimePolicy` enum (Strict, Journaled, Relaxed) |
| `crates/vb_core/src/ids/mod.rs` | `RunId`, `StepIdx`, `WorkflowDigest` |
| `crates/vb_core/src/budget/tests.rs` | Comprehensive unit tests including blackhat adversarial cases (BH-BUD-01 through BH-BUD-13) |

### vb_runtime (runtime shell)

| File | Key Types/Functions |
|------|---------------------|
| `crates/vb_runtime/src/admission.rs` | `admit_run_with_budget()`, `RunAdmission`, `AdmissionError::ResourceCapacityExceeded`, `ArtifactStore`, `AcceptedArtifactStore` |
| `crates/vb_runtime/src/shard/types.rs` | `Shard`, `ShardConfig`, `RunState` (with `admission: Option<RunAdmission>`), `ShardStatus` |
| `crates/vb_runtime/src/lib.rs` | Runtime library root |

---

## Core Domain Types (Exact Paths)

### Aggregate Budget Types (`crates/vb_core/src/budget.rs`)

```rust
// Line 287-307: Aggregate budget for runtime admission
pub struct AggregateResourceBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
}

// Line 310-326: Shard-local aggregate admission capacity
pub struct AggregateResourceCapacity { /* same dimensions as budget, u64 widths */ }

// Line 329-345: Active shard aggregate usage snapshot
pub struct AggregateResourceUsage { /* same dimensions as capacity */ }

// Line 348-352: Exact budget reservation associated with a run
pub struct AggregateReservation {
    pub run: RunId,
    pub requested: AggregateResourceBudget,
}

// Line 355-390: Aggregate resource-accounting failure
pub enum AggregateBudgetError {
    WorkflowBudget(WorkflowError),
    PolicyExceeded { resource: &'static str, actual: u64, limit: u64 },
    CapacityExceeded { resource: &'static str, requested: u64, available: u64 },
    Overflow { resource: &'static str },
    Underflow { resource: &'static str },
    InvalidCapacity { resource: &'static str },
    ReservationNotFound { run: RunId },
    StepCeilingExceeded { requested: u64, limit: u64 },
    PerTickCeilingExceeded { requested: u64, limit: u64 },
}
```

### ResourceContract (`crates/vb_core/src/workflow/mod.rs`, line 172-209)

```rust
pub struct ResourceContract {
    pub max_steps: u16,
    pub max_slots: u16,
    pub max_constants: u16,
    pub max_accessors: u16,
    pub max_expressions: u16,
    pub max_expr_stack: u8,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_ipc_payload_bytes: u32,
    pub max_retry_attempts: u16,
    pub max_fanout: u16,
    pub max_collect_items: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub allows_secret_results: bool,
}
```

### Admission (`crates/vb_runtime/src/admission.rs`)

```rust
// Line 59-71: RunAdmission record
pub struct RunAdmission {
    artifact_digest: WorkflowDigest,
    run_id: RunId,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
    budget: Option<AggregateResourceBudget>,  // Carries aggregate budget when budget admission used
}

// Line 91-105: Constructor with budget
pub fn with_budget(...) -> Self

// Line 444-471: Main admission function with budget checking
pub fn admit_run_with_budget(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    requested: AggregateResourceBudget,
    available: AggregateResourceCapacity,
) -> Result<RunAdmission, AdmissionError>

// Line 139-186: AdmissionError including ResourceCapacityExceeded
pub enum AdmissionError {
    ResourceCapacityExceeded { resource: &'static str, requested: u64, available: u64 },
    // ... other variants
}
```

---

## Key Methods on AggregateResourceBudget

**Lines 392-428**: `AggregateResourceBudget` constructors:
- `from_workflow(workflow: &CompiledWorkflow) -> Result<Self, AggregateBudgetError>` — walks IR, computes `WholeWorkflowBudget`, validates step ceilings
- `from_whole_workflow_budget(budget, contract) -> Result<Self, AggregateBudgetError>` — converts whole-workflow budget plus contract fields

**Lines 431-624**: `AggregateResourceUsage` methods:
- `try_add_budget(&self, budget: &AggregateResourceBudget) -> Result<Self, AggregateBudgetError>` — checked addition, returns Overflow on failure
- `try_subtract_budget(&self, budget: &AggregateResourceBudget) -> Result<Self, AggregateBudgetError>` — checked subtraction, returns Underflow on failure
- `fits_within(&self, capacity: &AggregateResourceCapacity) -> Result<(), AggregateBudgetError>` — capacity comparison, returns CapacityExceeded on failure

**Lines 627-740**: `validate_aggregate_budget(budget, policy) -> Result<(), AggregateBudgetError>` — validates against `BoundednessPolicy`

**Lines 703-740**: `validate_step_ceilings(budget) -> Result<(), AggregateBudgetError>` — validates `max_step_budget_per_tick` and `max_transitions_per_tick` against hard limits (1_000_000 each)

---

## Existing Tests

### budget/tests.rs
- Unit tests for `WholeWorkflowBudget::compute()` (linear, branching, nested loops, fanout)
- Unit tests for `BoundednessPolicy::validate()`
- Unit tests for `StepBudget` (creation, consumption, exhaustion)
- BLACKHAT adversarial tests BH-BUD-01 through BH-BUD-13 documenting known issues:
  - BH-BUD-01: `max_steps_executable` silent saturation (u32::MAX)
  - BH-BUD-02: `max_run_time_seconds` hardcoded to 0
  - BH-BUD-03: `From<WorkflowError>` loses information
  - BH-BUD-04: ForEach limit=0 counts as 1 iteration
  - BH-BUD-05: Step count overflow uses misleading error variant
  - BH-BUD-06: `action_tickets` saturating_add hides overflow
  - BH-BUD-07: `gather_items` saturating_add accumulation
  - BH-BUD-08: `retries_per_action` copied from contract not computed
  - BH-BUD-09: forward jump does not trigger cycle detection
  - BH-BUD-10: policy boundary exact vs over
  - BH-BUD-11: StepBudget clamping is silent
  - BH-BUD-12: self-referencing loop body graceful handling
  - BH-BUD-13: ReduceStart uses MAX_LIST_ITEMS_PER_VALUE iterations

### admission.rs tests (line 523-748)
- Unit tests for `RunAdmission`, `AdmissionError`
- Tests for `admit_run` (Strict, Journaled, Relaxed)
- Tests for `check_capability` (granted, denied, hierarchical grants)

---

## Architecture Boundaries

1. **`vb_core::budget`** — Pure, deterministic budget value types, checked arithmetic, policy validation. No runtime, storage, HTTP, JSON, YAML, or allocation-heavy config parsing dependencies.

2. **`vb_core::workflow`** — `CompiledWorkflow` exposes `resource_contract()` and `to_parts()`. `ResourceContract` structurally covers nodes, slots, constants, expressions, accessors, expression stack, fanout, and output bytes.

3. **`vb_core::validation::resource`** — Structural validation layer for resource contracts. Does NOT have runtime dependencies.

4. **`vb_runtime::admission`** — Performs admission decisions against artifact presence, capabilities, and aggregate capacity. Uses core budget/domain types.

5. **`vb_runtime::shard::types`** — May carry capacity snapshots or reservation state but uses core budget/domain types (not parallel budget dimensions).

---

## Open Questions (from contract.md)

1. Should aggregate capacity be configurable only through `ShardConfig`, or also through a runtime-level policy distributed evenly across shards?
2. Should `RunAdmission` store the exact granted aggregate budget for audit/journal replay, or store only digest/run/capabilities/policy and rely on recomputation?
3. Should `max_step_budget_per_tick` contribute to aggregate capacity, or remain an execution throttle separate from admission capacity?
4. Should result bytes and journal batch bytes be reserved pessimistically at admission or checked at write boundaries only?

---

## Risk Tags

| Tag | Description |
|-----|-------------|
| `resource-safety` | Aggregate budgets control run admission; overflow/underflow can cause unbounded resource usage |
| `overflow` | Checked arithmetic required; `checked_add`/`checked_sub` already used in budget.rs |
| `underflow` | Release/subtraction must not go negative |
| `capacity-comparison` | `requested <= available` admits; `requested > available` rejects; equality admits |
| `reservation-lifecycle` | Admission reserves, release must be exact and idempotent-safe |
| `partial-admission-leak` | If reservation succeeds but subsequent admission fails, rollback must be correct |
| `shard-scope` | Per-shard capacity is source of truth; global reporting sums shard snapshots |

---

## Recommended Downstream Owners

- **Contract/Proof**: `rust-contract` skill, `proof-planner` skill — for proof obligation ledger
- **Test**: `test-planner` skill, `test-writer` skill — for integration tests for budget enforcement
- **Implementation**: `holzman-rust` skill — for NASA/JPL Power-of-Ten enforcement (no unsafe, unwrap, panic, todo)
- **Verification**: `formal-verifier` skill — for type-level resource boundedness proof
- **Review**: `black-hat-reviewer` skill — for adversarial review of overflow/underflow paths

---

## Verification Artifacts Present

| Artifact | Path |
|----------|------|
| Contract | `.beads/vb-qi37.2.1/contract.md` |
| Baseline Report | `.beads/vb-qi37.2.1/baseline-report.md` |
| Implementation (partial) | `.beads/vb-qi37.2.1/implementation.md` |
| Lean Contract | `.beads/vb-qi37.2.1/lean-contract.md` |
| Proof Obligations | `.beads/vb-qi37.2.1/proof-obligations.jsonl` |
| Test Plan | `.beads/vb-qi37.2.1/test-plan.md` |
| Verification Layers | `.beads/vb-qi37.2.1/verification-layers.md` |
| Traceability Matrix | `.beads/vb-qi37.2.1/traceability-matrix.jsonl` |
| Red Phase | `.beads/vb-qi37.2.1/red-phase.md` |
| Test Plan Review | `.beads/vb-qi37.2.1/test-plan-review.md` |
| Martin Fowler Tests | `.beads/vb-qi37.2.1/martin-fowler-tests.md` |
| Manual QA Smoke | `.beads/vb-qi37.2.1/manual-qa-smoke.md` |

---

## Excluded/Out-of-Scope

- Production code edits (this is artifact-writing scout phase)
- YAML, JSON, HTTP, or CLI parsing for aggregate budgets
- Generated Rust lowering for the aggregate model
- Distributed/multi-server global capacity coordination
- Performance claims without benchmark evidence
- Replacing existing artifact/capability admission behavior (only composing budget checks with it)
