# TLA+ Temporal Model Plan: vb-m5gp

## Boundary

- Temporal/workflow behavior: none introduced or changed by this bead.
- Rust/core behavior excluded from TLA+: private module extraction, public API compatibility, source-length governance, deterministic compiler parity.
- External systems abstracted: none.
- Non-applicability rationale: this is a single-crate structural refactor of synchronous compiler code. It has no scheduler, queue, retry, claim/lease, lifecycle protocol, concurrency, distributed coordination, fairness, or liveness behavior to model. Creating a TLA+ model here would be fake assurance.

## TLA+-Owned Clauses

- None.

## Model Shape

- Module/model path: none planned.
- Variables: not applicable.
- Init action: not applicable.
- Next/actions: not applicable.
- State constraints: not applicable.
- Symmetry sets: not applicable.
- Bounded model limits: not applicable.

## Properties

- Safety invariants: structural/API invariants are assigned to static scan, compile, API, Miri/Kani, and tests in `verification-layers.md`.
- Liveness/eventuality: not applicable.
- Fairness assumptions: not applicable.
- Deadlock freedom: not applicable.
- Refinement to Rust/runtime behavior: not applicable.

## Evidence Command

- No `tlc` or `apalache` command is required for this bead.

## Waiver

- TLA-WAIVER-001: TLA+ waived for all clauses because the requested change has no temporal/state-over-time semantics. Owner: State 3 contract. Expiry: this bead only. Compensating evidence: `cargo +nightly test -p vb_compile --all-targets --all-features`, workspace compile integration tests, Kani idempotency parity if available, static source-structure checks, and API compatibility checks.
