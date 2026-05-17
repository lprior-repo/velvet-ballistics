# Contract Specification — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 3 (Contract and type model)
- **Scope**: Adversarial boundedness testing for vb_core budget, value store, limits, and engine

## Context

### Feature Domain
Boundedness enforcement in Velvet-ballistics ensures workflows cannot exceed
resource limits (step budget, arena slots, value counts, nesting depth). Any
violation must result in a typed error returned to the caller — never a panic,
hang, or resource leak.

### Domain Terms
| Term | Definition |
|------|-----------|
| `StepBudget` | Per-tick deterministic transition counter; `try_take` returns `Ok(false)` at 0 |
| `WholeWorkflowBudget` | Compile-time-computed worst-case budget across all IR paths |
| `BoundednessPolicy` | Validation policy for computed budgets against hard limits |
| `ValueStore` | Cold arena for symbols, lists, objects, blobs with a `max_arena_entries` cap |
| `MAX_STEP_BUDGET` | Hard ceiling = 10_000; `StepBudget::new` clamps any value above it |
| `MAX_VALUES_PER_RUN` | Cap on total arena values = 1_000_000 |
| `MAX_LANGUAGE_NESTING_DEPTH` | Compile-time max nesting depth = 8 |
| `EngineSignal::StepBudgetExhausted` | Signal when `run_until_blocked` depletes budget |

### Assumptions
1. `StepBudget::new(v)` always produces a valid budget ≤ `MAX_STEP_BUDGET` (clamped)
2. `run_until_blocked` always terminates: either `EngineSignal::Continue` + terminal node,
   or `StepBudgetExhausted`, or an error — never an infinite loop with budget available
3. `ValueStore::with_max_slots` enforces the cap; no overflow beyond `u16::MAX` arena entries
4. `WholeWorkflowBudget::compute` traverses the IR correctly and does not overflow `u64` accumulators
5. `count_total_steps` uses worst-case loop iteration multiplication but is bounded by `MAX_STEPS_PER_WORKFLOW`

### Open Questions
1. **vb-qi37.2.4 (nested composition bounds)**: If not resolved, adversarial tests must
   assume bounded loop iteration counts and fail-closed on unbounded detection
2. **vb-qi37.2.2 (value arena caps)**: If not resolved, adversarial tests assume
   `ValueStore::with_max_slots` cap is enforced and verify fail-closed behavior
3. **vb_runtime build (DEFERRED_GLOBAL)**: `chunk_001.rs` missing — does not affect
   vb_core boundedness scope

---

## Preconditions

- **PRE-001**: `StepBudget::new(v)` accepts any `u64` and clamps to `MAX_STEP_BUDGET` without panicking
- **PRE-002**: `ValueStore::with_max_slots(max: u16)` creates a store where insertions beyond `max_arena_entries` return `CoreError::BudgetExceeded`
- **PRE-003**: `WholeWorkflowBudget::compute(nodes, entry, contract)` requires `entry < nodes.len()`
- **PRE-004**: `run_until_blocked(plan, run, budget, store)` requires `budget.remaining >= 0` (always true by construction)

---

## Postconditions

- **POST-001**: `StepBudget::try_take()` returns `Ok(true)` exactly `initial_value` times, then `Ok(false)` thereafter; never panics
- **POST-002**: `StepBudget::new(v)` where `v > MAX_STEP_BUDGET` returns `StepBudget { remaining: MAX_STEP_BUDGET }`
- **POST-003**: `run_until_blocked` returns `EngineSignal::StepBudgetExhausted` when budget hits zero before workflow completion
- **POST-004**: `ValueStore::insert_*` returns `CoreError::BudgetExceeded` when total arena count would exceed `max_arena_entries`
- **POST-005**: `WholeWorkflowBudget::compute` returns `WorkflowError` when `count_total_steps` exceeds `MAX_STEPS_PER_WORKFLOW`
- **POST-006**: `BoundednessPolicy::validate` returns `Err` when any budget dimension exceeds its policy limit

---

## Invariants

- **INV-001**: `StepBudget::remaining` is always in `[0, MAX_STEP_BUDGET]` — never exceeds hard ceiling
- **INV-002**: `ValueStore::total_arena_count() <= max_arena_entries` at all times after construction
- **INV-003**: `count_total_steps` result for any IR is bounded by `MAX_STEPS_PER_WORKFLOW` (overflow returns `WorkflowError`)
- **INV-004**: `run_until_blocked` loop terminates in at most `budget.remaining` iterations — no infinite loop with available budget
- **INV-005**: `WholeWorkflowBudget` fields are all non-decreasing across multiple `compute` calls (monotonic accumulation)
- **INV-006**: `StepBudget::try_take` is the only mutator of `remaining`; it always decreases monotonically

---

## Error Taxonomy

All fallible operations return `Result<T, Error>`:

| Error | Variant | When |
|-------|---------|------|
| `CoreError::BudgetExceeded` | Budget limit hit | ValueStore arena cap exceeded |
| `EngineError::StepCounterOverflow` | Invariant violation | `StepBudget::remaining > MAX_STEP_BUDGET` |
| `EngineError::StepBudgetExhausted` | Budget depleted | `try_take` called with 0 remaining |
| `WorkflowError::BudgetTooLarge` | Budget policy violation | `WholeWorkflowBudget` exceeds policy |
| `WorkflowError::NestingDepthExceeded` | Depth limit hit | AST nesting > `MAX_LANGUAGE_NESTING_DEPTH` |
| `WorkflowError::StepCountOverflow` | Step count overflow | `count_total_steps` would exceed `MAX_STEPS_PER_WORKFLOW` |
| `WorkflowError::EntryOutOfBounds` | Invalid entry index | `entry >= nodes.len()` |

---

## Contract Signatures (Public API)

```rust
// StepBudget — per-tick budget
pub struct StepBudget { remaining: u64 }
impl StepBudget {
    pub const MAX: Self
    pub const fn new(value: u64) -> Self
    pub fn try_take(&mut self) -> Result<bool, EngineError>
    pub fn remaining(&self) -> u64
}

// ValueStore — arena with cap
impl ValueStore {
    pub fn with_max_slots(max_slots: u16) -> Self
    pub fn insert_list(...) -> CoreResult<ListId>
    pub fn insert_object(...) -> CoreResult<ObjectId>
    pub fn total_arena_count(&self) -> u64
    pub fn max_arena_entries(&self) -> u64
}

// Budget computation
pub struct WholeWorkflowBudget { ... }
impl WholeWorkflowBudget {
    pub fn compute(nodes: &[CompiledNode], entry: StepIdx, contract: &ResourceContract) -> Result<Self, WorkflowError>
}

pub struct BoundednessPolicy { ... }
impl BoundednessPolicy {
    pub const DEFAULT: Self
    pub fn validate(&self, budget: &WholeWorkflowBudget) -> Result<(), BudgetError>
}

// Run loop
pub fn run_until_blocked(plan: &CompiledWorkflow, run: &mut RunFrame, budget: StepBudget, store: &mut ValueStore) -> Result<EngineSignal, EngineError>
```

---

## TLA+-Owned Clauses

Boundedness for this bead is primarily **Rust-local pure/core logic** — no distributed
protocols, schedulers, or temporal liveness properties are in scope. `run_until_blocked`
is a deterministic loop with a hard iteration bound (`budget.remaining`), so no TLA+
model is needed for the boundedness contract.

**Non-applicability rationale**: The workflow engine is single-threaded and deterministic.
Termination is guaranteed by the `budget.remaining` counter. No concurrent actors,
no message passing, no eventual liveness — only a bounded number of synchronous
transition firings.

---

## Verus-Owned Clauses

All Rust-local pure boundedness invariants are owned by Verus:

- **INV-001**: `StepBudget::remaining in [0, MAX_STEP_BUDGET]` — monotonic decrease, never overflows
- **INV-002**: `ValueStore::total_arena_count <= max_arena_entries` — cap enforcement
- **INV-003**: `count_total_steps` boundedness — no u64 overflow in worst-case step accumulation
- **INV-006**: `StepBudget::try_take` monotonic decrease invariant

---

## Non-goals

- Temporal/liveness properties of multi-step workflows (no TLA+ needed for this bead)
- vb_runtime `chunk_001.rs` build failure (DEFERRED_GLOBAL, outside scope)
- Byzantine/fuzzing adversarial input beyond ValueStore arena bounds
- Performance benchmarking (separate bead)
