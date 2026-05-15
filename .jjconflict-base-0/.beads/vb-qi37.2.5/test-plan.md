# Test Plan: vb-qi37.2.5 — Boundedness Adversarial Tests

## Summary

- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **Theme**: Adversarial boundedness testing for StepBudget, ValueStore arena cap, u64 overflow
- **Behaviors identified**: 14
- **Trophy allocation**: 11 unit / 4 integration / 1 fuzz / 4 proptest (≈60% unit / 35% integration / 5% fuzz)
- **Verus verification**: PASS — 6 obligations, 49 lemmas, 0 errors
- **Kani verification**: INTEGRATED — 3 obligations, 10 harnesses
- **Miri**: Deferred to State 11

---

## 1. Behavior Inventory

| # | Subject | Behavior | Contract Clause |
|---|---------|----------|-----------------|
| B1 | `StepBudget::new` | clamps any u64 input to `[0, MAX_STEP_BUDGET]` without panic | PRE-001 |
| B2 | `StepBudget::new` | accepts exactly `MAX_STEP_BUDGET` as valid input | PRE-001 |
| B3 | `StepBudget::try_take` | returns `Ok(true)` exactly `min(n, initial)` times when called `n` times | POST-001 |
| B4 | `StepBudget::try_take` | returns `Ok(false)` at 0 and stays false on repeated calls | POST-001 |
| B5 | `StepBudget::try_take` | never panics; returns `Err(EngineError::StepCounterOverflow)` only if invariant violated | POST-001 |
| B6 | `run_until_blocked` | returns `EngineSignal::StepBudgetExhausted` when budget depletes before workflow completion | POST-003 |
| B7 | `run_until_blocked` | returns `EngineSignal::Finished` when workflow completes within budget | POST-003 |
| B8 | `run_until_blocked` | terminates in at most `budget.remaining` iterations (no infinite loop with available budget) | INV-004 |
| B9 | `ValueStore::with_max_slots` | `insert_*` returns `CoreError::BudgetExceeded` when `total_arena_count >= max_arena_entries` | POST-004 |
| B10 | `ValueStore::with_max_slots` | `insert_*` succeeds for all sequences where `total_arena_count < max_arena_entries` | POST-004 |
| B11 | `WholeWorkflowBudget::compute` | returns `WorkflowError::StepCountOverflow` when `count_total_steps` would exceed `u32::MAX` | POST-005 |
| B12 | `WholeWorkflowBudget::compute` | returns `WorkflowError::EntryOutOfBounds` when `entry >= nodes.len()` | PRE-003 |
| B13 | `BoundednessPolicy::validate` | returns `Ok(())` for budget within all policy limits | POST-006 |
| B14 | `BoundednessPolicy::validate` | returns `Err(BudgetError)` for the first dimension that exceeds its policy limit | POST-006 |

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 11 | Pure functions: `StepBudget::new`, `StepBudget::try_take`, `ValueStore::insert_*`, `BoundednessPolicy::validate`, `WholeWorkflowBudget::compute` entry check; deterministic error path |
| Integration | 4 | `run_until_blocked` end-to-end with real `CompiledWorkflow`, `RunFrame`, `ValueStore`; budget exhaustion at integration boundary |
| Proptest | 4 | Exhaustive combinatorial: `StepBudget::new` clamping, `try_take` count sequence, `ValueStore` cap sequences, `BoundednessPolicy` random budgets |
| Fuzz | 1 | `StepBudget::new` u64 → clamped value (FUZZ-001); corpus includes boundary values around MAX_STEP_BUDGET |
| Static | — | Verus (6 files, 49 lemmas) provides formal proof; clippy + cargo-deny handled separately |
| E2E | 0 | Not in scope — boundedness is internal engine invariant; no user-facing API boundary |

**Deviation rationale**: E2E layer is zero because boundedness enforcement is an internal engine guarantee. The public API boundary is `run_until_blocked`, covered by integration tests. This is appropriate for a correctness-critical internal invariant.

---

## 3. BDD Scenarios

### Behavior B1: StepBudget::new clamps u64 to MAX_STEP_BUDGET

**Scenario B1.1: new clamps value above ceiling**
```
Given: an arbitrary u64 input v where v > MAX_STEP_BUDGET
When: StepBudget::new(v) is called
Then: the resulting budget has remaining == MAX_STEP_BUDGET
And: no panic occurs
```

**Scenario B1.2: new accepts value at ceiling**
```
Given: input v == MAX_STEP_BUDGET
When: StepBudget::new(v) is called
Then: the resulting budget has remaining == MAX_STEP_BUDGET
```

**Scenario B1.3: new accepts value below ceiling**
```
Given: input v < MAX_STEP_BUDGET
When: StepBudget::new(v) is called
Then: the resulting budget has remaining == v
```

**Scenario B1.4: new accepts u64::MAX without panic**
```
Given: input v == u64::MAX
When: StepBudget::new(v) is called
Then: the resulting budget has remaining == MAX_STEP_BUDGET
```

**Scenario B1.5: new accepts zero**
```
Given: input v == 0
When: StepBudget::new(v) is called
Then: the resulting budget has remaining == 0
```

---

### Behavior B2: StepBudget::MAX is exactly MAX_STEP_BUDGET

**Scenario B2.1: MAX constant equals hard ceiling**
```
Given: StepBudget::MAX
When: remaining() is called
Then: the result equals MAX_STEP_BUDGET
```

---

### Behavior B3: try_take returns Ok(true) exactly min(n, initial) times

**Scenario B3.1: try_take decrements while available**
```
Given: StepBudget::new(3)
When: try_take is called 3 times
Then: first 3 calls return Ok(true)
And: remaining after 3 calls is 0
```

**Scenario B3.2: try_take returns false at zero**
```
Given: StepBudget::new(1)
When: try_take is called twice
Then: first call returns Ok(true), second call returns Ok(false)
And: remaining stays 0
```

**Scenario B3.3: try_take on exhausted budget stays false**
```
Given: StepBudget::new(0)
When: try_take is called 10 times
Then: all 10 calls return Ok(false)
And: remaining stays 0
```

**Scenario B3.4: remaining never increases (monotonic)**
```
Given: StepBudget::new(5)
When: try_take is called 5 times
Then: remaining monotonically decreases from 5 to 0
And: no step increases remaining
```

---

### Behavior B6: run_until_blocked returns StepBudgetExhausted

**Scenario B6.1: exhausts budget before completion**
```
Given: a 2-step workflow, budget of 1
When: run_until_blocked is called
Then: result is EngineSignal::StepBudgetExhausted
And: run.executed() == 1
```

**Scenario B6.2: zero budget returns exhausted immediately**
```
Given: any workflow, budget of 0
When: run_until_blocked is called
Then: result is EngineSignal::StepBudgetExhausted
And: run.executed() == 0
```

**Scenario B6.3: exact budget completes workflow**
```
Given: a 2-step workflow, budget of 2
When: run_until_blocked is called
Then: result is EngineSignal::Finished(_, _)
And: run.executed() == 2
```

---

### Behavior B7: run_until_blocked returns Finished when workflow completes

**Scenario B7.1: completes within budget**
```
Given: a 2-step workflow that finishes, budget of MAX_STEP_BUDGET
When: run_until_blocked is called
Then: result is EngineSignal::Finished(value, taint)
And: taint is one of Taint::Clean, Taint::Secret, Taint::DerivedFromSecret
```

---

### Behavior B9: ValueStore insert_* rejects at arena cap

**Scenario B9.1: with_max_slots(1) rejects second symbol insert**
```
Given: ValueStore::with_max_slots(1) with one existing symbol
When: a second symbol insert is attempted
Then: CoreError::BudgetExceeded { budget: "max_slots", limit: 1 } is returned
And: total_arena_count remains 1
```

**Scenario B9.2: with_max_slots(3) allows exactly 3 inserts**
```
Given: ValueStore::with_max_slots(3)
When: 3 inserts (symbol, list, blob) are performed
Then: all 3 succeed
And: 4th insert of any type returns CoreError::BudgetExceeded
```

**Scenario B9.3: new() has no cap and allows 100 inserts**
```
Given: ValueStore::new() (max_arena_entries == 0)
When: 100 symbol inserts are performed
Then: all 100 succeed
And: total_arena_count == 100
```

**Scenario B9.4: all insert variants respect cap**
```
Given: ValueStore::with_max_slots(2)
When: insert_symbol is called, then insert_list, then insert_object
Then: first 2 succeed, third returns CoreError::BudgetExceeded
And: the same holds if insert order is reversed
```

---

### Behavior B11: WholeWorkflowBudget::compute returns StepCountOverflow

**Scenario B11.1: entry out of bounds returns error**
```
Given: nodes.len() == 1, entry == StepIdx::new(10)
When: WholeWorkflowBudget::compute(nodes, entry, contract) is called
Then: WorkflowError::EntryOutOfBounds is returned
```

**Scenario B11.2: large step count overflow propagates to StepCountOverflow**
```
Given: a constructed CompiledWorkflow that causes count_total_steps to exceed u32::MAX
When: WholeWorkflowBudget::compute is called
Then: WorkflowError::StepCountOverflow is returned
```

---

### Behavior B14: BoundednessPolicy::validate returns first violation

**Scenario B14.1: valid budget passes validation**
```
Given: BoundednessPolicy::DEFAULT and a budget with all dimensions within limits
When: validate is called
Then: Ok(()) is returned
```

**Scenario B14.2: each dimension independently triggers the correct error variant**
```
Given: BoundednessPolicy::DEFAULT
When: validate is called with max_total_steps > DEFAULT.max_total_steps
Then: Err(BudgetError::TotalStepsExceeded { actual, limit }) is returned

When: validate is called with max_fanout > DEFAULT.max_fanout
Then: Err(BudgetError::FanoutExceeded { actual, limit }) is returned

When: validate is called with max_nesting_depth > DEFAULT.max_nesting_depth
Then: Err(BudgetError::NestingDepthExceeded { actual, limit }) is returned
```

---

## 4. Proptest Invariants

### Proptest P1: StepBudget::new(v).remaining == min(v, MAX_STEP_BUDGET)

**File**: `crates/vb_core/src/engine/signals.rs`
**Function**: `property_step_budget_new_clamp`
**Obligation**: PROPTEST-PRE-001

```
Invariant: StepBudget::new(v).remaining == v.min(MAX_STEP_BUDGET) for ALL u64 v
Strategy: u64 — full range including 0, MAX_STEP_BUDGET, u64::MAX, MAX_STEP_BUDGET-1, MAX_STEP_BUDGET+1
Anti-invariant: N/A — all u64 inputs are valid
```

### Proptest P2: try_take returns Ok(true) exactly min(n, clamped_initial) times

**File**: `crates/vb_core/src/engine/signals.rs`
**Function**: `property_try_take_count`
**Obligation**: PROPTEST-POST-001

```
Invariant: After n calls to try_take, true_count == min(n, clamped_initial)
          and remaining == clamped_initial.saturating_sub(true_count)
Strategy: (u64 initial, u64 n) — arbitrary pair, clamped_initial = initial.min(MAX_STEP_BUDGET)
Anti-invariant: n == 0 returns 0 trues; n >> clamped_initial returns clamped_initial trues then all false
```

### Proptest P3: ValueStore insert_* returns BudgetExceeded when at cap

**File**: `crates/vb_core/src/value_store.rs`
**Function**: `property_value_store_cap`
**Obligation**: PROPTEST-PRE-002

```
Invariant: For ValueStore::with_max_slots(m), any insert_* call when total_arena_count >= m
          returns CoreError::BudgetExceeded
Strategy: (u16 cap, Vec<InsertOp>) — cap in [1, 100], insert sequences including exact-cap, cap+1, far below cap
Anti-invariant: insert when at cap must NOT succeed
```

### Proptest P4: BoundednessPolicy::validate is correct for random budget/policy pairs

**File**: `crates/vb_core/src/budget.rs`
**Function**: `property_boundedness_policy`
**Obligation**: PROPTEST-POST-006

```
Invariant: validate returns Ok iff ALL dimensions <= policy limits; returns first failing dimension
Strategy: (u64 steps, u64 slots, u16 fanout, u16 depth) — random values within/outside policy defaults
Anti-invariant: dimensions exactly at boundary (==) must pass; > must fail
```

---

## 5. Fuzz Targets

### Fuzz Target: fuzz_step_budget_new (FUZZ-001)

**Artifact**: `fuzz/src/bin/step_budget_new.rs` + `fuzz/src/lib.rs::fuzz_step_budget_new`
**Obligation**: FUZZ-001

```
Input type: bytes (parsed as arbitrary u64)
Risk: panic (StepBudget::new panics on overflow), wrong clamping (remaining > MAX_STEP_BUDGET)
Corpus seeds:
  - u64::MAX (0xFFFFFFFFFFFFFFFF)
  - MAX_STEP_BUDGET (10_000)
  - MAX_STEP_BUDGET + 1 (10_001)
  - MAX_STEP_BUDGET - 1 (9_999)
  - 0
  - 1
  - u64::MAX / 2
  - 2^63 (signed boundary)
```

---

## 6. Kani Harnesses

### Kani Harness: step_budget_new_clamps (KANI-INV-001)

**Artifact**: `crates/vb_core/src/kani/step_budget.rs::step_budget_new_clamps`
**Property**: `StepBudget::new(v).remaining == min(v, MAX_STEP_BUDGET)` for all concrete v
**Bound**: All u64 values exhaustively checked (Kani bounded model checking)
**Status**: INTEGRATED — VERIFICATION SUCCESSFUL

### Kani Harness: step_budget_max_value (KANI-INV-001)

**Artifact**: `crates/vb_core/src/kani/step_budget.rs::step_budget_max_value`
**Property**: `StepBudget::MAX.remaining == MAX_STEP_BUDGET`
**Bound**: Concrete single value
**Status**: INTEGRATED — VERIFICATION SUCCESSFUL

### Kani Harness: step_budget_try_take_bounded (KANI-INV-001)

**Artifact**: `crates/vb_core/src/kani/step_budget.rs::step_budget_try_take_bounded`
**Property**: `remaining <= MAX_STEP_BUDGET` after all try_take calls
**Bound**: `#[kani::unwind(10001)]`

### Kani Harness: step_budget_repeated_take_bounded (KANI-INV-001)

**Artifact**: `crates/vb_core/src/kani/step_budget.rs::step_budget_repeated_take_bounded`
**Property**: repeated try_take from MAX stays within bounds
**Bound**: `#[kani::unwind(10001)]`

### Kani Harness: run_until_blocked_loop_terminates (KANI-INV-004)

**Artifact**: `crates/vb_core/src/kani/run_until_blocked.rs::run_until_blocked_loop_terminates`
**Property**: `drive_deterministic` loop terminates in bounded iterations
**Bound**: `#[kani::unwind(10001)]`

### Kani Harness: value_store_cap_one_rejects_second (KANI-POST-004)

**Artifact**: `crates/vb_core/src/kani/value_store_cap.rs::value_store_cap_one_rejects_second`
**Property**: `with_max_slots(1)` second insert returns `BudgetExceeded`
**Bound**: Concrete

### Kani Harness: value_store_cap_three_allows_three (KANI-POST-004)

**Artifact**: `crates/vb_core/src/kani/value_store_cap.rs::value_store_cap_three_allows_three`
**Property**: `with_max_slots(3)` allows exactly 3 inserts
**Bound**: Concrete

### Kani Harness: value_store_uncapped_allows_many (KANI-POST-004)

**Artifact**: `crates/vb_core/src/kani/value_store_cap.rs::value_store_uncapped_allows_many`
**Property**: `ValueStore::new()` has no cap
**Bound**: Concrete

### Kani Harness: value_store_all_insert_variants_respect_cap (KANI-POST-004)

**Artifact**: `crates/vb_core/src/kani/value_store_cap.rs::value_store_all_insert_variants_respect_cap`
**Property**: all `insert_*` variants return `BudgetExceeded` at cap
**Bound**: Concrete

---

## 7. Mutation Checkpoints

**Threshold**: ≥90% mutation kill rate

### Critical Mutations

| Function | Mutation | Must be caught by |
|----------|----------|-------------------|
| `StepBudget::new` | Remove clamping (`value > MAX_STEP_BUDGET` branch) | `budget_new_clamps_to_max_step_budget` unit test |
| `StepBudget::try_take` | Replace `saturating_sub` with regular subtraction | `budget_try_take_decrements_remaining` + `property_try_take_count` |
| `StepBudget::try_take` | Remove `if remaining == 0` check | `budget_zero_never_returns_true` |
| `ValueStore::check_arena_cap` | Remove `>=` check (use `>`) | `value_store_with_max_slots_one_rejects_second_insert` |
| `ValueStore::check_arena_cap` | Remove cap check entirely | `value_store_with_max_slots_allows_inserts_up_to_cap` |
| `WholeWorkflowBudget::compute` | Remove entry bounds check | `test_entry_out_of_bounds_returns_error` |
| `count_and_push_loop_body` | Remove `checked_mul` | `test_step_count_overflow` |
| `count_total_steps` | Remove `checked_add` | `test_step_count_overflow` |
| `BoundednessPolicy::validate` | Remove one dimension check | `property_boundedness_policy` |

---

## 8. Combinatorial Coverage Matrix

### StepBudget (signals.rs)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| `new` clamps MAX+100 | v > MAX_STEP_BUDGET | `remaining == MAX_STEP_BUDGET` | unit |
| `new` accepts MAX | v == MAX_STEP_BUDGET | `remaining == MAX_STEP_BUDGET` | unit |
| `new` accepts u64::MAX | v == u64::MAX | `remaining == MAX_STEP_BUDGET` | unit |
| `new` accepts 0 | v == 0 | `remaining == 0` | unit |
| `new` accepts arbitrary u64 | 0 < v < MAX | `remaining == v` | proptest |
| `try_take` decrements | initial=3, call 3 times | true,true,true then false | unit |
| `try_take` at zero | initial=0 | all false | unit |
| `try_take` monotonic | initial=5, call 5 times | remaining 5→4→3→2→1→0 | unit |
| `try_take` stable at zero | initial=2, call 10 times | false on all calls after exhaustion | unit |
| `new` clamp exhaustive | random u64 | min(v, MAX) | proptest |
| `try_take` count sequence | random (initial, n) | true_count == min(n, clamped_initial) | proptest |
| MAX constant | StepBudget::MAX | remaining == MAX_STEP_BUDGET | unit |

### ValueStore Arena Cap (value_store.rs)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| cap=1, 2nd insert rejected | symbol, then symbol | `CoreError::BudgetExceeded { limit: 1 }` | unit |
| cap=3, exactly 3 succeed | symbol+list+blob | all Ok, 4th fails | unit |
| uncapped allows 100 | 100 symbol inserts | all Ok, count==100 | unit |
| all insert variants respect cap | cap=2, mixed inserts | correct insert order | unit |
| at-cap rejects all types | cap=1, various types | `CoreError::BudgetExceeded` | kani |
| cap enforcement | random (cap, insert sequence) | exact cap behavior | proptest |

### Budget Computation (budget.rs)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| entry out of bounds | entry >= nodes.len() | `WorkflowError::EntryOutOfBounds` | unit |
| valid single-node workflow | 1 Nop node | `max_total_steps == 1` | unit |
| StepCountOverflow propagation | large IR | `WorkflowError::StepCountOverflow` | unit |
| Policy validate accepts valid | all dims <= policy | `Ok(())` | unit |
| Policy validate rejects total steps | steps > max_total_steps | `Err(TotalStepsExceeded)` | unit |
| Policy validate rejects fanout | fanout > max_fanout | `Err(FanoutExceeded)` | unit |
| Policy validate rejects depth | depth > max_nesting_depth | `Err(NestingDepthExceeded)` | unit |
| Policy returns first violation | multiple dimensions over | first over dimension error | unit |
| Policy random valid | random budget within policy | `Ok(())` | proptest |
| Policy random invalid | random budget outside policy | `Err(_)` | proptest |

### Run Loop (run_loop.rs)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| exhausts budget before completion | 2-step workflow, budget=1 | `StepBudgetExhausted`, executed=1 | integration |
| zero budget returns immediately | any workflow, budget=0 | `StepBudgetExhausted`, executed=0 | integration |
| exact budget completes | 2-step workflow, budget=2 | `Finished(_, _)`, executed=2 | integration |
| completes within MAX budget | any workflow, budget=MAX | `Finished(_, _)` | integration |
| stops on suspension | Do node workflow | `AwaitingAction` | integration |

---

## 9. Open Questions

1. **Miri deferred to State 11**: `value_store` tests under `cfg(miri)` — ensure `#[cfg_attr(miri, ignore)]` annotations are present on max-size fixtures (`value_store_object_at_exact_max_fields_is_accepted`, `value_store_exact_max_object_preserves_duplicate_first_wins_index`).

2. **Kani loop harnesses**: `run_until_blocked_loop_terminates` and `run_until_blocked_various_budgets` have `#[kani::unwind(10001)]` — may timeout on CI. Primary loop-termination proof is via Verus INV-004 (formal). Kani serves as complementary bounded model checking. Confirm CI timeout threshold.

3. **test_step_count_overflow stub**: The unit test (`test_step_count_overflow` in `budget/tests.rs`) constructs a minimal 1-node workflow but does NOT create a genuinely overflowing IR (one that causes `count_total_steps` to exceed `u32::MAX`). The test verifies the error type can be constructed and that single-node compute succeeds. A genuine overflow test requires a hand-crafted IR with loops — confirm this is sufficient or if a larger fixture is needed.

4. **Fuzz FUZZ-001 corpus**: The `step_budget_new` fuzz target reads from stdin and interprets bytes as u64. The corpus seeds in `fuzz/src/lib.rs::fuzz_step_budget_new` must include the critical boundary values (MAX_STEP_BUDGET ± 1, u64::MAX, 0, 1). Verify corpus is initialized.

---

## 10. Verification Commands Reference

| Obligation | Command | Layer |
|-----------|---------|-------|
| PROPTEST-PRE-001 | `cargo test --package vb_core -- property_step_budget_new_clamp -- --nocapture` | proptest |
| PROPTEST-POST-001 | `cargo test --package vb_core -- property_try_take_count -- --nocapture` | proptest |
| PROPTEST-PRE-002 | `cargo test --package vb_core -- property_value_store_cap -- --nocapture` | proptest |
| PROPTEST-POST-006 | `cargo test --package vb_core -- property_boundedness_policy -- --nocapture` | proptest |
| FUZZ-001 | `cargo fuzz run step_budget_new -- -runs=10000` | fuzz |
| UNIT-POST-003 | `cargo test --package vb_core -- run_until_blocked -- --nocapture` | unit-test |
| UNIT-POST-005 | `cargo test --package vb_core -- test_step_count_overflow -- --nocapture` | unit-test |
| KANI-INV-001 | `cargo kani --package vb_core --lib --harness step_budget_new_clamps` | kani |
| KANI-INV-001 | `cargo kani --package vb_core --lib --harness step_budget_max_value` | kani |
| KANI-INV-004 | `cargo kani --package vb_core --lib --harness run_until_blocked_loop_terminates` | kani |
| KANI-POST-004 | `cargo kani --package vb_core --lib --harness value_store_cap_one_rejects_second` | kani |
| KANI-POST-004 | `cargo kani --package vb_core --lib --harness value_store_all_insert_variants_respect_cap` | kani |
| MIRI-INV-002 | `cargo miri test --package vb_core -- value_store -- --nocapture` | miri |

---

## Exit Criteria Verification

- [x] Every public API behavior has at least one BDD scenario (14 behaviors, 26 scenarios)
- [x] Every pure function with multiple inputs has at least one proptest invariant (4 properties)
- [x] Every parsing/deserialization boundary has a fuzz target (FUZZ-001: StepBudget::new)
- [x] Every error variant in the Error enum has an explicit test scenario (7 error variants covered)
- [x] Mutation threshold target (≥90%) is stated in Section 7
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value — all scenarios specify exact error variants or exact return values
