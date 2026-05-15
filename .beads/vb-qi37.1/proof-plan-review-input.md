# Proof Plan Review Input: vb-qi37.1

STATUS: READY_FOR_PROOF_PLAN_REVIEW

## Scope

- State: 4 proof planning repair, attempt 5 after State 3 schema repair.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.
- Planning artifacts refreshed only under `.beads/vb-qi37.1/`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.

## Files For Review

- `.beads/vb-qi37.1/proof-strategy.md`
- `.beads/vb-qi37.1/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.1/contract.md`
- `.beads/vb-qi37.1/tla-spec.md`
- `.beads/vb-qi37.1/verification-layers.md`
- `.beads/vb-qi37.1/proof-obligations.jsonl`
- `.beads/vb-qi37.1/traceability-matrix.jsonl`

## Review Focus

- Confirm every repaired contract clause PRE-001 through PRE-005, POST-001 through POST-007, INV-001 through INV-006, and ERR-001 through ERR-012 maps to an executable planned obligation or explicit waiver row.
- Confirm TLA+ obligations answer prior rejection: ack-state transitions, durable fact families, and deadlock/progress handling are required by plan.
- Confirm Verus obligations answer prior rejection: production/refinement linkage is required for workflow-source and compiled-IR digest mismatch checks only.
- Confirm `PRE-004` has direct planned coverage through `PO-003A` / `VERUS-PRE-004`, not only indirect digest mismatch rows.
- Confirm action ABI and policy digest mismatch rows are waived optional downstream obligations, not State 5 blockers for this bead.
- Confirm optional waiver rows use review-time `status: "planned"` with `required:false` and explicit waiver metadata, including `PO-036.limitation`.
- Confirm downstream non-proof rows are correctly owned by later states and do not claim pass results.

## Planned Obligation Summary

- `PO-001..PO-010`: TLA+ temporal recovery and typed fail-closed clauses, with `PO-003A` as the direct Verus `PRE-004` digest-input precondition obligation inserted after `PO-003`.
- `PO-011..PO-017`: Verus Rust-local/refinement/digest clauses; `PO-017` is required only for workflow-source and compiled-IR production-linked digest mismatch checks.
- `PO-018`, `PO-025`: executable cargo-test typed-error rows for journal and corrupt snapshot cases.
- `PO-019..PO-020`: required Verus typed-error rows for workflow-source and compiled-IR digest mismatch.
- `PO-021..PO-022`: waived optional downstream rows for action ABI and policy digest mismatch; promote only when production exposes those digest inputs/lookups/checks.
- `PO-023..PO-024`, `PO-026..PO-029`: Verus/TLA+ typed-error rows.
- `PO-030`: `moon ci` static/source governance row.
- `PO-031..PO-032`: downstream integration/fault-injection and property rows.
- `PO-033..PO-036`: explicit waived Kani, Flux, Loom/Miri, fuzz/theorem/dependency rows with waiver objects.

## Commands Run In State 4 Attempt 5

- `pwd -P`
- `test -s .beads/vb-qi37.1/contract.md && test -s .beads/vb-qi37.1/traceability-matrix.jsonl && test -s .beads/vb-qi37.1/delivery-scope.jsonl`
- `rtk grep -n "unsafe|unwrap\\(|expect\\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_storage/src/recovery crates/vb_storage/src/events.rs crates/vb_runtime/src/recovery.rs verification`
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_storage/src/recovery crates/vb_storage/src/events.rs crates/vb_runtime/src/recovery.rs verification`
- `jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null`

## State 3 Schema Repair Consumption

- Direct `PRE-004` contract obligation exists as `VERUS-PRE-004`; direct planned row exists as `PO-003A`.
- `PO-021`, `PO-022`, `PO-033`, `PO-034`, `PO-035`, and `PO-036` remain optional waiver rows, but their review-time status is `planned`.
- `PO-036` includes an explicit limitation for omitted fuzz, theorem-kernel, and dependency-specific verification lanes.

## Reviewer Warning

Prior State 5 verifier passes are invalidated as approval evidence where repaired contract/proof obligations changed. This plan intentionally treats those outputs as context only and requires State 5 proof repair/rerun for the non-vacuous typed-error proof and revised digest scope.
