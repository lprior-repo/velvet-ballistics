# Proof Strategy: vb-qi37.2 State 4 Attempt 3

## Status

Planning only. No production source, tests, proof models, harnesses, specs, dependency files, or source checkout files were edited. Previous State 4/5/6 proof outputs are invalid where repaired State 3 changed obligations.

## Inputs Read

- Repaired State 3: `contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `codebase-map.md`.
- State 6 rejection context: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`.
- Prior proof evidence as context only: `proof-evidence.md`, `proof-writer-report.md`, `verification/tla/WorkflowBoundedAdmission.tla`, `verification/tla/WorkflowBoundedAdmission.cfg`.

## Discovery Evidence

- `pwd -P` -> `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2`.
- `test -s ".beads/vb-qi37.2/contract.md"` -> exit 0.
- `test -s ".beads/vb-qi37.2/traceability-matrix.jsonl"` -> exit 0.
- `test -s ".beads/vb-qi37.2/delivery-scope.jsonl"` -> exit 0.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" ...scoped paths...` -> exit 0; found boundedness, state, transition, serialization, assertion, and panic-in-test/proptest risk triggers in scoped Rust/TLA files.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" ...scoped paths...` -> exit 0; found Verus proof functions, existing Kani harnesses in `budget.rs`, proptest surfaces, Miri attributes, and TLA artifacts.
- No discovery command was blocked.

## Risk Classification

- Temporal lifecycle risk: TLA+ remains required for certificate-before-ack, aggregate reservation/rejection before ack, fail-closed states, finite step exhaustion, and deterministic terminal outcome class.
- Rust-local invariant risk: Verus remains required for budget finite/policy bounds, nested monotonicity, checked arithmetic, step budget monotonicity, and ValueStore cap invariants.
- Bounded-model parity risk: Kani is required for concrete aggregate budget field behavior, ValueStore exact `CoreError::BudgetExceeded { budget: "max_slots", limit }`, and existing budget helper add/sub harnesses.
- Untrusted/adversarial input risk: proptest/fuzz are required for nested workflow and aggregate budget fail-closed behavior.
- UB/handle risk: Miri is required only as second-ring ValueStore cap/handle evidence and does not replace Verus/Kani.
- Static/runtime boundary risk: `moon ci` remains the canonical static gate for source restrictions and runtime-core no YAML/JSON/HTTP policy.
- Performance risk: Criterion evidence is optional unless implementation touches hot budget paths or claims performance.

## Repair-Driven Plan

- State 5 must repair or explicitly block the TLA deadlock/refinement findings from State 6 before claiming TLA coverage. Prior TLC pass with `CHECK_DEADLOCK FALSE` is context only, not sufficient final evidence.
- State 5 must execute Verus rows for `resource_budget.rs`, `budget_monotonic.rs`, `budget_bounded.rs`, `step_budget.rs`, and `value_store_invariant.rs`, or record precise reviewer-accepted blockers.
- State 5 must execute or create exact Kani harnesses for aggregate admission and ValueStore cap parity. Existing budget add/sub Kani harnesses are exact and should be run as additional bounded-model parity evidence.
- State 5 must run focused `cargo test`/proptest and parity commands, plus reviewer source classification for `compiled_workflow.rs` ResourceContract active/legacy status.
- State 12/formal-verifier may own deep fuzz, Miri, static CI, and performance evidence, but required rows cannot be silently deferred at State 6 without blocker/waiver classification.

## Waivers And Not Applicable Lanes

- Lean/Aeneas/Hax theorem kernel: not applicable unless a later reviewer identifies theorem-owned arithmetic outside Verus.
- Flux: not applicable because repaired State 3 has no Flux annotations or Flux-owned requirements.
- Loom: not applicable; no concurrency or memory-ordering model risk is traced for this bead.

## Outputs

- `.beads/vb-qi37.2/proof-obligations.planned.jsonl` contains planned, not-applicable, or waiver rows only. No row claims verifier pass status.
- `.beads/vb-qi37.2/proof-plan-review-input.md` summarizes this plan for proof-reviewer.
