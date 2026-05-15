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
