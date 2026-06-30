<<<<<<< HEAD
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
=======
# Proof Strategy

Bead: `vb-qi37.4.2`
State: 4 proof planning repair attempt 4
Scope: isolated workspace `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`

## Inputs Read

- Repaired State 3 artifacts after contract status repair: `contract.md`, `verification-layers.md`, `martin-fowler-tests.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `tla-spec.md`, `lean-contract.md`, and `delivery-scope.jsonl`.
- State 6 rejection artifacts: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, and `contract-verification-review.md`.
- Prior proof evidence used only as context: `proof-evidence.md` and `proof-writer-report.md`.

## Discovery Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`.
- `test -s ".beads/vb-qi37.4.2/contract.md" && test -s ".beads/vb-qi37.4.2/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.4.2/delivery-scope.jsonl"` exited 0.
- Scoped risk scan over `delivery-scope.jsonl` paths found admission state/allocation, serialization/deserialization, queue/retry/cancel terms, and `#![forbid(unsafe_code)]` markers. It also found test-only `assert!` in scoped source files, which affects later lint/static review but does not create a proof-planner edit.
- Scoped verifier scan found existing Verus proof functions in `verification/verus/capability_artifact_model.rs` and `verification/verus/accepted_envelope_model.rs`; it found no `kani::`, `loom::`, `proptest!`, `fuzz_target`, or Flux proof hooks in the scoped runtime/storage/CLI source files.
- Discovery blockers: none for the required proof-planner discovery commands.

## Rejection Repair Deltas Reflected

- `VERUS-ENV-006` is now an executable Verus planning row targeting `verification/verus/accepted_envelope_model.rs` with command `verus verification/verus/accepted_envelope_model.rs`.
- TLA+/Verus evidence expectations now reference existing `proof-evidence.md` sections rather than missing `tla-report.md` or `verus-report.md` files.
- `PO-007`, `PO-008`, `PO-009`, `PO-011`, and `PO-012` are planned downstream evidence-policy rows with owner, reason, expiry, limitation, and compensating evidence in `waiver_policy`. They remain `status:"planned"` and do not claim pass or waiver approval evidence at contract/planning time.
- `TEST-STRICT-009` expected evidence now enumerates exact ERR-001 through ERR-008 diagnostic scenarios.
- `PO-010` remains required and executable for later static/lint verification because it has concrete scoped source targets and an exact command.

## Risk Classification

- Temporal admission lifecycle: strict/journaled denial must occur before frame allocation, run insertion, `drive_run`, and `RunAccepted`. Primary lane: TLA+ safety model.
- Gate-count mismatch: runtime canonical gate `15` versus existing storage gate `2` must fail closed until reconciled. Primary lanes: TLA+ and Verus decoded accepted-envelope predicate.
- Capability exactness: capability grants must be exact by name/action and cardinality. Primary lanes: Verus and TLA+.
- Dummy-store bypass: strict/journaled production paths must not admit through `AlwaysPresentArtifactStore` or existence-only APIs. Primary lanes: TLA+ abstraction plus later static scan and integration tests.
- Parser/codec hostile input: raw `WorkflowParts`, YAML/JSON bytes, malformed postcard, unknown schema, missing fields, and random bytes must deny without allocation and preserve diagnostics. Current executable proof lane is decoded-value Verus; byte-level fuzz remains a downstream evidence-policy row until the exact target exists or a later WAIVED/DEFERRED evidence record is approved.
- Digest mismatch: requested digest, persisted record digest, and envelope digest disagreement is a hard denial. Kani remains a downstream evidence-policy row until a harness exists or a later WAIVED/DEFERRED evidence record is approved; integration/domain scenarios remain required.
- Diagnostics: ERR-001 through ERR-008 must preserve category, rejected/requested digest where present, and semantic cause. Mutation remains a downstream evidence-policy row until diagnostic tests exist or a later WAIVED/DEFERRED evidence record is approved.
- Unsafe/UB: no unsafe/FFI/raw-pointer scope trigger was found; Miri is not applicable as a bead-specific lane.
- Concurrency: no thread/atomic/lock/channel/async interleaving scope trigger was found; Loom is not applicable.
- Dependency/supply chain: no dependency manifest or policy file is in delivery scope; cargo audit/deny/geiger is not applicable for this bead.

## Verifier Lane Plan

- TLA+ required: run existing `verification/tla/CapabilityLifecycle.tla` focused configs for no allocation on denial, gate mismatch, exact/excess capability grants, and legacy/dummy bypass.
- Verus required: run existing `verification/verus/capability_artifact_model.rs` and `verification/verus/accepted_envelope_model.rs` for decoded capability and accepted-envelope predicates.
- Runtime/tests required later: run strict admission scenarios covering ERR-001 through ERR-008 and no-allocation behavior.
- Static scan/lint required later: audit strict/journaled production paths for dummy-store and YAML/JSON runtime parse bypasses, then run `moon run :lint-src`.
- Kani/fuzz/proptest/mutation/CI: planned downstream evidence-policy rows where exact targets or state permission are absent; each policy has an expiry requiring raw evidence or downstream WAIVED/DEFERRED evidence before anyone claims that lane.
- Lean/Aeneas/Hax, TLA+ liveness, Loom, Miri, Flux, dependency audit/geiger: not applicable with explicit rationale in the obligation ledger.

## Handoff Rules

- Do not treat planned rows as pass evidence. Later execution states must run commands and record raw outputs.
- Do not create proof/model/harness/test/source/dependency/config files from State 4.
- Replace downstream evidence-policy rows only when an exact artifact and executable command exist, or when formal-verifier/landing records a WAIVED/DEFERRED evidence decision with owner, reason, expiry, limitation, and compensating evidence.
- Preserve decoded-value versus byte-level boundaries: Verus owns decoded predicates; fuzz/integration own hostile bytes; Kani remains optional only until a bounded digest harness exists.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
