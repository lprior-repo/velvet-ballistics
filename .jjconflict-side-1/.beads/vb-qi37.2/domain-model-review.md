# Domain Model Review: vb-qi37.2

## Status
STATUS: CONTRACT_READY_WITH_OPEN_PARITY_QUESTIONS

This is a State 3 domain/type review by the rust-contract agent. It is not an independent approval and must be reviewed later by contract-verification-reviewer.

## Ubiquitous Language Check
- WholeWorkflowBudget is the computed certificate surface for static whole-workflow bounds.
- BoundednessPolicy is the global safety ceiling and policy validator.
- ResourceContract is the per-workflow cap surface used by validation and runtime admission.
- AggregateResourceBudget/Capacity/Usage are runtime admission accounting surfaces.
- ValueStore with max slots is the per-run arena cap surface.
- StepBudget is the deterministic per-tick execution ceiling surface.

## Type Model Contracts
- Illegal unbounded runtime states must be unrepresentable at admission: accepted runs require finite ResourceContract dimensions and a computed bound certificate.
- Aggregate reservation must be explicit: a run cannot be acknowledged without requested budget fitting within capacity.
- Fixture-only uncapped ValueStore construction must not be reachable through production runtime submission.
- Step budget exhaustion is a normal typed terminal/blocking signal, not exceptional process failure.

## Parity Risks
- PARITY-001: `validation.rs` and `workflow/mod.rs` both map budget errors; State 2 reported non-identical diagnostic details for some dimensions. Downstream implementation/review must classify as approved aliases or fix drift.
- PARITY-002: `compiled_workflow.rs` has a separate ResourceContract shape missing fields reported in canonical workflow ResourceContract. Downstream implementation/review must prove it is legacy/dead or restore parity.
- PARITY-003: `BoundednessPolicy::DEFAULT.max_total_slots` and `ResourceContract::DEFAULT.max_slots` have different values. Contract stance: global safety ceiling and per-workflow default are distinct, and accepted contracts must satisfy both.

## DDD Boundary Split
- Core domain: compiled IR budget computation, checked arithmetic, policy validation, ResourceContract semantics, ValueStore cap invariants, StepBudget invariants.
- Runtime shell: admission persistence/acknowledgment, capacity reservation, run-state creation, deterministic execution loop.
- External systems excluded from Rust-local domain proof: wall-clock scheduling, storage durability details, CLI/UI, YAML parsing.

## Required Follow-up Before Implementation Consumption
- Independent `contract-verification-review.md` must approve or reject this contract.
- Test planning must include explicit parity scenarios for PARITY-001 and PARITY-002.
- Proof planning must bind Verus obligations to actual proof files or mark missing proof targets as blocked rather than inventing modules.
