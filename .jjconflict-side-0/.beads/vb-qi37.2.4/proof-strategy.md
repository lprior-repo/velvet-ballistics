# Proof Strategy: vb-qi37.2.4

## Scope
- Bead scope: bounded nested workflow composition verification for `collect`, `reduce`, `repeat`, and `together` fanout/composition.
- Acceptance claims: bounded nested workflows produce a conservative `WholeWorkflowBudget`, aggregate budget validation accepts only policy-fitting budgets, unbounded or overflowing composition rejects before runtime admission, and diagnostics name structural growth sources.
- Planning-only boundary: no production code, test code, proof code, TLA+ code, or Verus code is edited by this state.

## Discovery Evidence
- `pwd -P` returned `/home/lewis/src/vb-femdation/vb-qi37-2-4`.
- `test -s .beads/vb-qi37.2.4/contract.md` passed.
- `test -s .beads/vb-qi37.2.4/traceability-matrix.jsonl` passed.
- `test -s .beads/vb-qi37.2.4/delivery-scope.jsonl` passed.
- `crates/vb_core/src/budget.rs` contains `WholeWorkflowBudget`, `BoundednessPolicy`, `AggregateResourceBudget`, `validate_aggregate_budget`, checked aggregate add/sub, nested loop step counting, and fanout/nesting metric collection.
- `verification/verus/budget_bounded.rs` exists but currently proves only simple bounded step/add/overflow lemmas; nested multiplication, branch max, aggregate refinement, and diagnostic provenance proof surfaces are missing.
- `specs/tla/BoundedAdmission.tla` exists and models reservation-before-admission, but it has no explicit verified-budget state; admission without verified budget cannot yet be checked directly.

## Verifier Lanes
- TLA+ owns temporal admission ordering: runtime admission must require verified aggregate budget reservation before a run becomes admitted.
- Verus owns pure Rust-local budget arithmetic: sequential monotonic addition, nested collect/reduce/repeat multiplication, checked overflow rejection, conservative branch/together composition, and whole-to-aggregate refinement.
- Kani owns bounded panic-freedom and overflow/rejection exploration for concrete arithmetic helpers and nested composition harnesses.
- Proptest owns generated nested IR shapes and diagnostics: accepted bounded cases stay within policy, rejected cases expose primitive, node, structural path, actual, and limit.
- Deep CI owns defense in depth for Miri/fuzz/mutation/lint once proof/test artifacts exist.

## Required Repair Expectations
- `specs/tla/BoundedAdmission.tla` must gain explicit verified/rejected budget state before `TLA-ADM-002` can pass.
- `verification/verus/budget_bounded.rs` must gain nested multiplication, branch max/together, aggregate refinement, and checked metric proofs before Verus obligations can pass.
- Kani/proptest harnesses for `collect`, `reduce`, `repeat`, `together`, aggregate acceptance/rejection, and diagnostics may need to be added by later states.

## Non-Applicable Or Waived Lanes
- Loom is not required: the scoped claims are static budget/admission ordering claims, not concurrent scheduling or memory ordering claims.
- Miri is defense-in-depth only: the repository forbids unsafe code and the bead does not introduce unsafe or raw pointer claims.
- Lean/theorem projection remains waived by `verification-layers.md`; Verus/TLA+ are sufficient for this bead's contract.
