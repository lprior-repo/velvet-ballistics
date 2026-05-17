<<<<<<< HEAD
# Proof Plan Review Input: vb-qi37.4.2

## Review Request

Review the State 4 proof plan for `vb-qi37.4.2`. This is a proof-planning deliverable. Reject the plan if it asks State 4 to write production code, tests, proof code, harnesses, models, specs, dependencies, or CI config.

## Skill and Scope Basis

- Proof-planner skill requires planning-only proof strategy, traceability, explicit waiver rows, and JSONL schema.
- Proof-planner prohibits writing proof code, tests, production code, harnesses, models, specs, dependencies, or CI config.
- Scoped discovery and blocked-discovery recording required.
- No invented pass results; exact paths, commands, assumptions, model bounds, and skipped-lane waiver rows required.

## Discovery Summary For Reviewer

- Workdir: `/home/lewis/src/vb-femdation/vb-qi37-4-2`.
- Required planning inputs were non-empty: `STATE.md`, `contract.md`, `lean-contract.md`, `verification-layers.md`, `tla-spec.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`.
- Contract defines: 15 preconditions (PRE-001 to PRE-015), 10 postconditions (POST-001 to POST-010), 15 invariants (INV-001 to INV-015), 88 error variants.
- 55 proof obligations spanning Verus L4 (13), Kani L3 (20), TLA+ L3 (11), Loom L3 (1), proptest L1 (6), fuzz L2 (3), static-scan L0 (3), gauntlet (2).
- Theorem kernel: none required (WAIVER-LEAN-01, WAIVER-LEAN-02).
- No discovery command was blocked.

## Required Review Questions

- Does `proof-obligations.planned.jsonl` include all 55 IDs from current `proof-obligations.jsonl`?
- Does every planned row include the required schema fields?
- Are layer assignments (Verus L4, Kani L3, TLA+ L3, Loom L3, proptest L1, fuzz L2, static-scan L0) correct per verification-layers.md?
- Does the plan preserve all waiver rows with correct owner_state, rerun_from, and compensating_evidence?
- Does the plan handle VB-CORE-TAINT-006 (taint join in EvalExpr) correctly with Kani L3 compensating evidence per WAIVER-VRF-01?
- Does the plan preserve SRC-LINT-001 and SRC-LINT-002 as static-scan L0 obligations?
- Does VB-CONC-LOOM correctly use loom L3 for shard concurrency race-free verification?
- Are TLA+ obligations using correct commands: `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla` etc.?
- Do GATE-001 and GATE-002 correctly reference `moon run :verify-proof` and `moon run :verify-all` with owner_state 12?
- Are there any blocked obligations that should be planned instead?

## Planned Verifier Lanes Summary

| Lane | Count | Tool | Owner State |
|------|-------|------|-------------|
| Verus L4 | 13 | verus | 3 |
| Kani L3 | 20 | cargo kani | 3 |
| TLA+ L3 | 11 | tlc | 3 |
| Loom L3 | 1 | cargo loom | 6 |
| Proptest L1 | 6 | cargo nextest | 5 |
| Fuzz L2 | 3 | cargo fuzz | 6 |
| Static-scan L0 | 3 | cargo clippy/xtask | 0 |
| Gauntlet | 2 | moon | 12 |

## Planned Waiver Rows

- `WAIVE-VRF-01`: VB-CORE-TAINT-006 Verus proof deferred; Kani L3 + proptest L1 compensating evidence.
- `WAIVE-VRF-02`: VB-CORE-IDX-002 is static-scan L0 obligation, not formal verifier.
- `WAIVE-VRF-03`: Supply chain (cargo audit, cargo deny, cargo vet) is L0/L6 non-goal.
- `WAIVE-TLA-01` through `WAIVE-TLA-05`: Taint lattice, StepBudget, FiniteF64, IPC/Record decode, numeric ID safety have no temporal/state-over-time behavior; appropriate lower-layer tools sufficient.

## Known Scope Boundaries

- Generated codegen parity: non-goal for vb-qi37.4.2; covered by differential testing.
- UI rendering (makepad): non-goal; covered by integration tests.
- Fjall compaction internals: non-goal; covered by Fjall test suite.
- External systems (OS scheduler, network transport, hardware memory model): excluded from formal proof.

## Downstream Dependencies

- Verus L4 proofs require vb_core verification/verus/ directory structure.
- Kani L3 harnesses require vb_core, vb_ipc, vb_storage, vb_expr verification/kani/ directory structure.
- TLA+ L3 models require verification/tla/ directory with LifecycleJournal.tla, ConcurrencyControl.tla, RetryFSM.tla, CapabilityLifecycle.tla.
- GATE-001 and GATE-002 require moon v2 tasks :verify-proof and :verify-all configured.
# State 4 Review Input Addendum: vb-qi37.4.2

STATUS: UPDATED

Contract/traceability repair rerun from State 3 changed the proof plan surface:

- Added `VB-CORE-RUNFRAME-001` for `PRE-001`.
- Added `VB-CORE-RUNFRAME-002` for `POST-001`.
- Added `VB-CORE-RUNFRAME-003` for `INV-007`.
- Added `VB-CORE-IDEMPOTENCY-001` for `INV-014`.
- Reconciled `POST-010` and `VB-CORE-RESOURCE-001/003` to saturating, policy-bounded semantics instead of unbounded no-overflow semantics.
- Required State 5 proof repair to add TLC `PROPERTY`/deadlock evidence for liveness rows and to execute/waive every required Kani/proptest/fuzz/Loom/static/gauntlet row.

JSONL gate evidence: `jq -c .` succeeded for `delivery-scope.jsonl`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, and `traceability-matrix.jsonl`; row counts are 59 obligations and 40 traceability rows.
=======
# Proof Plan Review Input

Bead: `vb-qi37.4.2`
Reviewer target: State 4 proof-plan review after repaired State 3 contract status repair
Planner output: `.beads/vb-qi37.4.2/proof-obligations.planned.jsonl`

## Inputs Read

- `.beads/vb-qi37.4.2/STATE.md`
- `.beads/vb-qi37.4.2/contract.md`
- `.beads/vb-qi37.4.2/delivery-scope.jsonl`
- `.beads/vb-qi37.4.2/codebase-map.md`
- `.beads/vb-qi37.4.2/domain-model-review.md`
- `.beads/vb-qi37.4.2/tla-spec.md`
- `.beads/vb-qi37.4.2/lean-contract.md`
- `.beads/vb-qi37.4.2/verification-layers.md`
- `.beads/vb-qi37.4.2/proof-obligations.jsonl`
- `.beads/vb-qi37.4.2/traceability-matrix.jsonl`
- `.beads/vb-qi37.4.2/martin-fowler-tests.md`
- `.beads/vb-qi37.4.2/proof-review.md`
- `.beads/vb-qi37.4.2/proof-findings.jsonl`
- `.beads/vb-qi37.4.2/proof-repair-guide.md`
- `.beads/vb-qi37.4.2/contract-verification-review.md`
- `.beads/vb-qi37.4.2/proof-evidence.md` as prior context only

## Discovery Commands Run

- `pwd -P` -> exit 0; output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`.
- `test -s ".beads/vb-qi37.4.2/contract.md" && test -s ".beads/vb-qi37.4.2/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.4.2/delivery-scope.jsonl"` -> exit 0.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" <scoped delivery files plus existing proof assets>` -> exit 0; found 291 matches in 10 files.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" <scoped delivery files plus existing proof assets>` -> exit 0; found 59 matches in 9 files, including existing Verus proof functions.
- Blocked discovery commands: none.

## Repaired Review Questions

- Do `PO-001` through `PO-006` now have exact executable TLA+/Verus commands and existing artifacts?
- Are `PO-007`, `PO-008`, `PO-009`, `PO-011`, and `PO-012` correctly encoded as `status:"planned"` downstream evidence-policy rows rather than contract-time waiver approvals?
- Does `TEST-STRICT-009` enumerate all ERR-001 through ERR-008 diagnostic cases clearly enough for State 8 test planning/writing?
- Is `PO-010` sufficiently executable for later static scan and `moon run :lint-src` evidence?
- Are non-triggered lanes correctly represented as `not_applicable` or waiver rows rather than omitted?

## Known Constraints

- State 4 wrote only planning artifacts under `.beads/vb-qi37.4.2/`.
- No production code, test code, proof/model/harness/spec code, dependency files, or CI config were edited.
- No source checkout writes were performed.
- Planner rows do not claim pass results; they define expected evidence only.

## Acceptance Criteria For This Plan

- `proof-obligations.planned.jsonl` parses with `jq -c .`.
- Every row has required schema fields: `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, and `waiver`.
- Statuses are limited to `planned` and `not_applicable`; no State4 planned row may use `status:"waived"` after the State3 contract status repair.
- Downstream evidence-policy rows have `waiver_policy` with owner, reason, expiry, limitation, and compensating evidence, while remaining `status:"planned"`.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
