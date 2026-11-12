# Contract Build Report

bead_id: vb-qi37.6
state: 3
workspace: /home/lewis/src/vb-qi37-6
status: REPAIRED

## Inputs rebuilt from current workspace

- `.beads/vb-qi37.6/STATE.md`
- `.beads/vb-qi37.6/baseline-report.md`
- `.beads/vb-qi37.6/codebase-map.md`
- `.beads/vb-qi37.6/delivery-scope.jsonl`
- Current code under `/home/lewis/src/vb-qi37-6` only.

## Outputs written

- `.beads/vb-qi37.6/contract.md`
- `.beads/vb-qi37.6/domain-model-review.md`
- `.beads/vb-qi37.6/tla-spec.md`
- `.beads/vb-qi37.6/lean-contract.md`
- `.beads/vb-qi37.6/verification-layers.md`
- `.beads/vb-qi37.6/proof-obligations.jsonl`
- `.beads/vb-qi37.6/traceability-matrix.jsonl`
- `.beads/vb-qi37.6/contract-build-report.md`

## Scope decisions

- Exact vs hierarchical grants: exact-only. Hierarchical and partial prefixes deny.
- Least privilege: strict cardinality exactness. Extra grants deny.
- Gate count mismatch: runtime canonical count is 15; storage count 2 fails closed until aligned.
- Required capabilities: storage persistence must preserve non-empty requirements from validated action contracts; current empty persistence is a blocker for implementation/proof states.
- Runtime API: current public submits use empty grants; State 3 contract requires a grant path or explicit denial for protected artifacts.
- Shard drive: current empty contract slice is safe-by-denial but blocks contracted Do execution; contract requires threading validated action contracts.
- Legacy admission: existence-only admission is not valid evidence for protected Strict/Journaled submits.
- UI parity: `ActionDescriptionView.required_capabilities` must project the same validated source of truth.

## Validation

- Required outputs are non-empty.
- `proof-obligations.jsonl` and `traceability-matrix.jsonl` are valid JSONL.
- Every proof-obligation row includes required user fields: `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, `waiver`.
- No proof-obligation row has status `PASS`; State 3 only plans.
- No old State 3 conversational summary was consumed as approval.

## Next gate

Independent contract-verification review must approve or reject these artifacts before proof planning/test planning/implementation consumes them.
