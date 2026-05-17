# TLA+ Temporal Model Plan — vb-qi37.1.5

## Non-applicability Rationale

**TLA+ is not applicable for this bead.**

The digest mismatch detection is a **pure deterministic function** over an immutable event stream:

- `check_workflow_source_digest`: reads `RunAccepted.workflow` from journal, compares with expected `WorkflowDigest` using byte-exact equality
- `check_compiled_ir_digest`: compares two `WorkflowDigest` values by byte equality
- `verify_digests`: orchestrates the above two checks at a requested `DigestCheck` level

There are **no temporal/state-machine properties** to model:

- No workflow protocol or lifecycle state machine
- No concurrent actors or message passing
- No retry/lease/claim logic
- No deadlock, liveness, or fairness concerns
- No distributed coordination
- No eventuality/liveness requirements

The property being verified is: `expected == found → Ok(()), expected ≠ found → Err(Mismatch)`. This is a pure function property that belongs in:
1. **Verus** (Rust-local pure logic, this module's own functions)
2. **Unit/property tests** (corruption injection)
3. **Kani** (bounded model check for state-transition safety)

A TLA+ model would not add discriminatory power over these alternatives for this specific bead.

## TLA+-Owned Clauses

None.
