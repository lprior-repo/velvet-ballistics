# TLA+ Temporal Model Plan — vb-core-lower-coverage-matrix

## Boundary
- **Temporal/workflow behavior**: NOT APPLICABLE - this bead proves static acceptance/rejection parity
- **Rust/core behavior excluded from TLA+**: Compiler acceptance/rejection classification
- **External systems abstracted**: NOT APPLICABLE
- **Non-applicability rationale**: This bead verifies that the compiler produces consistent accept/reject results for discrete YAML inputs. There are no event-driven state transitions, workflows, protocols, schedulers, queues, retry logic, claims, leases, liveness properties, or concurrency concerns. TLA+ model-checking is inappropriate for this static parity matrix work.

## TLA+-Owned Clauses
None.

## Model Shape
Not applicable.

## Properties
Not applicable.

## Evidence Command
Not applicable.

## Waivers
- No TLA+ model for INV-001 through INV-004: Static compiler behavior, not temporal
- Owner: vb-core-lower-coverage-matrix State 3
- Reason: Coverage matrix proves discrete input/output classification, not state machine transitions
- Compensating evidence: Unit tests in `v1_primitive_lowering.rs` + Verus proofs in `verification/verus/v1_primitive_lowering.rs`
