# Proof Plan Review Input: vb-engine-yaml

## Reviewer Task

Review State 4 attempt 3 for `vb-engine-yaml`. Approve only if the plan reflects repaired State 3, preserves State 6 rejection lessons, maps every obligation to a requirement/contract/risk, and does not claim proof results.

## Inputs Read

- `.beads/vb-engine-yaml/contract.md`
- `.beads/vb-engine-yaml/proof-obligations.jsonl`
- `.beads/vb-engine-yaml/traceability-matrix.jsonl`
- `.beads/vb-engine-yaml/delivery-scope.jsonl`
- `.beads/vb-engine-yaml/codebase-map.md`
- `.beads/vb-engine-yaml/tla-spec.md`
- `.beads/vb-engine-yaml/verification-layers.md`
- `.beads/vb-engine-yaml/lean-contract.md`
- `.beads/vb-engine-yaml/proof-review.md`
- `.beads/vb-engine-yaml/proof-findings.jsonl`
- `.beads/vb-engine-yaml/proof-repair-guide.md`
- `.beads/vb-engine-yaml/contract-verification-review.md`
- `.beads/vb-engine-yaml/proof-evidence.md`
- `.beads/vb-engine-yaml/proof-writer-report.md`

## Outputs To Review

- `.beads/vb-engine-yaml/proof-strategy.md`
- `.beads/vb-engine-yaml/proof-obligations.planned.jsonl`

## Discovery Commands

- `pwd -P`
- `test -s ".beads/vb-engine-yaml/contract.md" && test -s ".beads/vb-engine-yaml/traceability-matrix.jsonl" && test -s ".beads/vb-engine-yaml/delivery-scope.jsonl"`
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_yaml crates/vb_validate crates/vb_compile crates/vb_core crates/vb_runtime crates/vb_storage crates/vb_ipc crates/velvet_ballastics fuzz kani verification tests xtask .moon Cargo.toml Cargo.lock velvet-ballistics-MASTER.md`
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_yaml crates/vb_validate crates/vb_compile crates/vb_core crates/vb_runtime crates/vb_storage crates/vb_ipc crates/velvet_ballastics fuzz kani verification tests xtask .moon Cargo.toml Cargo.lock velvet-ballistics-MASTER.md`

## Discovery Result Summary

- Required State 3 planning inputs exist.
- Risk discovery found 12766 matches in 470 scoped files.
- Proof discovery found 1750 matches in 385 scoped files.
- Existing proof/evidence surfaces include TLA+ models, Verus files, Kani harnesses, Loom model paths, fuzz targets, proptest usage, Miri/Moon governance, and queue/retry/state surfaces.
- No discovery command was blocked.

## Required Reviewer Checks

- Check that `TLA-INGRESS-001` is preserved for PRE-006/POST-007 and is not replaced by Loom.
- Check that TLA+ expected evidence demands non-vacuous eventuality/fairness where the contract claims progress.
- Check that ingress/backpressure expected evidence is non-tautological and includes full-queue submit rejection without growth.
- Check that Loom uses the exact rejected-gate command and remains required until raw PASS evidence exists.
- Check that Kani rows avoid relying on the previous timed-out workspace run and name focused command alternatives.
- Check that fuzz/proptest/operator diagnostic rows cover hostile IPC/direct-ingress and every repaired error-taxonomy scenario through executable evidence.
- Check that Lean waiver and Flux not-applicable rows are explicit, reviewable, and not hiding applicable proof risk.
- Reject if any planned row lacks `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, or `waiver`.

## Known Blockers For Downstream States

- `LOOM-IPC-001` was previously FAIL_LOCAL because `Arc` was undeclared in two existing Loom model files; this plan does not waive it.
- Kani was previously INCOMPLETE_TIMEOUT under `rtk cargo kani --workspace`; this plan requires focused harness evidence or a completed longer CI run.
- Prior TLA+ PASS output was rejected for vacuity/tautology; this plan requires strengthened properties before proof approval.
