# Proof Plan Review Input - vb-qi37.2.5 State 4 FUZZ-RESOURCE-001 repair

STATUS: READY_FOR_INDEPENDENT_REVIEW

## Review Scope
- Review artifacts: `.beads/vb-qi37.2.5/proof-strategy.md` and `.beads/vb-qi37.2.5/proof-obligations.planned.jsonl`.
- Repaired upstream artifacts: `.beads/vb-qi37.2.5/contract.md`, `.beads/vb-qi37.2.5/verification-layers.md`, `.beads/vb-qi37.2.5/proof-obligations.jsonl`, and `.beads/vb-qi37.2.5/traceability-matrix.jsonl`.
- Blocker evidence to verify against: `.beads/vb-qi37.2.5/formal-verification-report.md`, `.beads/vb-qi37.2.5/machine-gate-report.md`, `.beads/vb-qi37.2.5/regression-diff.md`, and `.beads/vb-qi37.2.5/verification-ledger.jsonl`.

## Reviewer Checks Requested
- Confirm every JSONL row has required fields: `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, `waiver`.
- Confirm every `status` is one of `planned`, `blocked_tooling`, `waived`, or `not_applicable`.
- Confirm `PO-003` and `PO-004` use exact executable TLC commands with explicit `-metadir` paths.
- Confirm `PO-005` is `status: waived`, not a Kani pass claim, and includes owner, pending-review detail, reason, limitation, expiry, and compensating evidence.
- Confirm `PO-006` and `PO-007` use exact `cargo test --package vb_core --lib -- ...` commands instead of blocked placeholders.
- Confirm `PO-009` maps to `FUZZ-RESOURCE-001` / `INV-008` and uses stdin replay plus companion property-test evidence, not `cargo fuzz run resource_budget -- -runs=1000`, as the required executable lane.
- Confirm the `PO-009` waiver covers only the invalid cargo-fuzz command as evidence for the current stdin-once driver; it does not waive `INV-008`, hostile-input boundedness, no-panic/no-OOM/no-timeout expectations, or companion malformed-byte/property tests.
- Confirm Miri, static, and global-runtime-classification lanes are still planned and do not claim pass results; `DEFERRED_GLOBAL` may only be a later State 11 classification from raw evidence.
- Confirm no production, test, proof, model, harness, dependency, or config edits are required by this State 4 output.

## Discovery Evidence Summary
- `pwd -P`: isolated workspace confirmed as `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`.
- Isolation guard: PASS; workspace is outside `/home/lewis/src/velvet-ballistics`.
- Required repaired State 3 inputs and State 11 blocker artifacts exist and are non-empty.
- `FUZZ-RESOURCE-001` source obligation and `PO-009` planned obligation were inspected with `jq -c`.
- Blocked discovery commands: none.

## Expected Review Decision Boundary
- Approval may unlock downstream proof/test execution review with the repaired `FUZZ-RESOURCE-001` plan.
- Approval must not count as proof execution evidence.
- Kani remains waived only if the independent reviewer accepts the waiver; otherwise the plan must return to repair.
- Any global runtime missing-chunk failure remains planned for State 11 classification and is not waived by State 4.
