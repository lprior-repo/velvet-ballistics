# Proof Plan Review Input: vb-core-yaml-e2e-chain

## Status

- State: 4 proof planning repair.
- Attempt: 3-of-7.
- Trigger: State 3 repaired contract/proof obligations after State 6 rejection.
- Decision requested: review refreshed `proof-strategy.md` and `proof-obligations.planned.jsonl` for adequacy before proof/model/harness writing resumes.

## Rejection Items Addressed In Plan

- Kani `yaml_e2e_admission_matrix` remains explicit as `blocked_tooling` instead of hidden or treated as passed.
- TLA+ obligations now require deadlock/progress evidence or explicit approved waiver; safety-only TLC output with `CHECK_DEADLOCK FALSE` is insufficient.
- TLA+ temporal properties must be encoded as properties or explicitly waived with compensation.
- Verus obligations include shell-linkage waivers with owner, expiry, limitation, and compensating executable evidence.
- Miri codec lane is required for this release-critical parser/codec scope.
- Strict YAML rejection has a focused executable obligation.
- Exact error taxonomy coverage is consolidated into a mandatory evidence obligation across compile/storage/runtime/CLI/recovery tests.

## Files For Review

- `.beads/vb-core-yaml-e2e-chain/proof-strategy.md`
- `.beads/vb-core-yaml-e2e-chain/proof-obligations.planned.jsonl`
- `.beads/vb-core-yaml-e2e-chain/contract.md`
- `.beads/vb-core-yaml-e2e-chain/verification-layers.md`
- `.beads/vb-core-yaml-e2e-chain/proof-obligations.jsonl`
- `.beads/vb-core-yaml-e2e-chain/traceability-matrix.jsonl`
- Rejection context: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`

## Discovery Evidence

- Workspace check: `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- Required input check: `test -s contract.md`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl` succeeded.
- Risk scan command: `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" <scoped delivery paths>` found parser/codec, persistence, recovery state, retry, serialization, assertions, and no production unsafe-code permission in scoped files.
- Proof scan command: `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" <scoped delivery paths plus verification paths>` found existing Kani/proptest/Verus/TLA trigger surfaces and no discovered bead-specific fuzz target.
- Blocked discovery: none.

## Reviewer Focus

- Verify every row has required fields: `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, `waiver`.
- Verify all repaired State 3 obligations are represented or explicitly waived/not applicable.
- Reject any row that implies a pass result; this plan intentionally records only `planned`, `blocked_tooling`, `waived`, or `not_applicable` statuses.
- Confirm Kani, Miri, TLA deadlock/progress, strict YAML rejection, and error taxonomy gates are strong enough for State 6 retry.
