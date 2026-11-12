# Codebase Map: vb-qi37.2

Bead: `vb-qi37.2`
Title: `runtime: Prove whole-workflow boundedness and resource caps`
State: 2 artifact repair, retry 2
Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2`
Source checkout: `/home/lewis/src/velvet-ballistics` used only for `bd --db ... show vb-qi37.2 --json`

## Bead Scope

The bead asks for proof and evidence that workflow admission and runtime execution are bounded across nested workflow composition, per-run ValueStore/arena growth, step/event/action ceilings, and collect/reduce/repeat/together behavior. No production code, tests, or proofs were edited during this State 2 repair.

## Primary Code Surfaces

- `crates/vb_core/src/budget.rs`: Primary whole-workflow budget model. Contains `WholeWorkflowBudget`, `BoundednessPolicy`, `BudgetError`, `AggregateResourceBudget`, `AggregateResourceCapacity`, `AggregateResourceUsage`, `AggregateReservation`, `AggregateBudgetError`, `AggregateResourceBudget::from_workflow`, `AggregateResourceBudget::from_whole_workflow_budget`, `AggregateResourceUsage::try_add_budget`, `AggregateResourceUsage::try_sub_budget`, `AggregateResourceUsage::fits_within`, `validate_aggregate_budget`, and `validate_step_ceilings`.
- `crates/vb_core/src/budget.rs`: `WholeWorkflowBudget::compute` walks compiled IR and records `max_total_steps`, `max_total_slots`, `max_fanout`, `max_nesting_depth`, `max_steps_executable`, `max_action_tickets`, `max_parallel_in_flight`, `max_gather_pages`, `max_gather_items`, `max_for_each_iterations`, `max_together_branches`, `max_repeat_attempts`, `max_run_time_seconds`, `max_result_bytes`, and `max_total_slots_written`.
- `crates/vb_core/src/budget.rs`: `count_and_push_loop_body`, `count_body_region_nodes`, `visit_body_region_node`, and `count_nested_for_region` use checked multiplication/addition for nested `ForEachStart`, `CollectStart`, `ReduceStart`, and `RepeatStart` worst-case step accounting.
- `crates/vb_core/src/workflow/mod.rs`: Canonical workflow model and validation path. `ResourceContract` includes `max_steps`, `max_slots`, `max_step_budget_per_tick`, `max_transitions_per_tick`, `max_output_bytes`, `max_retry_attempts`, `max_fanout`, `max_collect_items`, `max_queue_depth`, and `max_journal_batch_bytes`. `validate_budget` computes `WholeWorkflowBudget` and applies `BoundednessPolicy::DEFAULT` before accepting workflow parts.
- `crates/vb_core/src/validation.rs`: Secondary validation path also computes `WholeWorkflowBudget` and maps `BoundednessPolicy` failures to `WorkflowError::BudgetPolicyExceeded`. Note detail strings differ from `workflow/mod.rs` for some resources (`max_parallel`, `max_runtime`), so downstream contract review should decide whether this is intentional or drift.
- `crates/vb_core/src/limits.rs`: Hard compile/runtime constants: `MAX_STEPS_PER_WORKFLOW`, `MAX_SLOTS_PER_WORKFLOW`, `MAX_LANGUAGE_NESTING_DEPTH`, `MAX_LIST_ITEMS_PER_VALUE`, `MAX_OBJECT_FIELDS_PER_VALUE`, `MAX_SYMBOL_BYTES_PER_VALUE`, `MAX_BLOB_BYTES_PER_VALUE`, `MAX_VALUES_PER_RUN`, and `MAX_STEP_BUDGET`.
- `crates/vb_core/src/value_store.rs`: Per-run cold arenas for symbols, lists, objects, and blobs. `ValueStore::with_max_slots` configures `max_arena_entries`, insert paths call `check_arena_cap`, and cap exhaustion returns `CoreError::BudgetExceeded { budget: "max_slots", limit }` before inserting.
- `crates/vb_core/src/engine/signals.rs`: `StepBudget` clamps input to `MAX_STEP_BUDGET`, exposes `StepBudget::MAX`, `StepBudget::new`, `StepBudget::try_take`, and returns `EngineError::StepCounterOverflow` if the private counter ever exceeds the ceiling.
- `crates/vb_core/src/engine/run_loop.rs`: `run_until_blocked` and `drive_deterministic` consume one `StepBudget` unit per deterministic step and return `EngineSignal::StepBudgetExhausted` when exhausted.
- `crates/vb_runtime/src/admission.rs`: `RunAdmission` can carry `Option<AggregateResourceBudget>`. `admit_run_with_budget` checks requested aggregate usage with `AggregateResourceUsage::try_add_budget` and `fits_within` before returning `RunAdmission::with_budget`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`: Runtime run submission creates `RunState` with `ValueStore::with_max_slots(workflow.resource_contract().max_slots)`, tying runtime arena cap to accepted workflow resource contract.
- `crates/vb_runtime/src/collect_tests.rs`: Existing collect primitive tests include `collect_start_rejects_fanout_one_over_limit_without_collector_state` and `collect_next_honors_value_store_arena_cap_without_advancing_cursor`, directly relevant to fail-closed collect behavior.

## Existing Test And Evidence Surfaces

- `crates/vb_core/src/budget/tests.rs`: Existing budget module tests cover `WholeWorkflowBudget::compute`, `BoundednessPolicy::validate`, aggregate budget types, policy/capacity checks, overflow/underflow behavior, nested loops, fanout, collect/reduce/repeat/together dimensions, and step ceiling validation.
- `crates/vb_core/src/workflow/tests.rs`: Existing workflow validation tests map every `BudgetError` variant to exact `WorkflowError::BudgetPolicyExceeded` detail strings and include workflow-level budget-policy rejection cases.
- `crates/vb_core/src/engine/tests/integration_budget.rs`: Existing engine budget tests exercise `run_until_blocked` with zero and small step budgets.
- `crates/vb_core/tests/proptest_core_types.rs`: Property tests assert mixed ValueStore insert sequences never exceed `ValueStore::with_max_slots` cap and over-cap inserts fail with `CoreError::BudgetExceeded`.
- `crates/velvet_ballistics/tests/cross_crate_adversarial.rs`: Cross-crate adversarial tests include runtime step-budget exhaustion evidence and broader seam tests around resource limit enforcement.
- `crates/workspace_tests/benches/velvet_ballistics.rs`: Benchmark group includes `WholeWorkflowBudget::compute`, `BoundednessPolicy::DEFAULT.validate`, `run_until_blocked`, `StepBudget::new`, and `ValueStore` budget variants. These are performance scaffolds, not proof by themselves.
- `fuzz/src/bin/resource_budget.rs`, `fuzz/src/bin/budget_compute.rs`, `fuzz/src/bin/aggregate_workflow_budget.rs`, `fuzz/src/bin/aggregate_artifact_budget.rs`, `fuzz/src/bin/step_budget_new.rs`: Existing fuzz entrypoints for budget and step-budget surfaces.
- `verification/verus/resource_budget.rs`, `verification/verus/step_budget.rs`, `verification/verus/budget_monotonic.rs`, `verification/verus/budget_bounded.rs`: Existing Verus proof surfaces for budget composition/monotonic/bounded obligations.

## Risk Tags

- `boundedness`: Primary risk. Nested collect/reduce/repeat/together accounting must compose without bypassing aggregate caps.
- `performance`: `WholeWorkflowBudget::compute` and nested traversal must stay bounded and avoid pathological memory/time growth.
- `persistence`: Runtime `ValueStore` and journal batch dimensions must be capped per run.
- `public-api`: `ResourceContract`, `WholeWorkflowBudget`, `AggregateResourceBudget`, `StepBudget`, `ValueStore::with_max_slots`, and `admit_run_with_budget` are public or cross-crate surfaces.
- `user-visible-behavior`: Over-budget cases must fail with typed errors, not panic/OOM/hang.
- `proof`: Verus/Kani/proptest/fuzz evidence is required to prove more than ordinary unit examples.
- `admission`: Aggregate budget must be computed before admission and compared to capacity before run acknowledgment.

## Open Questions For State 3 Contract

- The bead acceptance says a workflow-level bound certificate is computed before admission. `AggregateResourceBudget::from_workflow` exists, and `admit_run_with_budget` accepts a requested budget, but this map did not prove every submit path computes and passes that budget before acknowledgment.
- `ValueStore::new` remains uncapped by design (`max_arena_entries == 0`). Runtime submission uses `ValueStore::with_max_slots`, but tests/benches often use uncapped stores. Contract should distinguish production runtime paths from isolated unit fixtures.
- `BoundednessPolicy::DEFAULT` allows `max_total_slots: 65_535`, while `ResourceContract::DEFAULT.max_slots` is `1_024`. Contract should specify which limit is authoritative at admission versus runtime execution.
- `crates/vb_core/src/validation.rs` and `crates/vb_core/src/workflow/mod.rs` both map budget errors, but detail strings are not identical for parallel/runtime dimensions. Contract review should classify as acceptable aliasing or diagnostic drift.
- `compiled_workflow.rs` contains a separate `ResourceContract` shape missing `max_transitions_per_tick` and `allows_secret_results`; determine whether it is legacy/dead or an active parity risk before implementation.

## Recommended Downstream Owners

- `rust-contract`: Specify exact pre-admission bound certificate semantics, capacity comparison semantics, and typed error guarantees for aggregate, arena, step, event, and action budgets.
- `proof-planner`: Require Verus for arithmetic/monotonic/capacity obligations, Kani for small bounded step/arena state machines, proptest for nested IR budget generation, fuzz for parser/compiler budget surfaces, and Miri where ValueStore handle access is touched.
- `test-planner`: Add adversarial Given/When/Then scenarios for nested collect/reduce/repeat/together, aggregate budget overflow, admission capacity rejection, ValueStore cap exhaustion, and step exhaustion determinism.
- `holzman-rust`: If implementation changes are needed later, preserve checked arithmetic, no unchecked indexing/casts, no panic paths, and keep runtime core free of YAML/JSON/HTTP.

## State 2 Verification Inputs

- `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.2 --json` succeeded from the isolated workspace.
- `grep` evidence found the core symbols in `crates/vb_core/src/budget.rs`, `crates/vb_core/src/workflow/mod.rs`, `crates/vb_core/src/value_store.rs`, `crates/vb_core/src/engine/signals.rs`, `crates/vb_core/src/engine/run_loop.rs`, and runtime budget admission in `crates/vb_runtime/src/admission.rs`.
- Artifact repair writes are limited to `.beads/vb-qi37.2/codebase-map.md`, `.beads/vb-qi37.2/delivery-scope.jsonl`, and appending `.beads/vb-qi37.2/STATE.md`.
