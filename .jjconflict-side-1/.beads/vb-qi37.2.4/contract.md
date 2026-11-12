# Contract Specification: vb-qi37.2.4 State 3

## Context
- Feature: bounded nested workflow composition verification for `collect`, `reduce`, `repeat`, and `together` before runtime admission.
- Bead: `vb-qi37.2.4`.
- Authoritative scope: verifier checks nested fanout/composition, accepts aggregate bounded workflows, rejects unbounded nested composition, and emits diagnostics naming structural growth sources.
- Authoritative plan references: `velvet-ballistics-MASTER.md` Sections 13, 37, 64, and DRIFT-3.
- Existing production loci to be verified, not edited by this State 3 artifact repair: `crates/vb_core/src/budget.rs`, `crates/vb_validate/src/gate_12_14_15.rs`, `verification/verus/budget_bounded.rs`, `specs/tla/BoundedAdmission.tla`.

## Domain Terms
- `ResourceContract`: declared per-workflow static limits.
- `WholeWorkflowBudget`: verifier-computed conservative per-run upper bound from IR plus declared limits.
- `BoundednessPolicy`: absolute acceptance ceiling applied across workflows.
- `AggregateResourceBudget`: admission reservation shape derived from the accepted whole-workflow budget.
- `composition`: sequential, conditional, nested-loop, or parallel branch combination of IR regions.
- `growth source`: the primitive and structural path responsible for multiplied or maximized cost.

## Assumptions
- Final IR is the verification input; YAML AST is out of scope except as the cold authoring source that produced IR.
- `for_each` participates in nested composition because Section 64 cites nested fanout examples, but bead acceptance specifically requires `collect`, `reduce`, `repeat`, and `together` diagnostics.
- Existing TLA+/Verus artifacts may need later proof-code repair; this State 3 pass specifies obligations only.

## Preconditions
- PRE-001: Input workflow artifact exposes `WorkflowParts`/`CompiledWorkflow` with finite `nodes`, `entry`, and `ResourceContract`.
- PRE-002: `ResourceContract` fields used in composition are explicit and finite; sentinel maxima (`u16::MAX`, `u32::MAX`, `u64::MAX`) are not acceptable as defaults for fanout, collect items, or tick budget.
- PRE-003: Each `collect`, `reduce`, `repeat`, and `together` node carries enough structural metadata to identify its body/done/branch region and declared limit.
- PRE-004: `BoundednessPolicy` is available before artifact acceptance.

## Postconditions
- POST-001: Every accepted workflow has a `WholeWorkflowBudget` whose dimensions are known and `<= ResourceContract` where applicable and `<= BoundednessPolicy` everywhere required.
- POST-002: Sequential composition sums bounded costs with checked arithmetic.
- POST-003: Conditional composition takes the maximum bounded branch cost, never the minimum or unchecked sum of alternatives.
- POST-004: Nested loop composition multiplies outer and inner bounded costs with checked arithmetic and rejects overflow or unknown factors.
- POST-005: `together` composition contributes bounded branch count to `max_parallel_in_flight`/fanout and combines branch costs conservatively.
- POST-006: `collect` contributes bounded page/item growth; missing page/item/time bounds are rejected before runtime admission.
- POST-007: `reduce` contributes bounded iteration growth derived from a proven finite input/list bound, not an implicit unbounded collection.
- POST-008: `repeat` contributes bounded attempt/time growth; missing attempt/time bounds are rejected before runtime admission.
- POST-009: Rejected workflows return typed boundedness errors that include the resource dimension, actual/computed value when known, policy/contract limit, primitive kind, node/step index, and structural path to the growth source.
- POST-010: Accepted aggregate budgets are materialized before runtime admission so shard capacity reservation can fail closed.

## Invariants
- INV-001: No admitted run lacks a prior aggregate resource reservation.
- INV-002: Budget arithmetic is monotone: adding bounded sequential work cannot reduce any budget dimension.
- INV-003: Budget arithmetic is checked: overflow during sum or multiplication rejects the workflow.
- INV-004: Unknown bounds are rejection conditions, not warnings, defaults, or runtime discovery.
- INV-005: Diagnostics preserve structural provenance for nested growth: root path, primitive sequence, node index, resource dimension, actual, and limit.
- INV-006: Runtime admission only consumes `AggregateResourceBudget` derived from a verified `WholeWorkflowBudget`; runtime never recomputes or guesses unbounded YAML semantics.

## Error Taxonomy
- `BudgetError::TotalStepsExceeded { actual, limit }` - computed executable steps exceed policy.
- `BudgetError::FanoutExceeded { actual, limit }` - `together`/nested fanout exceeds policy.
- `BudgetError::NestingDepthExceeded { actual, limit }` - structural nesting exceeds policy.
- `BudgetError::ActionTicketsExceeded { actual, limit }` - nested action-ticket growth exceeds policy.
- `BudgetError::ParallelExceeded { actual, limit }` - parallel in-flight bound exceeds policy.
- `BudgetError::StepsExecutableExceeded { actual, limit }` - executable step ceiling exceeded.
- `AggregateBudgetError::PolicyExceeded { resource, actual, limit }` - aggregate budget exceeds `BoundednessPolicy`.
- `AggregateBudgetError::CapacityExceeded { resource, requested, available }` - shard capacity cannot reserve the accepted aggregate budget.
- `AggregateBudgetError::Overflow { resource }` - checked addition/multiplication overflow while composing bounds.
- Required diagnostic extension: each boundedness error surfaced to users must attach `primitive`, `node`, and `structural_path` fields or equivalent cold diagnostic metadata.

## Contract Signatures
- `WholeWorkflowBudget::compute(nodes: &[CompiledNode], entry: StepIdx, contract: &ResourceContract) -> Result<WholeWorkflowBudget, WorkflowError>`.
- `BoundednessPolicy::validate(&self, budget: &WholeWorkflowBudget) -> Result<(), BudgetError>`.
- `AggregateResourceBudget::from_workflow(workflow: &CompiledWorkflow) -> Result<AggregateResourceBudget, AggregateBudgetError>`.
- `validate_aggregate_budget(budget: &AggregateResourceBudget, policy: &BoundednessPolicy) -> Result<(), AggregateBudgetError>`.
- Future diagnostic contract: `explain_boundedness_failure(error, workflow_parts) -> Diagnostic` must be total over all budget failure variants.

## Verus-Owned Clauses
- PRE-002, POST-001..POST-008, INV-002, INV-003, INV-004: pure budget arithmetic, monotonicity, checked overflow behavior, and reject-on-unknown/refinement obligations.

## TLA+-Owned Clauses
- INV-001 and INV-006: admission state machine never admits without verified/reserved aggregate budget and never consumes unverified workflow budgets.

## Theorem-Owned Clauses
- None required for State 3. Verus is sufficient for Rust-local budget arithmetic; TLA+ owns admission ordering.

## Required Verifier Lanes
- `moon run :verify-proof` for TLA+/Verus/Kani proof lane after proof artifacts exist or are repaired.
- `moon run :verify-deep` for proptest, fuzz smoke, Miri, coverage, and mutation defense-in-depth.
- `moon run :verify-standard` for normal compile/test/lint/property confidence.

## Status / Evidence Summary
- Status: planned contract repair only; no production code, proof code, or tests edited.
- Evidence basis: mandatory startup skills read from `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md`; both require contract-first artifacts, TLA+ for temporal admission, Verus for Rust-local pure budget logic, JSONL obligations with `owner_state` and `rerun_from`.
- This artifact replaces the incorrect prior State 3 scope with bounded nested workflow composition scope.

## Non-goals
- No unrelated action ABI, replay, or data-sensitivity contract changes.
- No production code, tests, TLA+ model code, Verus code, or Lean code changes in this State 3 repair.
- No use of `/home/lewis/src/velvet-ballistics` as an editable source checkout.
