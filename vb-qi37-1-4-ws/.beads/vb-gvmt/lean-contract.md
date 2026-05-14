# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: action/ask suspension and resume lifecycle, journal ordering, terminal completion, step-budget behavior.
- Verus-owned Rust core: bounded array access, journal capacity arithmetic, taint lattice monotonicity, pure resume transition pre/postconditions, no mutation before identity validation.
- Theorem-owned kernel: none required initially.
- Rust/runtime shell: generated source emission, concrete runtime adapters, action execution, ask external answer collection, journal persistence.
- External systems excluded from theorem proof: action executor, IPC, Fjall, postcard encoding, wall clock, and OS process execution of generated files.

## Theorem-Owned Clauses
- None at contract time.

## Rationale
- The critical algebra here is the three-point taint lattice and bounded state transition preservation. That is intentionally small enough for Verus specs/proofs over an abstract model.
- Lean/Aeneas/Hax must not be introduced unless Verus cannot express the chosen abstraction or an independent reviewer requires a tiny theorem kernel for refinement.

## Waiver Candidate
- THM-WAIVE-001: Lean theorem kernel deferred.
  - Owner: future proof implementation bead.
  - Reason: no theorem beyond Verus-owned taint/order/state invariants has been identified.
  - Expiry: before independent contract-verification approval if reviewer rejects Verus-only theorem stance.
  - Compensating evidence: Verus obligations `VERUS-TAINT-001`, `VERUS-RESUME-001`, `VERUS-JOURNAL-001`; TLA+ obligation `TLA-GEN-001`; Fowler scenarios in `martin-fowler-tests.md`.
