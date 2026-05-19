# Theorem Kernel Projection - vb-0sps State 3 Repair

## Boundary

- TLA+-owned temporal model: separate IR/generated transition systems, block/resume, event ordering, unsupported rejection, and observation/refinement.
- Verus-owned Rust core when adapters exist: pure normalization and equality/refinement of observed parity records.
- Theorem-owned kernel: none currently required.
- Rust/runtime shell: public API calls, generated source emission, rustc/trybuild compile checks, and BDD execution harness.
- External systems excluded from theorem proof: action dispatcher, timers, ask responder, filesystem, compiler process, and journal storage adapters.

## Theorem-Owned Clauses

- None.

## Rationale

Lean/Aeneas/Hax is not justified for this bead because the critical kernels are temporal/state-over-time behavior better modeled in TLA+ or Rust-local pure comparison/refinement better specified in Verus after concrete adapters exist. Introducing a theorem kernel now would risk a vacuum proof not bound to production/test-support adapters.

## Waiver

- THM-WAIVER-001: Lean/Aeneas/Hax deferred. Owner: State 4 proof-planner and State 6 proof-reviewer. Reason: no tiny algebraic theorem beyond Verus/TLA+ identified. Limitation: no theorem-assistant proof is provided for parity algebra. Expiry/follow-up: revisit if State 5/implementation introduces a nontrivial normalized-event algebra not expressible cleanly in Verus. Compensating evidence: TLA+ temporal model, Verus-bound adapter obligations/waivers, BDD focused tests, and proptest/focused single-field-difference checks.
