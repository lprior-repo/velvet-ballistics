# Contract Specification: vb-qi37.2

## Context
- Feature: runtime whole-workflow boundedness and resource caps.
- Bead: `vb-qi37.2` - `runtime: Prove whole-workflow boundedness and resource caps`.
- Source of truth read for State 3: State 2 artifacts in `.beads/vb-qi37.2/` and bead JSON from `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.2 --json`.
- Scope: contract only. No production source, tests, or proof code are written by this artifact.

## Domain Terms
- Workflow-level bound certificate: deterministic budget summary computed from accepted compiled workflow IR before runtime admission.
- ResourceContract: author-provided or default workflow caps for steps, slots, fanout, collect items, queue depth, journal batch bytes, retry attempts, output bytes, and per-tick step/transition ceilings.
- WholeWorkflowBudget: computed aggregate worst-case dimensions for steps, slots, fanout, nesting, executable steps, action tickets, parallel in-flight work, gather pages/items, loop iterations, together branches, repeat attempts, runtime seconds, result bytes, and slots written.
- AggregateResourceBudget: runtime admission projection derived from the workflow certificate and compared against AggregateResourceCapacity/Usage.
- Arena cap: per-run ValueStore slot/arena limit used by production runtime submission.
- Typed budget failure: an explicit domain error such as `BudgetError`, `WorkflowError::BudgetPolicyExceeded`, `AggregateBudgetError`, `CoreError::BudgetExceeded`, or `EngineSignal::StepBudgetExhausted`; never panic, OOM, or unbounded allocation.

## Assumptions
- `ValueStore::new` may remain uncapped only for explicit fixtures and tests; production runtime submission must use `ValueStore::with_max_slots(workflow.resource_contract().max_slots)`.
- `BoundednessPolicy::DEFAULT` is a global safety ceiling; `ResourceContract` is the per-workflow/runtime admission cap and must not exceed the safety ceiling.
- State 2 identified existing proof/fuzz surfaces but did not execute them; these artifacts plan evidence only.
- `compiled_workflow.rs` contains a separate ResourceContract shape whose active/legacy status is unresolved; parity is a required downstream review/implementation obligation.

## Open Questions
- OQ-001: Are `crates/vb_core/src/validation.rs` and `crates/vb_core/src/workflow/mod.rs` diagnostic detail string differences intentional aliases or drift?
- OQ-002: Is the `compiled_workflow.rs` ResourceContract shape active production API or legacy/dead code? This is no longer a blocker placeholder; `PARITY-001` names the focused test command and required source-review decision.
- OQ-003: Which command wrapper, if any, is canonical for the existing Verus/fuzz surfaces in this workspace? Direct Verus commands are listed with exact spec/proof function names where paths are known.

## Preconditions
- PRE-001: Workflow admission input is an already compiled/validated workflow artifact with a concrete `ResourceContract`.
- PRE-002: Every `ResourceContract` numeric dimension is finite, nonzero where zero would mean unbounded runtime work, and no larger than `BoundednessPolicy::DEFAULT` or corresponding hard limit constants.
- PRE-003: The admission caller computes or supplies an `AggregateResourceBudget` derived from the same workflow artifact that will be acknowledged for runtime execution.
- PRE-004: Runtime run creation receives an explicit per-run `max_slots` cap from the accepted workflow resource contract.
- PRE-005: Step/transition execution receives a finite `StepBudget` or equivalent per-tick ceiling clamped by `MAX_STEP_BUDGET`.

## Postconditions
- POST-001: Accepted workflow admission has a workflow-level bound certificate that is deterministic for the same compiled IR and resource contract.
- POST-002: Runtime admission rejects aggregate requested usage that would exceed aggregate capacity before any runnable acknowledgment is returned.
- POST-003: Nested collect/reduce/repeat/together/for-each composition contributes worst-case checked costs to the aggregate budget; overflow or unbounded structural growth is rejected with a typed budget error.
- POST-004: Runtime ValueStore insertions on production run state cannot increase total arena entries beyond the configured per-run cap.
- POST-005: Step execution stops at configured ceilings with `EngineSignal::StepBudgetExhausted` or a typed budget error and remains replay deterministic.
- POST-006: Every over-budget admission, validation, arena, or step condition fails closed without panic, unchecked arithmetic overflow, unchecked indexing, OOM, or unbounded allocation.
- POST-007: Public/user-visible diagnostics identify the budget dimension and limit that caused rejection.

## Invariants
- INV-001: For every accepted workflow, `WholeWorkflowBudget` dimensions are finite and satisfy `BoundednessPolicy::DEFAULT`.
- INV-002: For every admitted run, aggregate used plus requested resources fit within aggregate capacity before run acknowledgment.
- INV-003: Nested body accounting is monotonic: adding a bounded node/body cannot reduce computed max steps, slots, fanout, nesting depth, action tickets, or parallel in-flight dimensions.
- INV-004: Checked arithmetic is used for all aggregate and nested budget addition/multiplication; overflow becomes a typed budget error.
- INV-005: Production `ValueStore` total arena count is always `<= max_arena_entries` when `max_arena_entries > 0`.
- INV-006: StepBudget remaining units are monotonically non-increasing during `run_until_blocked`/`drive_deterministic`.
- INV-007: All fail-closed paths preserve deterministic replay outcome class for the same artifact, inputs, and configured budgets.
- INV-008: Runtime core boundedness enforcement does not require YAML, JSON, or HTTP at runtime.

## Error Taxonomy
- ERR-001: `BudgetError` - workflow-level budget computation or policy validation rejects overflow, excessive total steps/slots/fanout/nesting/runtime/result/action/parallel/gather/loop/repeat dimensions.
- ERR-002: `WorkflowError::BudgetPolicyExceeded` - workflow validation maps a budget policy failure into a user-visible validation diagnostic.
- ERR-003: `AggregateBudgetError` - runtime admission aggregate requested usage cannot fit remaining capacity or underflows on release.
- ERR-004: `CoreError::BudgetExceeded { budget: "max_slots", limit }` - ValueStore insertion would exceed the per-run arena cap.
- ERR-005: `EngineSignal::StepBudgetExhausted` - deterministic run loop consumes the configured step budget.
- ERR-006: `EngineError::StepCounterOverflow` - internal step counter invariant is violated; this is a typed fault, not panic.
- ERR-007: `ContractParityError` - active ResourceContract representations or validation diagnostic mappings diverge without an approved alias/waiver.

## Contract Signatures
- `fn compute_workflow_bound_certificate(workflow: CompiledWorkflow, contract: ResourceContract) -> Result<WholeWorkflowBudget, BudgetError>`
- `fn validate_budget(workflow: CompiledWorkflow, contract: ResourceContract, policy: BoundednessPolicy) -> Result<WholeWorkflowBudget, WorkflowError>`
- `fn aggregate_budget_from_workflow(workflow: CompiledWorkflow) -> Result<AggregateResourceBudget, AggregateBudgetError>`
- `fn admit_run_with_budget(requested: AggregateResourceBudget, usage: AggregateResourceUsage, capacity: AggregateResourceCapacity) -> Result<RunAdmission, AggregateBudgetError>`
- `fn create_run_value_store(contract: ResourceContract) -> Result<ValueStore, CoreError>`
- `fn drive_deterministic_with_step_budget(state: RunState, budget: StepBudget) -> Result<EngineSignal, EngineError>`

These are contract shapes, not implementation signatures. Downstream agents must bind them to existing Rust APIs without inventing new production code solely from this document.

## Verus-Owned Clauses
- PRE-002, POST-003, INV-001, INV-003, INV-004: budget arithmetic, monotonicity, boundedness, and overflow rejection in `crates/vb_core/src/budget.rs` and existing `verification/verus/resource_budget.rs`, `budget_monotonic.rs`, `budget_bounded.rs`.
- POST-004, INV-005, ERR-004: ValueStore arena cap model in `crates/vb_core/src/value_store.rs` and existing `verification/verus/value_store_invariant.rs`; exact Verus spec/proof surface is `spec_value_store_cap`, `spec_check_arena_cap`, `proof_arena_cap_enforced`, `proof_cap_exactly_rejects_insert`, `proof_check_arena_cap_gate`, and `proof_total_never_exceeds_cap`. Kani must additionally cover exact `CoreError::BudgetExceeded { budget: "max_slots", limit }` parity.
- PRE-005, POST-005, INV-006: StepBudget clamp/try_take invariants in `crates/vb_core/src/engine/signals.rs` and `verification/verus/step_budget.rs`.

## TLA+-Owned Clauses
- POST-001, POST-002, POST-005, INV-002, INV-007: admission-to-runtime lifecycle, capacity reservation, fail-closed rejection, and deterministic terminal outcome are temporal state-over-time properties.

## Theorem-Owned Clauses
- None required at State 3. Verus owns Rust-local arithmetic/state invariants; TLA+ owns lifecycle. Lean/Aeneas/Hax is a non-goal unless reviewer finds a tiny arithmetic theorem beyond Verus.

## Non-goals
- No production implementation, tests, fuzz harnesses, Verus code, Lean code, or TLA+ model code are authored in State 3.
- No speedup claim is made. Existing benchmark files are evidence surfaces only until executed against a baseline.
- No release-wide proof that unrelated workspace debt is fixed.
