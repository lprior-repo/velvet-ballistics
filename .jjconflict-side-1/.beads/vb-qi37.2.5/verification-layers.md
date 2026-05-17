# Verification Layers — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 3 (Contract and type model)
- **Scope**: Adversarial boundedness testing for vb_core budget, value store, limits, engine

---

## Boundary

- **Verus-owned kernel**: `StepBudget::remaining` invariant, `ValueStore` arena cap enforcement,
  `count_total_steps` overflow boundedness, monotonic `try_take` decrease
- **TLA+ temporal model**: None — deterministic bounded loop, no concurrent/temporal behavior
- **Theorem projection**: None — no algebraic kernels beyond Verus scope
- **Runtime shell**: `run_until_blocked` iteration, `EngineSignal` matching, `StepBudget::new` clamping
- **External systems excluded**: None — no I/O, networking, or DB in boundedness scope

---

## Layer Assignment

| Contract Clause | Primary Verifier | Complementary Evidence |
|----------------|-----------------|----------------------|
| INV-001 (StepBudget bounds) | Verus | Kani harness, proptest |
| INV-002 (ValueStore cap) | Verus | Kani harness, miri |
| INV-003 (count_total_steps bound) | Verus | Kani harness |
| INV-004 (run_until_blocked terminates) | Verus (loop invariant) | Kani |
| INV-005 (budget monotonic) | Verus | proptest |
| INV-006 (try_take monotonic) | Verus | Kani |
| PRE-001 (StepBudget::new clamp) | Verus | Kani |
| PRE-002 (ValueStore cap enforcement) | Verus | Kani + miri |
| PRE-003 (entry bounds check) | Verus (compile-time) | unit test |
| POST-001 (try_take count) | Verus | Kani |
| POST-002 (new clamps) | Verus | unit test |
| POST-003 (StepBudgetExhausted signal) | unit test | — |
| POST-004 (CoreError::BudgetExceeded) | Verus | Kani |
| POST-005 (WorkflowError on overflow) | Verus | Kani |
| POST-006 (BoundednessPolicy validate) | unit test | — |
| ERR (error taxonomy) | compile-time exhaustive match | unit test |

---

## Verus Scope

### Target Modules
- `crates/vb_core/src/engine/signals.rs` — `StepBudget`
- `crates/vb_core/src/value_store.rs` — `ValueStore` arena cap
- `crates/vb_core/src/budget.rs` — `WholeWorkflowBudget::compute`, `count_total_steps`
- `crates/vb_core/src/engine/run_loop.rs` — `run_until_blocked` termination

### Spec/Proof Functions
- `spec_step_budget_invariant` — `remaining <= MAX_STEP_BUDGET` always
- `spec_try_take_decreases` — `remaining` decreases by exactly 1 on `Ok(true)`, unchanged on `Ok(false)`
- `spec_value_store_cap` — `total_arena_count <= max_arena_entries` always
- `spec_count_total_steps_bounded` — result ≤ `MAX_STEPS_PER_WORKFLOW` or error
- `spec_run_until_blocked_terminates` — loop terminates in ≤ `initial_budget` iterations

### Invariants
- INV-001, INV-002, INV-003, INV-004, INV-005, INV-006

### Trusted Boundary
- `StepBudget::new` and `StepBudget::MAX` constructors (clamped)
- `ValueStore::with_max_slots` constructor (cap set once)
- `WholeWorkflowBudget::compute` (Result-returning, no panics)

### Shell Exclusions
- No I/O, no async scheduling, no storage, no wall-clock time

---

## Kani Scope

### Target Harnesses
- `crates/vb_core/src/kani/step_budget.rs` — StepBudget bounds and try_take behavior
- `crates/vb_core/src/kani/run_until_blocked.rs` — run_until_blocked loop termination
- `crates/vb_core/src/kani/value_store_cap.rs` — ValueStore arena cap enforcement

### Properties
- `StepBudget::try_take` never returns error through normal API use (overflow guard unreachable via API)
- `run_until_blocked` returns `StepBudgetExhausted` exactly when budget depletes
- `ValueStore` insert operations return error before exceeding cap

### Model Shape
- Concrete `StepBudget` construction with arbitrary `u64` input
- Concrete `ValueStore::with_max_slots` with `u16` cap
- Loop unrolling bounded by `MAX_STEP_BUDGET`

---

## Miri Scope

### Targets
- `ValueStore` insert operations — no undefined behavior, no use-after-free
- `StepBudget::try_take` — no UB in `saturating_sub`
- Raw pointer handling in `ObjectField` / `SlotValue` if any

### Command
- `cargo miri test --package vb_core`
- Focus on `signals::tests`, `value_store::tests`

---

## Proptest Scope

### Properties
- `StepBudget::new(v).remaining == min(v, MAX_STEP_BUDGET)` for all `u64` v
- `StepBudget::try_take` called N times returns `Ok(true)` exactly `min(N, initial)` times
- `ValueStore::insert_*` returns error when `total_arena_count >= max_arena_entries`
- `BoundednessPolicy::DEFAULT.validate(compute(...))` is deterministic

---

## Fuzz Scope

### Target
- `fuzz/src/lib.rs::fuzz_resource_budget` already exists
- Fuzz `StepBudget::new(u64)` with arbitrary u64 values (clamping boundary)
- Fuzz `ValueStore::insert_*` sequences up to `max_arena_entries` cap

---

## Unit Test Scope (Compile-time coverage)

### Targets
- `crates/vb_core/src/budget/tests.rs` (3396 lines)
- `crates/vb_core/src/engine/tests/integration_budget.rs` (184 lines)
- `crates/vb_validate/src/type_taint_tests.rs` (blackhat tests)
- `crates/velvet_ballastics/tests/cross_crate_adversarial.rs` (1538 lines)

---

## Waiver

- **TLA+**: No temporal/concurrent/actor behavior in scope — `run_until_blocked` is a
  deterministic bounded loop; termination is proven by Verus loop invariant, not model checking.
  Owner: State 3. Reason: Single-threaded deterministic loop; no liveness/deadlock/fairness
  concerns. Compensating evidence: Verus INV-004 loop invariant + Kani harness.

---

## Verification Coverage Summary

| Layer | Obligations | Primary Target |
|-------|------------|----------------|
| Verus | 6 invariants, 3 preconditions, 2 postconditions | budget.rs, signals.rs, value_store.rs |
| Kani | 3 harnesses | run_loop.rs, signals.rs, value_store.rs |
| Miri | 1 package | value_store.rs |
| Proptest | 4 property tests | budget.rs, signals.rs, value_store.rs |
| Fuzz | 1 existing target | fuzz/src/lib.rs |
| Unit test | 4 test suites (existing) | budget, engine, validation, integration |
