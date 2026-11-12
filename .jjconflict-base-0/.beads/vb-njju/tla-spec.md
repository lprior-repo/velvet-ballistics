# TLA+ Temporal Model Plan

## Boundary

- Temporal/workflow behavior: none required for this bead. The work is static quality-gate closure and BDD evidence classification.
- Rust/core behavior excluded from TLA+: evidence parsing/classification and acceptance test assertions, handled by cargo tests, proptest where applicable, cargo-fuzz, cargo-mutants, and release gate tests.
- External systems abstracted: Moon task runner, cargo-fuzz, cargo-mutants, and boundary inventory reports are treated as finite evidence records.
- Non-applicability rationale: no scheduler, queue, retry, claim/lease, distributed state, concurrency, lifecycle transition system, or temporal liveness property is introduced by vb-njju.

## Evidence lattice model, non-executable planning note

TLA+ execution is waived because the release gate is a finite fail-closed predicate over evidence records. The intended lattice is:

- Evidence status values: `Missing`, `Weak`, `BlockedFollowup`, `Present`.
- Gate result values: `Pass`, `Fail`.
- Rule: required evidence `Missing` or `Weak` maps to `Fail` unless explicitly represented as `BlockedFollowup` and accepted by independent review.

This lattice is simple enough to verify by BDD/property/mutation checks without a separate TLC model.

## TLA+-owned clauses

- None.

## Model shape

- Module/model path: not created; TLA+ non-applicability waiver.
- Variables: not applicable.
- Init action: not applicable.
- Next/actions: not applicable.
- State constraints: not applicable.
- Symmetry sets: not applicable.
- Bounded model limits: not applicable.

## Properties covered outside TLA+

- Safety invariant: release gate never passes when admission mutation closure is missing or unrelated.
- Safety invariant: release gate never treats build-only fuzz target discovery as fuzz-run evidence.
- Safety invariant: generated-vs-IR property gate fails when taint comparison is omitted.
- Safety invariant: release gate never passes when unsafe boundary fuzz evidence is missing without approved blocker/follow-up.
- Liveness/eventuality: not applicable.
- Fairness assumptions: not applicable.
- Deadlock freedom: not applicable.
- Refinement to Rust/runtime behavior: acceptance tests consume finite evidence records and public catalog/quality APIs; no runtime transition refinement exists.

## Evidence command

- TLA+ checker: waived for vb-njju by non-applicability.
- Compensating commands are listed in `verification-layers.md` and `proof-obligations.jsonl`.

## Waivers

- TLA-WAIVE-001: TLA+ not applicable. Owner: State 3 contract. Reason: no temporal behavior; finite evidence fail-closed predicates are better covered by BDD/property/mutation. Expiry: State 4 implementation review; if State 4 introduces a stateful release workflow, add TLA+ model before implementation acceptance.
