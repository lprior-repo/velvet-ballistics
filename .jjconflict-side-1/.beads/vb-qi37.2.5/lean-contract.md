# Theorem Kernel Projection - vb-qi37.2.5

## Boundary
- TLA+-owned temporal model: execution-slice exhaustion, finite admission/rejection lifecycle, and value-growth rejection temporal behavior.
- Verus-owned Rust core: step-budget monotonicity and resource-budget composition arithmetic.
- Theorem-owned kernel: none selected for State 3.
- Rust/runtime shell: engine stepping, workflow constructors, validation, arena storage, fuzz inputs, I/O-free tests.
- External systems excluded from theorem proof: action handlers, storage persistence, generated runtime chunks, wall-clock time, process memory, and OS scheduler behavior.

## Theorem-Owned Clauses
- None.

## Rationale
- Existing Verus proof surfaces already model the relevant arithmetic kernels:
  - `verification/verus/step_budget.rs`: `can_take`, `remaining_after_take`, `lemma_try_take_success_never_underflows`, `lemma_try_take_failure_preserves_remaining`, `lemma_try_take_monotonic`.
  - `verification/verus/resource_budget.rs`: `SpecBudget`, `sat_add`, `sat_mul`, `sequential_compose`, `branch_compose`, `loop_compose`, `policy_within`, and boundedness lemmas.
- No tiny kernel currently requires Lean/Aeneas/Hax beyond Verus. Introducing Lean here would add ceremony without a smaller theorem boundary.

## Waivers
- LEAN-WAIVER-001: Lean/Aeneas/Hax theorem projection waived for State 3.
  - Owner: proof-planner/proof-reviewer.
  - Reason: Verus owns the Rust-local arithmetic and state-transition clauses; TLA+ owns temporal lifecycle clauses.
  - Expiry: revisit if proof-review rejects Verus expressiveness for INV-001 or INV-006.
  - Compensating evidence: `verus verification/verus/step_budget.rs`, `verus verification/verus/resource_budget.rs`, TLA+ model obligation, Kani/proptest/fuzz obligations.
