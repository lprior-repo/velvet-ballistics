# Codebase Map — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 2 (Explore and scope)
- **Goal**: Map smallest relevant scope for adversarial boundedness tests

## Isolation
- **Source checkout**: /home/lewis/src/Velvet-ballistics
- **Isolated workspace**: /home/lewis/src/vb-qi37-2-5
- **Workspace type**: git-worktree

## Pre-existing DEFERRED_GLOBAL
- vb_runtime build failure: `crates/vb_runtime/src/runtime.rs:4` includes `runtime/chunk_001.rs` which does not exist.
  - This is OUTSIDE this bead's scope and must not block State 2.
  - Classification: DEFERRED_GLOBAL

---

## Touched Crates

| Crate | Role |
|-------|------|
| `vb_core` | Core runtime: budget, value store, limits, engine |
| `vb_validate` | Validation: type_taint resource contract checks |
| `velvet_ballastics` (root) | Integration tests: adversarial cross-crate tests |
| `fuzz` | Fuzz targets: resource budget fuzzing |
| `workspace_tests` | Bench integration tests |
| `vb_runtime` | DEFERRED_GLOBAL (build failure, not in scope) |

---

## Key Files and Symbols

### Budget Core (`crates/vb_core/src/budget.rs`)
- `WholeWorkflowBudget` struct — computed budget for entire workflow
- `WholeWorkflowBudget::compute()` — walks compiled IR, computes all budget dimensions
- `BoundednessPolicy` struct — policy limits for validation
- `BoundednessPolicy::DEFAULT` — conservative default policy (max_total_steps: 1_000_000, max_fanout: 64, etc.)
- `BoundednessPolicy::validate()` — validates computed budget against policy
- `AggregateResourceBudget` struct — aggregate run-level budget
- `AggregateResourceUsage`, `AggregateReservation`, `AggregateResourceCapacity`
- `validate_step_ceilings()` — hard limits for step budget per tick (1_000_000)
- `count_total_steps()` — DFS walk counting worst-case steps with loop iteration multiplication
- Error types: `BudgetError`, `AggregateBudgetError`

### Limits (`crates/vb_core/src/limits.rs`)
- `MAX_STEPS_PER_WORKFLOW: usize = 65_535`
- `MAX_VALUES_PER_RUN: usize = 1_000_000` — cap on total arena values
- `MAX_STEP_BUDGET: u64 = 10_000` — max deterministic transitions per tick
- `MAX_LIST_ITEMS_PER_VALUE: usize = 65_535`
- `MAX_OBJECT_FIELDS_PER_VALUE: usize = 65_535`
- `MAX_BLOB_BYTES_PER_VALUE: usize = 16_777_216`
- `MAX_LANGUAGE_NESTING_DEPTH: u8 = 8`
- `MAX_SLOTS: u16 = u16::MAX`

### ValueStore (`crates/vb_core/src/value_store.rs`)
- `ValueStore` struct — cold arenas for symbols, lists, objects, blobs
- `ValueStore::with_max_slots(u16)` — creates store with hard cap on total arena entries
- `check_arena_cap()` — enforces max_arena_entries cap
- `CoreError::BudgetExceeded { budget: "max_slots", limit }` — error when cap exceeded
- Per-type validation: `validate_list_len`, `validate_symbol_len`, `validate_blob_len`, `validate_object_len`

### Engine (`crates/vb_core/src/engine/`)
- `StepBudget` struct — per-tick step budget counter
- `StepBudget::new(u64)`, `StepBudget::try_take()`, `StepBudget::remaining()`
- `run_until_blocked()` — deterministic run loop
- `EngineSignal::StepBudgetExhausted` — signal when budget depleted
- `new_run_frame()` — creates run frame

### Validation (`crates/vb_validate/src/type_taint.rs`)
- `ResourceContract::validate()` — validates resource contract against hard limits
- `max_step_budget_per_tick` field and validation
- `hard_limits` validation (MAX_STEP_BUDGET_PER_TICK: 1_000_000)

### Existing Tests
- `crates/vb_core/src/budget/tests.rs` (3396 lines) — budget computation tests
- `crates/vb_core/src/engine/tests/integration_budget.rs` (184 lines) — step budget integration tests
- `crates/vb_validate/src/type_taint_tests.rs` — blackhat tests: `blackhat_zero_max_step_budget_per_tick_rejected`, `blackhat_max_step_budget_per_tick_exceeding_hard_limit_rejected`
- `fuzz/src/lib.rs::fuzz_resource_budget` — fuzz target for resource budget exhaustion
- `crates/velvet_ballastics/tests/cross_crate_adversarial.rs` (1538 lines) — adversarial integration tests

### Verification Artifacts
- `verification/verus/resource_budget.rs` — Verus specs for budget composition lemmas
- `verification/verus/step_budget.rs` — Verus proof obligations for step budget
- `kani/` — Kani harnesses: gate_07_stack, gate_08_accessor, gate_09_slots, gate_10_node, gate_11_loop, gate_12_14_15

---

## Risk Tags
- `boundedness` — primary concern
- `performance` — worst-case execution time
- `user-visible-behavior` — failures are user-visible typed errors
- `persistence` — value store arena growth
- `public-api` — StepBudget, run_until_blocked, ValueStore::with_max_slots are public APIs

---

## Required Verifier Modes
1. **Kani** — bounded model checking for step budget exhaustion, run_until_blocked behavior
2. **Miri** — undefined behavior checks for value store handle access
3. **Proptest** — property-based tests for budget computation, loop iteration multiplication

---

## Public API Surface for Boundedness

### vb_core
```rust
// StepBudget - per-tick budget
pub struct StepBudget { ... }
impl StepBudget {
    pub fn new(u64) -> Self
    pub fn try_take(&mut self) -> Result<bool, CoreError>
    pub fn remaining(&self) -> u64
    pub const MAX: u64  // if exists
}

// ValueStore - arena with cap
impl ValueStore {
    pub fn with_max_slots(max_slots: u16) -> Self
    pub fn insert_list(...) -> CoreResult<ListId>
    pub fn insert_object(...) -> CoreResult<ObjectId>
    pub fn insert_symbol(...) -> CoreResult<SymbolId>
    pub fn insert_blob(...) -> CoreResult<BlobId>
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
pub fn run_until_blocked(workflow: &CompiledWorkflow, run: &mut RunFrame, budget: &mut StepBudget, store: &mut ValueStore) -> Result<EngineSignal, EngineError>
```

### vb_validate
```rust
pub struct ResourceContract {
    pub max_step_budget_per_tick: usize,
    pub max_slots: u16,
    pub max_output_bytes: u32,
    // ...
}
impl ResourceContract {
    pub fn validate(&self) -> ValidationResult<()>
}
```

---

## Open Questions / Blockers
1. **vb_runtime build failure (DEFERRED_GLOBAL)**: `runtime/chunk_001.rs` missing — does not affect vb_core boundedness tests but blocks full workspace build.
2. **vb-qi37.2.2 dependency**: Value arena caps feature is OPEN — adversarial tests should assume caps exist and verify fail-closed behavior.
3. **vb-qi37.2.4 dependency**: Nested composition bounds is OPEN — tests may need to cover verifier rejection of unbounded compositions.

---

## Recommended Downstream Owners
- `rust-contract` → contract.md for boundedness contract clauses
- `proof-planner` → proof strategy for Kani bounded model checking
- `test-planner` → test plan for adversarial BDD scenarios
- `holzman-rust` → implementation of any new test fixtures
