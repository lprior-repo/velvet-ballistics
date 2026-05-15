# Proof Strategy: vb-qi37.4.2

## State Boundary

- State: 4, proof planning only.
- Workspace: `/home/lewis/src/vb-femdation/vb-qi37-4-2`.
- Artifact write scope: `.beads/vb-qi37.4.2/` only.
- Forbidden write scope: `/home/lewis/src/velvet-ballistics` and all production code, tests, proof code, harnesses, TLA/Lean/Verus/Kani specs, dependencies, and CI config.
- This artifact is a plan. It does not approve proof execution, close `vb-qi37.4.2`, or unblock any downstream bead.

## Skill Basis

- Read/cited proof-planner skill requirements.
- Proof-planner decides what must be proven and writes planning artifacts only, without proof code, tests, production code, harnesses, models, specs, dependencies, or CI config.
- Workspace-scoped discovery and recording of blocked discovery if a command cannot run.
- No hallucinated verifier results; stable IDs, exact artifact paths/commands, explicit assumptions/model bounds, and explicit skipped-lane waiver rows required.

## Inputs Read

- `.beads/vb-qi37.4.2/STATE.md`: current State 4 proof-planning after State 3 contract completion.
- `.beads/vb-qi37.4.2/contract.md`: State 3 contract with preconditions, postconditions, invariants, error taxonomy, and contract signatures.
- `.beads/vb-qi37.4.2/lean-contract.md`: theorem kernel projection; no Lean/Aeneas/Hax required; all Rust-local pure properties in Verus.
- `.beads/vb-qi37.4.2/verification-layers.md`: layer assignments for Verus L4, Kani L3, TLA+ L3, proptest L1, fuzz L2, loom L3, and static-scan L0.
- `.beads/vb-qi37.4.2/tla-spec.md`: TLA+ owned temporal behavior: journal ordering, replay safety, concurrency, retry FSM, capability lifecycle.
- `.beads/vb-qi37.4.2/proof-obligations.jsonl`: 55 proof obligation rows from State 3 contract.
- `.beads/vb-qi37.4.2/traceability-matrix.jsonl`: traceability from requirements to proof obligations.

## Discovery Evidence

- Required State 4 planning inputs confirmed non-empty: `STATE.md`, `contract.md`, `lean-contract.md`, `verification-layers.md`, `tla-spec.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`.
- Contract defines 15 preconditions (PRE-001 to PRE-005, plus PRE-006), 10 postconditions (POST-001 to POST-010), 15 invariants (INV-001 to INV-015), and 88 error variants.
- Verus-owned clauses: INV-001 to INV-006 (taint lattice), INV-007 (RunFrame dimensions), INV-008 (StepBudget monotonicity), INV-010 (EngineSignal canonical form), VB-CORE-RESOURCE-001 to VB-CORE-RESOURCE-003 (resource arithmetic).
- TLA+-owned clauses: INV-013 (journal ordering), VB-REPLAY-001 to VB-REPLAY-007 (replay safety), VB-CONC-001 to VB-CONC-005 (concurrency).
- No theorem kernel projection required (lean-contract.md WAIVER-LEAN-01, WAIVER-LEAN-02).

## Risk Classification

- `proof-complexity`: high. Taint lattice (6 proofs), StepState machine (8-state transition matrix), resource arithmetic (3 proof modes) require careful Verus encoding.
- `layer-alignment`: critical. 55 obligations span Verus L4, Kani L3, TLA+ L3, proptest L1, fuzz L2, loom L3, static-scan L0; must track mode and rerun_from correctly.
- `scope-control`: high. Generated codegen parity is non-goal; IPC/Record decode fuzz at L2 is compensating evidence only.
- `waiver-integrity`: medium. VB-CORE-TAINT-006 deferred to Kani (WAIVER-VRF-01); VB-CORE-IDX-002 static-scan only; supply chain non-goal (WAIVER-VRF-03).
- `path-consistency`: medium. TLA+ specs referenced by path; exact commands must use verified paths.

## Planned Verifier Lanes

### Verus L4 (Deductive Proof)

- `VB-CORE-TAINT-001` to `VB-CORE-TAINT-006`: taint lattice laws (associative, commutative, idempotent, identity, no-downgrade-Secret, no-downgrade-DerivedFromSecret).
- `VB-CORE-SIGNAL-001`: EngineSignal Finished canonical form carries (SlotValue, Taint).
- `VB-CORE-STATE-001-VERUS`: StepState valid transition matrix.
- `VB-CORE-BUDGET-003-VERUS`: StepBudget try_take monotonicity and no underflow.
- `VB-CORE-RESOURCE-001` to `VB-CORE-RESOURCE-003`: resource arithmetic (sequential sum, branch max, loop multiply).

### Kani L3 (Bounded Model Check)

- `VB-CORE-TAINT-006-KANI`: taint propagation in EvalExpr/BuildObject/BuildList (DRIFT-SECTION-68 correction).
- `VB-CORE-STATE-001-KANI`, `VB-CORE-STATE-002`: StepState transition matrix bounded check.
- `VB-CORE-BUDGET-001`, `VB-CORE-BUDGET-002`: budget 0/1 execution traces.
- `VB-CORE-BUDGET-003-KANI`: try_take bounded underflow check.
- `VB-CORE-IDX-001`: index access bounds validated.
- `VB-CORE-RESOURCE-004`: WholeWorkflowBudget bounded by policy.
- `VB-IPC-DECODE-001` to `VB-IPC-DECODE-003`: IPC header validation before allocation.
- `VB-STORAGE-DECODE-001` to `VB-STORAGE-DECODE-005`: Record decode validation before allocation.
- `VB-EXPR-002`: expression stack depth ≤ MAX_EXPR_STACK.

### TLA+ L3 (Model Check)

- `VB-REPLAY-001`: journal entry validity and monotonic sequence.
- `VB-REPLAY-002`: replay order preserved.
- `VB-REPLAY-003`: no duplicate replay.
- `VB-REPLAY-004`, `VB-REPLAY-005`: retry FSM max attempts and backoff.
- `VB-REPLAY-006`, `VB-REPLAY-007`: capability unique owner and valid access.
- `VB-CONC-001` to `VB-CONC-005`: concurrency control (single shard owner, no cross-shard alias, deadlock freedom, frame pool liveness, lock no starvation).

### Loom L3 (Concurrency Interleaving)

- `VB-CONC-LOOM`: shard concurrency frame pool operations race-free.

### Proptest L1 (Property Tests)

- `VB-CORE-STATE-003`: invalid StepState transitions return error.
- `VB-CORE-RESOURCE-004-PROP`: WholeWorkflowBudget within policy.
- `VB-EXPR-001`: AST/bytecode equivalence.
- `VB-UI-MODEL-envelope-001`, `VB-UI-MODEL-envelope-002`: envelope roundtrip and JSON handling.

### Fuzz L2 (Adversarial Input)

- `VB-IPC-DECODE-FUZZ`: IPC decoder arbitrary bytes corpus.
- `VB-STORAGE-DECODE-006`: record decode full pipeline corpus.
- `VB-EXPR-003`: f64 operations in expression evaluator.

### Static-Scan L0

- `VB-CORE-IDX-002`: no raw as_usize followed by direct indexing in hot paths.
- `SRC-LINT-001`: no unsafe code in forbid-unsafe crates.
- `SRC-LINT-002`: no panic in forbid-panic crates.

### Gauntlet (Gate)

- `GATE-001`: passes all proof-targeted verification lanes (L3/L4).
- `GATE-002`: passes all L0/L1/L2 gates (clippy, forbidden-scan, nextest, fuzz, loom).

## Planned State Transitions

- State 5: Proof writing — Verus specs/proofs, Kani harnesses, TLA+ models, proptest cases.
- State 6: Proof review — adversarial review of proof artifacts.
- State 7: Test execution — run proof lanes, collect evidence.
- State 8: Formal verification — run Verus, Kani, TLC, collect reports.
- State 9: Evidence packaging — assemble proof-obligations.verified.jsonl.
- State 10: Truth serum — audit AI-generated proof artifacts.
- State 11: Black-hat review — adversarial review of full delivery.
- State 12: Final delivery gate.

## Output Ledger

- `.beads/vb-qi37.4.2/proof-obligations.planned.jsonl` contains 55 rows with `status:"planned"`, one for every current `proof-obligations.jsonl` ID.
- Every planned row includes: `id`, `contract_clause`, `target`, `claim`, `layer`, `checker`, `command`, `evidence`, `expected_evidence`, `risk`, `scope`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, and where applicable: `verus_target`, `spec_fn`, `proof_fn`, `invariants`, `trusted_boundary`, `shell_exclusions`, `tla_module`, `model`, `config`, `variables`, `actions`, `invariants`, `temporal_properties`, `fairness`, `state_constraints`, `refinement`.
- No blocked rows; all obligations are planned with appropriate lane assignment.
- Waiver rows for deferred/scope-excluded obligations include `waiver`, `owner_state`, `rerun_from`, `status:"waived"`, `reason`, `compensating_evidence`, and `rerun_trigger`.
# State 4 Rerun Addendum: vb-qi37.4.2

STATUS: UPDATED

State 3 contract/traceability repair added exact RunFrame obligations (`VB-CORE-RUNFRAME-001..003`), added `INV-014` idempotency traceability (`VB-CORE-IDEMPOTENCY-001`), reconciled `POST-010` resource arithmetic to saturating policy-bounded semantics, and rewrote waivers with owner/expiry/follow-up conditions.

`proof-obligations.planned.jsonl` was regenerated from the repaired `proof-obligations.jsonl` and now contains 59 planned rows. Downstream proof writing must create/repair `verification/verus/run_frame_invariant.rs` coverage, update resource proof names/semantics, execute or validly waive non-Verus/TLA required rows, and add TLC cfg property/deadlock checks before State 6 approval can be requested.
