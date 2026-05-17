# Verification Layers: vb-qi37.2

## Boundary
- Verus-owned kernel: `crates/vb_core/src/budget.rs`, `crates/vb_core/src/value_store.rs`, `crates/vb_core/src/engine/signals.rs` via existing `verification/verus/*` surfaces where applicable.
- TLA+ temporal model: planned `verification/tla/WorkflowBoundedAdmission.tla` for certificate/admission/reservation/execution lifecycle.
- Theorem projection: none; Verus owns local arithmetic/state proofs.
- Runtime shell: `crates/vb_runtime/src/admission.rs`, `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`, `crates/vb_core/src/engine/run_loop.rs` verified by Kani/proptest/fuzz/Miri/tests in later states.
- External systems excluded from formal proof: YAML/CLI/UI/storage/wall-clock; runtime core must not depend on YAML/JSON/HTTP.

## Layer Assignment
- PRE-001 -> static-scan + test-planner scenario for compiled/validated artifact only.
- PRE-002 -> Verus + Kani + proptest over ResourceContract/BoundednessPolicy dimensions.
- PRE-003 -> TLA+ + proptest admission scenario.
- PRE-004 -> Verus + Kani + Miri/cargo-careful + proptest over ValueStore cap behavior. Verus owns the cap invariant; Kani owns exact typed rejection parity; Miri owns UB/handle sanity only.
- PRE-005 -> Verus + Kani + proptest for StepBudget clamp and consumption.
- POST-001 -> TLA+ + deterministic proptest for `WholeWorkflowBudget::compute`.
- POST-002 -> TLA+ + Kani/proptest for AggregateResourceUsage/Capacity comparison.
- POST-003 -> Verus + Kani + fuzz (`fuzz/src/bin/budget_compute.rs`, `fuzz/src/bin/aggregate_workflow_budget.rs`) for nested structure growth and overflow.
- POST-004 -> Verus + Kani + Miri + proptest for capped ValueStore insertions and exact `CoreError::BudgetExceeded { budget: "max_slots", limit }` behavior.
- POST-005 -> Verus + Kani + proptest for deterministic step exhaustion.
- POST-006 -> static-scan + fuzz + Miri + mutation for no panic/OOM/unbounded allocation fail-closed behavior.
- POST-007 -> test-planner and mutation for exact typed diagnostics.
- INV-001 -> Verus + proptest.
- INV-002 -> TLA+ + Kani/proptest.
- INV-003 -> Verus + proptest.
- INV-004 -> Verus + Kani + fuzz.
- INV-005 -> Verus (`verification/verus/value_store_invariant.rs`) + Kani + proptest + Miri. No Verus waiver is used; `VERUS-VS-001` is required.
- INV-006 -> Verus + Kani.
- INV-007 -> TLA+ + deterministic replay tests in later states.
- INV-008 -> static-scan production source gate.
- ERR-001..ERR-007 -> Fowler scenarios, mutation, and exact diagnostic assertions in later test states.
- PERF-001 -> performance benchmark evidence only for no-regression/bounded-runtime claims; no speedup claimed.

## Verus Scope
- Rust targets: `crates/vb_core/src/budget.rs`, `crates/vb_core/src/value_store.rs`, `crates/vb_core/src/engine/signals.rs`.
- Existing proof surfaces from State 2: `verification/verus/resource_budget.rs`, `verification/verus/step_budget.rs`, `verification/verus/budget_monotonic.rs`, `verification/verus/budget_bounded.rs`, `verification/verus/value_store_invariant.rs`.
- Spec/proof function names bound in `proof-obligations.jsonl`:
  - `resource_budget.rs`: `budget_ok`, `policy_ok`, `policy_within`, `lemma_policy_check_exact`, `lemma_policy_preserves_bounded_budget`, `lemma_empty_budget_ok`.
  - `budget_monotonic.rs`: `spec_budget_non_decreasing`, `proof_budget_accumulates_correctly_same_ir`, `proof_deterministic_step_count`, `proof_deterministic_fanout`, `proof_deterministic_nesting_depth`, `proof_whole_workflow_budget_deterministic`.
  - `budget_bounded.rs`: `spec_count_total_steps_bounded`, `checked_add`, `checked_mul`, `spec_count_total_steps_result`, `proof_steps_bounded`, `proof_sequential_add_bounded`, `proof_overflow_add_returns_none`, `proof_overflow_mul_returns_none`, `proof_counting_from_zero`.
  - `step_budget.rs`: `can_take`, `remaining_after_take`, `lemma_try_take_success_never_underflows`, `lemma_try_take_failure_preserves_remaining`, `lemma_try_take_monotonic`, `lemma_zero_request_noop`, `lemma_exact_request_reaches_zero`.
  - `value_store_invariant.rs`: `spec_value_store_cap`, `spec_arena_after_insert`, `spec_check_arena_cap`, `proof_arena_cap_enforced`, `proof_cap_exactly_rejects_insert`, `proof_one_below_cap_allows_insert`, `proof_uncapped_always_allows`, `proof_cap_one_rejects_second`, `proof_check_arena_cap_gate`, `proof_total_never_exceeds_cap`.
- Invariants: finite dimensions, monotonic aggregate cost, checked arithmetic overflow rejection, arena count cap, step budget monotonic decrease.
- Trusted boundary: validated compiled workflow, finite ResourceContract, construction of ValueStore through production submit path, StepBudget constructor.
- Shell exclusions: I/O, async scheduling, storage, wall-clock time, CLI, YAML parsing.
- Evidence commands where files are known:
  - `verus verification/verus/resource_budget.rs`
  - `verus verification/verus/budget_monotonic.rs`
  - `verus verification/verus/budget_bounded.rs`
  - `verus verification/verus/step_budget.rs`
  - `verus verification/verus/value_store_invariant.rs`

## TLA+ Scope
- Module/model path: planned `verification/tla/WorkflowBoundedAdmission.tla`.
- Variables: artifact state, certificate, requested budget, capacity, usage, reservation, run state, value slots, step budget, outcome.
- Actions: Init, ComputeCertificate, RejectInvalidCertificate, ReserveCapacity, RejectOverCapacity, AckRun, CreateCappedRunState, ExecuteStep, ExhaustStepBudget, ReleaseReservation, FailClosed.
- Safety invariants: no ack without certificate, no ack over capacity, no uncapped run state, fail-closed not runnable, step budget never negative.
- Temporal properties: eventually ack or reject; eventually blocked or terminal for finite step budget.
- Fairness/deadlock stance: weak fairness on enabled internal actions; no deadlock except explicit terminal/rejected states.
- Refinement boundary: Rust validation/admission/run-loop events refine model actions.
- Evidence command after model creation: `tlc -config verification/tla/WorkflowBoundedAdmission.cfg verification/tla/WorkflowBoundedAdmission.tla`.

## Second-Ring Evidence
- Kani obligations with exact State 5 harness names:
  - `cargo kani -p vb_core --harness aggregate_usage_try_add_budget_rejects_overflow_and_sums_fields`
  - `cargo kani -p vb_core --harness aggregate_usage_fits_within_rejects_over_capacity_fields`
  - `cargo kani -p vb_core --harness value_store_cap_rejects_insert_with_budget_exceeded_max_slots`
  These harnesses must bind to existing production targets in `crates/vb_core/src/budget.rs` and `crates/vb_core/src/value_store.rs`; they are not substitutes for Verus.
- Parity obligation command: `cargo test -p vb_core resource_contract -- --nocapture && cargo test -p velvet-ballastics-workspace resource_contract -- --nocapture`, plus reviewer source inspection resolving `crates/vb_core/src/validation.rs`, `crates/vb_core/src/workflow/mod.rs`, and `crates/vb_core/src/compiled_workflow.rs` active/legacy status.
- Fuzz commands after harness confirmation:
  - `cargo fuzz run resource_budget -- -runs=1000`
  - `cargo fuzz run budget_compute -- -runs=1000`
  - `cargo fuzz run aggregate_workflow_budget -- -runs=1000`
  - `cargo fuzz run aggregate_artifact_budget -- -runs=1000`
  - `cargo fuzz run step_budget_new -- -runs=1000`
- Performance commands after benchmark confirmation:
  - `cargo bench --bench aggregate_resource_budget`
  - `cargo bench -p workspace_tests --bench velvet_ballastics`
  Expected threshold: no claim of speedup; evidence must show bounded completion and no more than 10 percent regression against `origin/main` for budget hot paths if implementation changes touch them.
- Static source gate: `moon ci` remains canonical repository gate; source lint must preserve zero tolerance.

## Waivers
- THM-WAIVER-001 only: no Lean/Aeneas/Hax at State 3. No waiver for TLA+ lifecycle, ValueStore cap arithmetic, aggregate admission, or Verus-owned local boundedness clauses.
