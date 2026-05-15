# Test Plan: vb-qi37.2.5 — Boundedness adversarial tests

## Summary

- State: go-skill State 7 test-planner.
- Gate consumed: `.beads/vb-qi37.2.5/proof-review.md` and `.beads/vb-qi37.2.5/contract-verification-review.md` both contain `STATUS: APPROVED`.
- Planning doctrine cited: `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 8-10 forbid implementation and require `test-plan.md`; lines 41-171 require behavior inventory, trophy allocation, BDD, proptest, fuzz, Kani, mutation, and exact assertions. `/home/lewis/.agents/skills/test-planner/SKILL.md` has the same content and wins on conflict. `references/testing-philosophy.md` lines 5-10 require behavior/public-API tests; lines 82-86 require every cared-about behavior to have hermetic tests.
- Behaviors identified: 22 contract-traced behaviors.
- Trophy allocation: 10 unit/calc, 8 integration, 1 e2e, 3 static/formal-adjacent checks.
- Proptest invariants: 7.
- Fuzz targets: 3 planned true fuzz targets plus 1 repaired hostile-input stdin replay/property-test surrogate for the current `resource_budget` driver.
- Kani harnesses: 3 planned; one current waiver retained until Cargo-integrated harnesses exist.
- State 7 repair note: `resource_budget` is not a libFuzzer driver; `cargo fuzz run ... -- -runs=1000` is explicitly waived as evidence for `FUZZ-RESOURCE-001` until a `libfuzzer_sys::fuzz_target!` harness is implemented. `FUZZ-RESOURCE-001` is instead discharged by the executable stdin replay plus property-test commands in §5 and §11.
- Mutation threshold: `cargo-mutants` kill rate must be >= 90%; no survivor allowed in budget, value-store cap, step-budget, payload-limit, or admission diagnostics.

## 1. Behavior Inventory

1. Public constructors build adversarial workflows without private invalid states when boundedness fixtures are assembled.
2. Adversarial generators bound all size parameters before allocation when fuzz/proptest data is produced.
3. Deterministic execution returns `EngineSignal::StepBudgetExhausted` when an explicit step budget reaches zero.
4. Capped `ValueStore` preserves `total_arena_count() <= max_arena_entries()` when insertion reaches cap.
5. Nested composition declares finite fanout, loop, repeat, gather, and branch dimensions before budget computation.
6. Pre-existing `vb_runtime` missing chunk is classified `DEFERRED_GLOBAL` when full-workspace evidence observes it.
7. Runaway loops terminate with typed exhaustion or budget error when deterministic execution would otherwise continue.
8. `BoundednessPolicy::validate` rejects fanout above policy with `BudgetError::FanoutExceeded { actual, limit }`.
9. `BoundednessPolicy::validate` rejects nesting above policy with `BudgetError::NestingDepthExceeded { actual, limit }`.
10. Value growth at arena cap rejects next insertion with `CoreError::BudgetExceeded { budget: "max_slots", limit }` and leaves count capped.
11. Overlarge list/object/blob/symbol payloads reject with `CoreError::ResourceLimitExceeded { resource }` naming the exceeded dimension.
12. Nested repeat/together/collect composition that exceeds policy rejects before runtime with `BudgetError::TotalStepsExceeded`, `BudgetError::StepsExecutableExceeded`, or exact verifier diagnostic.
13. Bounded workflows within `BoundednessPolicy::DEFAULT` and `ResourceContract` hard limits are accepted with exact computed dimensions in range.
14. Every adversarial failure path returns a typed `BudgetError`, `CoreError`, `EngineError`, `WorkflowError`, `ValidationError`, or `EngineSignal`, not panic/OOM/timeout/process kill.
15. `StepBudget::new(x)` clamps to `MAX_STEP_BUDGET` and `try_take` monotonically decreases or reports no budget for all `u64` inputs.
16. `drive_deterministic` consumes no more deterministic transitions than mutable `StepBudget` permits.
17. Capped `ValueStore` successful insertions increase arena count by at most one and rejected insertions preserve the cap invariant.
18. `WholeWorkflowBudget::compute` rejects entry outside `nodes` with `WorkflowError::EntryOutOfBounds { entry }`.
19. `WholeWorkflowBudget::compute` rejects compact step-count overflow with `WorkflowError::StepCountOverflow { actual }`.
20. `BoundednessPolicy::validate` maps every over-limit budget dimension to its matching semantic `BudgetError` variant.
21. Nested composition accounting is monotonic as steps, branches, loops, repeats, gather pages/items, and slot writes increase.
22. Resource budget and compiled-IR parser/codec fuzz inputs stay bounded and never panic on malformed bytes.

## 2. Trophy Allocation

| Layer | Behaviors | Count | Rationale |
| --- | --- | ---: | --- |
| Static/formal-adjacent | 6, 14, hard-limit source-scan part of 20 | 3 | Source lint/no-panic and deferred-global classification are gates, not normal runtime assertions. |
| Unit / calc | 2, 8, 9, 11, 13, 15, 18, 19, 20, 21 | 10 | Pure budget/value/limit arithmetic and exact error mapping require exhaustive boundary coverage. |
| Integration | 1, 3, 4, 5, 7, 10, 12, 16, 17 | 8 | Public constructors plus real `CompiledWorkflow`, `RunFrame`, `StepBudget`, `ValueStore`, and policy interactions must be exercised without mocks. |
| E2E / acceptance | 22 | 1 | Fuzz/CLI-style target validates untrusted boundary behavior from outside the API. |

Deviation from 60/30/5/5: this bead is boundedness-critical and calc-heavy, so unit/proptest coverage is intentionally higher than the default trophy ratio. Integration remains broad enough to prove public-API realization, while E2E stays narrow because there is no user workflow beyond fuzz/command gates in the approved contract.

## 3. Given/When/Then BDD Scenarios

### Behavior 1: public constructors only
- Test name: `given_public_constructors_when_adversarial_workflow_built_then_no_private_invalid_state_required`
- Given: adversarial workflow dimensions are finite and representable through `CompiledWorkflow::try_from_parts`, `ResourceContract::validate`, or final public verifier API.
- When: the fixture builder constructs fanout, repeated, nested, and runaway candidates.
- Then: construction uses only public validated constructors and returns either an accepted workflow or exact typed constructor/validation error.
- And: no test may instantiate private invalid state or rely on internal field mutation.

### Behavior 2: prebounded generators
- Test name: `given_adversarial_size_parameters_when_generators_run_then_all_allocations_are_prebounded`
- Given: proptest/fuzz strategies produce fanout, nesting, payload length, slots, and loop counts.
- When: generated parameters are converted into fixtures.
- Then: every allocation length is clamped or rejected before allocation with a finite configured cap.
- Exact failure assertion: over-cap generated payload setup must return the configured fixture error or `CoreError::ResourceLimitExceeded { resource: <dimension> }`, never panic/OOM.

### Behavior 3: explicit step-budget exhaustion
- Test name: `given_explicit_step_budget_when_runaway_workflow_runs_then_step_budget_exhausted_is_returned`
- Given: a valid deterministic workflow that can continue beyond `N` transitions and `StepBudget::new(N)`.
- When: `drive_deterministic` or `run_until_blocked` runs the workflow.
- Then: result is exactly `Ok(EngineSignal::StepBudgetExhausted)` when the budget reaches zero.
- And: `budget.remaining()` is exactly `0` after exhaustion.

### Behavior 4: value-store cap construction
- Test name: `given_capped_value_store_when_insertions_hit_cap_then_budget_exceeded_preserves_count`
- Given: `ValueStore::with_max_slots(max_slots)` where `max_slots > 0`.
- When: inserts fill all slots and one additional insert is attempted.
- Then: the extra insert returns exactly `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`.
- And: `total_arena_count() == max_arena_entries()` and never exceeds it.

### Behavior 5: finite nested dimensions
- Test name: `given_finite_nested_composition_when_budget_computed_then_each_growth_dimension_is_explicit`
- Given: collect/reduce/repeat/together/branching fixture dimensions are finite and named.
- When: `WholeWorkflowBudget::compute` or final verifier admission computes aggregate dimensions.
- Then: each dimension contributes to the observable computed budget or exact typed rejection.

### Behavior 6: deferred global runtime chunk
- Test name: `given_vb_runtime_missing_chunk_when_scoped_evidence_collected_then_classified_deferred_global`
- Given: a full-workspace command later hits `crates/vb_runtime/src/runtime.rs` missing `runtime/chunk_001.rs`.
- When: evidence is classified.
- Then: classification is exactly `DEFERRED_GLOBAL` and not a bead-local boundedness test failure.

### Behavior 7: runaway loop fail-closed
- Test name: `given_runaway_loop_when_budget_reaches_zero_then_execution_returns_step_budget_exhausted_without_panic`
- Given: a validated cyclic-looking deterministic workflow and a low explicit step budget.
- When: execution advances until fuel is depleted.
- Then: result is exactly `Ok(EngineSignal::StepBudgetExhausted)` or a documented typed budget error.
- And: the process does not timeout, panic, OOM, or require kill.

### Behavior 8: fanout over policy
- Test name: `given_fanout_above_policy_when_policy_validates_then_fanout_exceeded_reports_actual_and_limit`
- Given: a `WholeWorkflowBudget` with `fanout = limit + 1` and all other dimensions within limit.
- When: `BoundednessPolicy::validate` runs.
- Then: result is exactly `Err(BudgetError::FanoutExceeded { actual, limit })`.
- And: assert `actual == limit + 1` and `actual > limit`.

### Behavior 9: nesting over policy
- Test name: `given_nesting_above_policy_when_policy_validates_then_nesting_depth_exceeded_reports_actual_and_limit`
- Given: a budget with `nesting_depth = limit + 1` and all other dimensions valid.
- When: policy validation runs.
- Then: result is exactly `Err(BudgetError::NestingDepthExceeded { actual, limit })` with `actual == limit + 1`.

### Behavior 10: value growth at cap
- Test name: `given_value_growth_at_cap_when_next_insert_attempted_then_budget_exceeded_and_count_stays_capped`
- Given: a capped store already containing `max_slots` arena entries.
- When: `insert_symbol`, `insert_list`, `insert_list_with_taint`, `insert_object`, and `insert_blob` each attempt one more insertion in separate scenarios.
- Then: each returns exactly `CoreError::BudgetExceeded { budget: "max_slots", limit }`.
- And: count remains `<= max_arena_entries()` after every rejected insertion.

### Behavior 11: payload hard limits
- Test name: `given_overlarge_payloads_when_inserted_then_resource_limit_exceeded_names_dimension`
- Given: list, object, blob, and symbol payloads whose lengths are exactly one above their hard limits.
- When: each payload is inserted or validated.
- Then: result is exactly `Err(CoreError::ResourceLimitExceeded { resource })`.
- Exact resource assertions: list => list-items dimension; object => object-fields dimension; blob => blob-bytes dimension; symbol => symbol-bytes dimension.

### Behavior 12: nested composition over policy
- Test name: `given_nested_repeat_together_collect_exceeds_policy_when_verified_then_typed_diagnostic_rejects_before_runtime`
- Given: a public nested composition fixture where aggregate steps or executable steps exceed policy.
- When: budget computation and boundedness validation run.
- Then: result is exactly `Err(BudgetError::TotalStepsExceeded { actual, limit })`, `Err(BudgetError::StepsExecutableExceeded { actual, limit })`, or the final verifier's exact boundedness diagnostic.
- And: runtime execution is not entered after admission rejection.

### Behavior 13: accepted bounded workflow
- Test name: `given_bounded_workflow_within_policy_when_computed_and_validated_then_budget_is_accepted`
- Given: finite workflow dimensions all within `BoundednessPolicy::DEFAULT` and `ResourceContract` hard limits.
- When: `WholeWorkflowBudget::compute` and `BoundednessPolicy::validate` run.
- Then: compute returns the exact expected `WholeWorkflowBudget` dimensions and validate returns `Ok(())`.
- Prohibited assertion: no bare `is_ok()` without comparing expected dimensions.

### Behavior 14: typed failures only
- Test name: `given_each_adversarial_failure_path_when_executed_then_result_is_typed_not_panic_oom_or_timeout`
- Given: one adversarial fixture per error taxonomy variant.
- When: each fixture is executed under bounded input sizes and explicit step budget.
- Then: each returns the exact planned `BudgetError`, `CoreError`, `EngineError`, `WorkflowError`, `ValidationError`, or `EngineSignal` variant.
- And: panic/OOM/timeout/process kill is never accepted as success.

### Behavior 15: step-budget clamp and monotonic take
- Test name: `given_any_u64_budget_when_step_budget_new_then_remaining_is_clamped_and_try_take_is_monotonic`
- Given: any `u64` input to `StepBudget::new`.
- When: `remaining()` and repeated `try_take()` calls are observed.
- Then: initial `remaining() == min(input, MAX_STEP_BUDGET)`.
- And: each successful take reduces remaining by exactly one; at zero `try_take()` returns exactly `Ok(false)` and leaves remaining zero.
- Exact error assertion: any overflow path must be exactly `Err(EngineError::StepCounterOverflow)`.

### Behavior 16: deterministic transitions bounded by budget
- Test name: `given_finite_execution_slice_when_budget_consumed_then_transitions_do_not_exceed_budget`
- Given: a finite deterministic workflow and mutable `StepBudget::new(N)`.
- When: `drive_deterministic` returns finish/block/error/exhaustion.
- Then: observed deterministic transitions are `<= N`.
- And: terminal result is exact `EngineSignal`/`EngineError` variant, not only success/failure.

### Behavior 17: interleaved value-store insertions
- Test name: `given_capped_store_when_success_and_failure_insertions_interleave_then_total_count_never_exceeds_cap`
- Given: a capped store and arbitrary sequence of valid/over-cap symbol/list/object/blob insert attempts.
- When: operations execute sequentially.
- Then: every success increases count by at most one and every failure leaves count `<= cap`.
- And: cap failure is exactly `CoreError::BudgetExceeded { budget: "max_slots", limit }`.

### Behavior 18: entry out of bounds
- Test name: `given_entry_out_of_bounds_when_budget_compute_runs_then_typed_workflow_error_returns`
- Given: `nodes.len() = L` and `entry >= L`.
- When: `WholeWorkflowBudget::compute(nodes, entry, contract)` runs.
- Then: result is exactly `Err(WorkflowError::EntryOutOfBounds { entry })` with the same entry value.

### Behavior 19: step-count overflow
- Test name: `given_step_count_overflow_when_budget_compute_runs_then_typed_workflow_error_returns`
- Given: a public fixture whose aggregate step count cannot fit the executable compact representation.
- When: `WholeWorkflowBudget::compute` runs.
- Then: result is exactly `Err(WorkflowError::StepCountOverflow { actual })` and `actual` is above the representable limit.

### Behavior 20: every policy dimension maps exactly
- Test name: `given_each_policy_dimension_above_limit_when_validate_runs_then_matching_budget_error_variant_returns`
- Given: one budget per over-limit dimension, with all other dimensions valid.
- When: `BoundednessPolicy::validate` runs.
- Then: exact variants are asserted:
  - `BudgetError::TotalStepsExceeded { actual, limit }`
  - `BudgetError::TotalSlotsExceeded { actual, limit }`
  - `BudgetError::FanoutExceeded { actual, limit }`
  - `BudgetError::NestingDepthExceeded { actual, limit }`
  - `BudgetError::ParallelExceeded { actual, limit }`
  - `BudgetError::ActionTicketsExceeded { actual, limit }`
  - `BudgetError::RunTimeExceeded { actual, limit }`
  - `BudgetError::ResultBytesExceeded { actual, limit }`
  - `BudgetError::StepsExecutableExceeded { actual, limit }`
- And: for every row, `actual == limit + 1` where fixture construction can enforce one-over.

### Behavior 21: nested monotonic accounting
- Test name: `given_larger_nested_dimensions_when_budget_computed_then_aggregate_bound_does_not_decrease`
- Given: two valid nested fixtures where the second has one increased dimension and all others equal.
- When: budgets are computed.
- Then: aggregate steps, slots, fanout, nesting, parallel, action tickets, runtime, result bytes, and executable steps do not decrease.

### Behavior 22: malformed bytes stay bounded
- Test name: `given_malformed_resource_budget_bytes_when_fuzzed_then_no_panic_and_input_stays_bounded`
- Given: malformed bytes for resource budget / compiled IR boundary.
- When: the repaired `FUZZ-RESOURCE-001` hostile-input replay command in §5/§11 executes the existing stdin-once `resource_budget` binary on 1000 deterministic bounded inputs, and the focused malformed-byte/property tests execute.
- Then: no panic, OOM, timeout, sanitizer failure, or unbounded allocation occurs.
- And: any semantic failure is a typed parse/validation/resource error.

## 4. Proptest Invariants

### Proptest: `StepBudget::new` / `StepBudget::try_take`
- Invariant: `remaining()` starts at `min(input, MAX_STEP_BUDGET)` and monotonically decreases by one per successful `try_take`.
- Strategy: any `u64`, plus operation count `0..MAX_STEP_BUDGET + 2` bounded for test runtime.
- Anti-invariant: remaining increases, underflows, or `try_take` succeeds after zero.

### Proptest: `BoundednessPolicy::validate`
- Invariant: generated budgets within policy validate to exact `Ok(())`; generated one-over budgets reject with the matching `BudgetError` variant and `actual > limit`.
- Strategy: generate `WholeWorkflowBudget` dimensions around `0`, `limit`, `limit + 1`, and max representable safe values.
- Anti-invariant: wrong variant for the exceeded semantic dimension or bare acceptance of one-over budget.

### Proptest: `WholeWorkflowBudget::compute`
- Invariant: valid entries compute deterministic dimensions; invalid entries always return `WorkflowError::EntryOutOfBounds { entry }`; step overflow returns `WorkflowError::StepCountOverflow { actual }`.
- Strategy: generated node vectors with bounded length, entry indices in/out of bounds, bounded loop/repeat multipliers.
- Anti-invariant: panic, unchecked indexing, or accepting an out-of-bounds entry.

### Proptest: nested composition monotonicity
- Invariant: increasing a single nested dimension never decreases computed aggregate bound.
- Strategy: pairs of finite public nested fixtures differing by one in fanout, loop count, repeat count, gather items/pages, branch count, or slot writes.
- Anti-invariant: larger fixture produces smaller aggregate dimension without typed rejection.

### Proptest: capped `ValueStore`
- Invariant: for cap `1..=u16::MAX`, successes keep `count <= cap`; rejected insertions leave count unchanged or still `<= cap` and report exact max-slots error.
- Strategy: bounded sequences of symbol/list/object/blob insert operations with payload lengths within hard per-value limits unless separately testing resource limits.
- Anti-invariant: count exceeds cap or cap failure returns wrong `CoreError`.

### Proptest: payload resource limits
- Invariant: payload length `<= limit` follows normal insertion semantics; payload length `limit + 1` rejects with `CoreError::ResourceLimitExceeded { resource }` naming the dimension.
- Strategy: boundary lengths `0`, `1`, `limit`, `limit + 1`, with allocation-safe generated payloads only.
- Anti-invariant: over-limit payload allocates unbounded memory or reports max-slots instead of resource dimension when store capacity is sufficient.

### Proptest: `ResourceContract::validate`
- Invariant: resource contracts at hard limits validate; one-over hard limits reject with exact `ValidationError` for the exceeded field.
- Strategy: generate `max_step_budget_per_tick`, max slots, output bytes, and hard-limit fields around boundaries.
- Anti-invariant: one-over contract accepted or rejected with unrelated error.

## 5. Fuzz Targets

### Repaired FUZZ-RESOURCE-001 surrogate: `fuzz_resource_budget` / `resource_budget` stdin replay
- Input type: bytes.
- Risk: malformed budget/IR bytes trigger panic, OOM, timeout, unchecked arithmetic, or unbounded allocation.
- Corpus seeds: empty input; all zeroes; all `0xff`; single-node valid workflow; max fanout one-over; nesting one-over; compact step overflow marker; max-slots cap one-over; overlarge payload length headers.
- Current driver fact: `fuzz/src/bin/resource_budget.rs` reads stdin once, calls `fuzz_lib::fuzz_resource_budget(&input)` once, and exits. It is not a `libfuzzer_sys::fuzz_target!` harness and does not honor `-runs=1000`.
- Waived command: `cargo fuzz run --target x86_64-unknown-linux-gnu resource_budget -- -runs=1000` is **not valid evidence** for this obligation because the binary ignores libFuzzer arguments. No PASS may be claimed from that command for `FUZZ-RESOURCE-001`.
- Required executable replay command for current repository target:

```bash
mkdir -p target/tmp && \
RUSTC_WRAPPER= TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5/target/tmp \
cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget && \
python3 -c "import subprocess; from pathlib import Path; t=Path('target/debug/resource_budget'); assert t.exists(), f'missing {t}'; fixed=[b'', b'\x00', b'\x00'*32, b'\xff'*32, b'fanout-over-policy', b'nesting-over-policy', b'compact-step-overflow', b'max-slots-cap-one-over', b'payload-length-header-one-over']; cases=fixed+[(i.to_bytes(8,'little') + bytes([(i*31)%256])*(i%64))[:72] for i in range(991)]; [(_ for _ in ()).throw(SystemExit(f'resource_budget stdin replay failed at case {idx} rc={r.returncode}')) for idx,data in enumerate(cases) for r in [subprocess.run([str(t)], input=data, timeout=2)] if r.returncode != 0]; print(f'resource_budget stdin replay PASS cases={len(cases)}')"
```

- Required companion property/test commands for `FUZZ-RESOURCE-001` / `INV-008`:

```bash
RUSTC_WRAPPER= TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5/target/tmp \
rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial \
given_malformed_resource_budget_bytes_when_fuzzed_then_no_panic_and_input_stays_bounded -- --nocapture

RUSTC_WRAPPER= TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5/target/tmp PROPTEST_CASES=10000 \
rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial proptest -- --nocapture
```

- Acceptance: the replay prints exactly `resource_budget stdin replay PASS cases=1000`; both companion commands exit 0; semantic failures remain typed; no panic/OOM/timeout/process kill is accepted. If State 8 wants to claim true cargo-fuzz coverage instead, it must first implement a real libFuzzer harness and then reroute through plan/test review.

### Fuzz Target: compiled workflow admission bytes
- Input type: bytes decoded to public compiled workflow or rejected parse result.
- Risk: invalid entry, node count, branch target, or nested shape causes unchecked indexing or panic.
- Corpus seeds: empty node list with entry 0; entry equal to node len; branch target outside nodes; cyclic-looking transition; repeated nested collect/reduce/together shapes.

### Fuzz Target: value payload codecs
- Input type: bytes split into symbol/list/object/blob payloads with bounded allocator guards.
- Risk: payload length/resource-limit parser mistakes, max-slot cap bypass, taint/list/object malformed boundaries.
- Corpus seeds: zero-length symbol; symbol at max bytes; max+1 symbol header; list count max/max+1; object field max/max+1; blob max/max+1 header with truncated body.

### Fuzz Target: `ResourceContract` decode/validation boundary
- Input type: bytes mapped to contract fields.
- Risk: one-over hard limits accepted; arithmetic overflow while deriving budget ceilings; panic on malformed contract bytes.
- Corpus seeds: all defaults; max-step-budget zero; max-step-budget hard limit; hard limit + 1; max slots zero/one/u16::MAX.

## 6. Kani Harnesses

### Kani Harness: step budget monotonic bounded model
- Property: for all bounded initial budgets and bounded take counts, remaining never increases or underflows and zero budget returns `Ok(false)`.
- Bound: initial budget `0..=MAX_STEP_BUDGET` with small harness cap representative subset if full max is too large; take count `0..=MAX_STEP_BUDGET + 1` or proof-planner-approved smaller unwind.
- Rationale: complements Verus pure proof with compiled Rust behavior; required before claiming Kani discharge.

### Kani Harness: deterministic run-loop fuel bound
- Property: finite deterministic run-loop model cannot consume more transitions than `StepBudget` permits and terminal outcome is typed.
- Bound: workflow nodes `0..=3`, budget `0..=3`, terminal states matching `TLA-SLICE-001`.
- Rationale: `KANI-LOOP-001` is currently waived because discovered files are not Cargo-integrated; test-writer/proof-writer must add a Cargo-integrated harness before Kani PASS is claimed.

### Kani Harness: value-store cap counter bound
- Property: bounded insertion sequence over capped store never yields `total_arena_count() > max_arena_entries()`.
- Bound: cap `0..=3`, operation length `0..=4`, operation kind symbol/list/object/blob.
- Rationale: model checks counter/cap state independent of randomized proptest sequences.

## 7. Mutation Checkpoints

Threshold: `cargo-mutants` kill rate must be >= 90% overall for touched boundedness paths, with 100% kill required for the critical mutations below.

- Change `actual > limit` to `actual >= limit` in each policy dimension: killed by one-at-limit accepted and one-over rejected scenarios.
- Swap `FanoutExceeded` and `NestingDepthExceeded`: killed by exact error variant BDD 8/9.
- Swap any `BudgetError` dimension in `BoundednessPolicy::validate`: killed by BDD 20.
- Remove `StepBudget::new` clamp to `MAX_STEP_BUDGET`: killed by proptest step-budget clamp.
- Make `try_take` leave remaining unchanged on success: killed by monotonic exact-decrement scenario.
- Let `try_take` underflow or return `Ok(true)` at zero: killed by zero-budget exact assertion.
- Remove `EngineSignal::StepBudgetExhausted` branch: killed by runaway loop BDD 3/7.
- Change arena cap comparison from `>=` to `>` or vice versa incorrectly: killed by cap-fill-plus-one scenarios.
- Return `ResourceLimitExceeded` instead of `BudgetExceeded { budget: "max_slots" }` at arena cap: killed by exact value-store error scenarios.
- Return `BudgetExceeded` instead of `ResourceLimitExceeded { resource }` for overlarge payload with sufficient store cap: killed by payload resource-limit scenarios.
- Accept out-of-bounds entry or default to entry zero: killed by `WorkflowError::EntryOutOfBounds { entry }` scenario.
- Saturating/overflow step count silently accepted: killed by `WorkflowError::StepCountOverflow { actual }` scenario.
- Remove nested monotonic aggregation term for loops/repeats/branches/gather items: killed by nested monotonic proptest.
- Enter runtime after admission rejection: killed by nested composition over-policy integration scenario.

## 8. Combinatorial Coverage Matrix

### Step budget / run loop

| Scenario | Input Class | Expected Output | Test Layer |
| --- | --- | --- | --- |
| zero budget | `StepBudget::new(0)` | `try_take() == Ok(false)`, `remaining() == 0` | unit |
| one budget | `StepBudget::new(1)` | first take `Ok(true)`, second `Ok(false)`, remaining `0` | unit |
| over max constructor | `u64::MAX` | `remaining() == MAX_STEP_BUDGET` | unit/proptest |
| runaway deterministic slice | valid continuing workflow, budget N | `Ok(EngineSignal::StepBudgetExhausted)`, transitions `<= N` | integration |
| private overflow guard | forced/fixture step-counter overflow if public API can reach it | `Err(EngineError::StepCounterOverflow)` | unit/integration |

### Boundedness policy dimensions

| Scenario | Input Class | Expected Output | Test Layer |
| --- | --- | --- | --- |
| all dimensions at limit | budget exactly at policy limits | `Ok(())` and exact budget unchanged | unit |
| total steps one-over | `total_steps = limit + 1` | `Err(BudgetError::TotalStepsExceeded { actual, limit })` | unit |
| total slots one-over | `total_slots = limit + 1` | `Err(BudgetError::TotalSlotsExceeded { actual, limit })` | unit |
| fanout one-over | `fanout = limit + 1` | `Err(BudgetError::FanoutExceeded { actual, limit })` | unit |
| nesting one-over | `nesting_depth = limit + 1` | `Err(BudgetError::NestingDepthExceeded { actual, limit })` | unit |
| parallel one-over | `parallel = limit + 1` | `Err(BudgetError::ParallelExceeded { actual, limit })` | unit |
| action tickets one-over | `action_tickets = limit + 1` | `Err(BudgetError::ActionTicketsExceeded { actual, limit })` | unit |
| runtime one-over | `run_time = limit + 1` | `Err(BudgetError::RunTimeExceeded { actual, limit })` | unit |
| result bytes one-over | `result_bytes = limit + 1` | `Err(BudgetError::ResultBytesExceeded { actual, limit })` | unit |
| executable steps one-over | `steps_executable = limit + 1` | `Err(BudgetError::StepsExecutableExceeded { actual, limit })` | unit |
| arbitrary valid/invalid budgets | generated dimensions | exact accept/reject invariant holds | proptest |

### Workflow budget computation

| Scenario | Input Class | Expected Output | Test Layer |
| --- | --- | --- | --- |
| empty nodes, entry 0 | no nodes | `Err(WorkflowError::EntryOutOfBounds { entry: 0 })` | unit |
| entry equals len | finite nodes len L, entry L | `Err(WorkflowError::EntryOutOfBounds { entry })` | unit/proptest |
| accepted bounded workflow | valid nodes and contract | exact `WholeWorkflowBudget` dimensions, policy `Ok(())` | integration |
| compact step overflow | aggregate steps above compact representation | `Err(WorkflowError::StepCountOverflow { actual })` | integration |
| nested monotonic pair | one dimension increased | computed aggregate dimensions do not decrease or exact over-limit typed reject | proptest |

### Value store and payload limits

| Scenario | Input Class | Expected Output | Test Layer |
| --- | --- | --- | --- |
| cap one first insert | cap 1, one valid symbol/list/object/blob per separate test | first insert returns exact handle and count `1` | unit |
| cap one second insert | cap 1, second insert | `Err(CoreError::BudgetExceeded { budget: "max_slots", limit })`, count `1` | unit |
| cap N interleaving | random bounded operation sequence | count never exceeds cap; exact max-slots error on cap rejection | proptest |
| list one-over | sufficient store cap, list len max+1 | `Err(CoreError::ResourceLimitExceeded { resource: <list-items> })` | unit/fuzz |
| object one-over | sufficient store cap, object fields max+1 | `Err(CoreError::ResourceLimitExceeded { resource: <object-fields> })` | unit/fuzz |
| blob one-over | sufficient store cap, blob bytes max+1 header/fixture | `Err(CoreError::ResourceLimitExceeded { resource: <blob-bytes> })` | unit/fuzz |
| symbol one-over | sufficient store cap, symbol bytes max+1 | `Err(CoreError::ResourceLimitExceeded { resource: <symbol-bytes> })` | unit/fuzz |

### Resource contract / validation

| Scenario | Input Class | Expected Output | Test Layer |
| --- | --- | --- | --- |
| hard limits exactly | all fields at hard limit | `Ok(())` with exact accepted contract | unit |
| max step per tick zero | zero if forbidden by validator | exact `ValidationError` variant for field | unit |
| max step per tick one-over | hard limit + 1 | exact `ValidationError` for `max_step_budget_per_tick` | unit |
| generated contracts | boundary field strategies | exact accept/reject by field | proptest |

### Fuzz / static / deferred gates

| Scenario | Input Class | Expected Output | Test Layer |
| --- | --- | --- | --- |
| malformed resource bytes | arbitrary bytes | no panic/OOM/timeout/sanitizer failure | fuzz/e2e |
| repaired resource stdin replay | 1000 deterministic bounded stdin byte cases | `resource_budget stdin replay PASS cases=1000`, exit 0, no panic/OOM/timeout | hostile-input replay/e2e |
| source lint boundedness paths | changed production source | `moon run :lint-src` exits 0; no unsafe/panic/unwrap/expect/todo/unimplemented/dbg regressions | static |
| full workspace hits runtime chunk | missing `runtime/chunk_001.rs` if encountered | exact classification `DEFERRED_GLOBAL`, not bead-local failure | static/evidence |

## 9. Exact Error Assertion Requirements

Tests must never assert only `is_ok()` or `is_err()`. Every negative test must match and inspect fields:

- `BudgetError::TotalStepsExceeded { actual, limit }`: assert `actual > limit`; prefer `actual == limit + 1` in boundary tests.
- `BudgetError::TotalSlotsExceeded { actual, limit }`: assert `actual > limit`.
- `BudgetError::FanoutExceeded { actual, limit }`: assert `actual == limit + 1`.
- `BudgetError::NestingDepthExceeded { actual, limit }`: assert `actual == limit + 1`.
- `BudgetError::ParallelExceeded { actual, limit }`: assert `actual > limit`.
- `BudgetError::ActionTicketsExceeded { actual, limit }`: assert `actual > limit`.
- `BudgetError::RunTimeExceeded { actual, limit }`: assert `actual > limit`.
- `BudgetError::ResultBytesExceeded { actual, limit }`: assert `actual > limit`.
- `BudgetError::StepsExecutableExceeded { actual, limit }`: assert `actual > limit`.
- `CoreError::BudgetExceeded { budget: "max_slots", limit }`: assert exact budget string and exact cap limit.
- `CoreError::ResourceLimitExceeded { resource }`: assert exact resource dimension for list/object/blob/symbol.
- `EngineError::StepCounterOverflow`: assert exact variant if reachable through public fixture.
- `EngineSignal::StepBudgetExhausted`: assert exact signal and zero remaining budget.
- `WorkflowError::EntryOutOfBounds { entry }`: assert exact entry value.
- `WorkflowError::StepCountOverflow { actual }`: assert actual exceeds compact representation.
- `ValidationError`: assert exact field/dimension variant exposed by `ResourceContract::validate`.

## 10. Open Questions / Blockers

1. OQ-001 remains: final public API from `vb-qi37.2.4` for nested collect/reduce/repeat/together verifier diagnostics must be confirmed before implementing BDD 5/12/21.
2. OQ-002 remains: test-writer must choose whether adversarial tests live in existing module/integration files or a dedicated `vb_qi37_2_5_*` integration module.
3. `KANI-LOOP-001` remains waived for proof-review scope; no Kani PASS may be claimed until a Cargo-integrated harness exists.
4. `PO-006` through `PO-011` remain downstream execution obligations; this plan does not claim they have run.
5. The `vb_runtime` missing chunk remains `DEFERRED_GLOBAL` only if encountered by later full-workspace gates; it is not a bead-local boundedness blocker.

## 11. State 7 Repair Completion Evidence

- Isolation verified in workdir `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`; `git status` is not available there because this is the known isolated JJ workspace, not the source checkout git root.
- Mandatory startup files read and applied: `/home/lewis/.claude/skills/test-planner/SKILL.md` and conflict-winner `/home/lewis/.agents/skills/test-planner/SKILL.md`; both require planning only, public-behavior BDD, fuzz-boundary planning, and exact assertions.
- Rejection inputs read: `test-plan-review.md`, `test-suite-review.md`, `test-repair-guide.md`, and `test-writer-report.md`.
- Driver fact checked: `fuzz/src/bin/resource_budget.rs` reads stdin once and does not honor `-runs=1000`.
- Repair completed: `FUZZ-RESOURCE-001` now uses an explicit valid waiver for the hollow cargo-fuzz command plus a concrete stdin replay/property-test alternative mapped to BDD 22 and `INV-008`.
- Executability smoke evidence: after building `resource_budget`, the compact no-heredoc replay script completed with `resource_budget stdin replay PASS cases=1000`.
- Production code edits: none. Test code edits: none. Artifact edits: this `test-plan.md` and `STATE.md` only.
