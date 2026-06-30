# Domain Model Review: Bounded Nested Workflow Composition

## Verdict
STATUS: PLANNED_FOR_REVIEW

The corrected State 3 domain is whole-workflow boundedness, not unrelated action ABI, replay, or data-sensitivity work. The model must make unbounded nested composition impossible to admit and must make the structural source of aggregate growth visible in diagnostics.

## Core Ubiquitous Language
- Workflow artifact: trusted compiled IR candidate awaiting verification.
- Primitive region: structural subgraph owned by one loop/fanout primitive.
- Leaf cost: bounded primitive-local cost before composition.
- Sequential composition: sum of costs.
- Conditional composition: max of possible branch costs.
- Nested composition: multiplication of outer iteration/fanout factor by inner region cost.
- Parallel composition: bounded branch fanout plus conservative aggregation of branch costs.
- Growth diagnostic: cold-path explanation linking budget failure to primitive, node index, and structural path.

## Aggregates and Boundaries
- `CompiledWorkflow` is the candidate aggregate. It is not accepted until boundedness analysis succeeds.
- `WholeWorkflowBudget` is the derived verifier aggregate. It must be immutable evidence for runtime admission.
- `BoundednessPolicy` is the operator/global guardrail aggregate.
- `AggregateResourceBudget` is the runtime reservation aggregate derived from `WholeWorkflowBudget`.
- Diagnostics are cold validation outputs; they must not pollute hot runtime state.

## Required Rules by Primitive
- `collect`: requires finite items/pages/time; nested body cost is multiplied by the finite collect iteration/page bound; diagnostics name collect node and missing/exceeded bound.
- `reduce`: requires finite input/list bound; reducer body cost is multiplied by the item bound; diagnostics name reducer node and accumulator/input growth source.
- `repeat`: requires finite attempts or time; body cost is multiplied by attempts; diagnostics name repeat node and missing/exceeded attempt/time bound.
- `together`: requires branch count `<= policy`; parallel in-flight is at least branch count and nested branch costs are aggregated conservatively; diagnostics name branch path.

## Illegal States
- A workflow accepted with unknown `collect`, `reduce`, `repeat`, or `together` aggregate growth.
- A workflow accepted after arithmetic overflow in cost composition.
- A workflow admitted at runtime without an `AggregateResourceBudget` produced from verified whole-workflow analysis.
- A diagnostic that says only `limit exceeded` without node/primitive/path provenance.
- Runtime admission falling back to YAML, JSON, strings, or ad-hoc recomputation of structure.

## Review Risks
- Existing code may compute some fields as maxima rather than aggregate sums; proof/review must decide per dimension which lattice is correct.
- Existing `reduce` bound appears tied to `MAX_LIST_ITEMS_PER_VALUE`; proof must verify this is a real policy/input bound, not a disguised unbounded default.
- Existing `BoundedAdmission.tla` may be too generic and may need a later proof-writer repair to model verified-vs-unverified budget status explicitly.
- Diagnostics may require a cold metadata layer not visible in `BudgetError`; this contract requires it without prescribing implementation.

## Acceptance Questions for Independent Reviewer
1. Does every required primitive (`collect`, `reduce`, `repeat`, `together`) have bounded growth rules?
2. Are sum/max/multiply composition rules unambiguous?
3. Are unbounded and overflow cases fail-closed before admission?
4. Do diagnostics identify structural source of growth?
5. Do proof obligations include `owner_state` and `rerun_from`?

## Status / Evidence Summary
- Scope repaired to bounded nested workflow composition.
- No production code or tests changed.
- Independent contract-verification review remains required before proof planning consumes this artifact.
