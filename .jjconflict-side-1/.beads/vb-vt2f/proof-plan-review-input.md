# proof-plan-review-input.md - vb-vt2f State 4 proof planning sync

## Review Request

Review this State 4 proof plan for bead `vb-vt2f` under sublane `proof-plan-sync-after-owner-kani-contract-alignment`. The primary 40-row ledger was accepted at State 3. This State 4 plan syncs the planned JSONL to the primary ledger and updates review guidance to reflect the owner-authorized Kani projection-kernel contract terms. This review input does not claim any proof pass.

## Artifacts Under Review

- `.beads/vb-vt2f/proof-strategy.md` (this artifact)
- `.beads/vb-vt2f/proof-obligations.jsonl` (40 rows, primary ledger)
- `.beads/vb-vt2f/proof-obligations.planned.jsonl` (40 rows, synced planned ledger)
- `.beads/vb-vt2f/traceability-matrix.jsonl` (32 rows)
- `.beads/vb-vt2f/contract.md`
- `.beads/vb-vt2f/proof-architecture-report.md`
- `.beads/vb-vt2f/contract-verification-review.md`

## Ledger Shape Validation

- Primary obligation rows: 40.
- Planned obligation rows: 40.
- Primary and planned JSONL row sets are identical (synced output of this state).
- Traceability rows: 32.
- Formal required rows present in both ledgers:
  - `TLA-VT2F-LIFECYCLE-001` (owner_state 5, exact-command)
  - `TLA-VT2F-STRICT-ADMISSION-001` (owner_state 5, exact-command)
  - `KANI-VT2F-RUNTIME-FACADE-001` (owner_state 5, exact-command)
  - `KANI-VT2F-SHARD-LOWER-001` (owner_state 5, exact-command)
  - `PROJ-EQ-VT2F-001` (owner_state 6, review-artifact)
  - `WAIVER-VERUS-VT2F-002` (owner_state 6, candidate_only, review-artifact)
- Superseded waiver rows present for audit only: `WAIVER-TLA-VT2F-001`, `WAIVER-TLA-VT2F-002`, `WAIVER-VERUS-VT2F-001`. These must not be counted as approval evidence.

## Owner-Authorized Kani Projection Kernel Contract Terms

The contract (lines 157-160) explicitly authorizes owner-authorized projection proof kernels as the required Kani proof target for `KANI-VT2F-RUNTIME-FACADE-001` and `KANI-VT2F-SHARD-LOWER-001`. Key terms:

- The proof targets are `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs::vt2f_runtime_facade_semantics` and `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs::vt2f_shard_lower_semantics` under `#[cfg(kani)]`.
- Trusted projection types: `KernelRuntimeError`, `KernelInspectResponse`, `FacadeKernelState`, `ShardKernelState`, `StoreMode`, `TicketShape`, `AskKernelFrame`. These are manual projections of concrete Runtime/shard/admission/ask behavior.
- These Kani PASS results prove only the projected bead-local semantics and must not be cited as concrete-runtime Kani equivalence, full store/Fjall behavior, scheduler fairness, or public API execution proof.
- `PROJ-EQ-VT2F-001` is the compensating manual review obligation mapping each projection type/action to concrete runtime/shard/admission/ask code; it records residual risk, owner authorization, expiry, and non-reuse caveat.
- Verus waiver `WAIVER-VERUS-VT2F-002` remains candidate-only until State 6 reviewer approval after accepted TLA PASS, owner-authorized Kani projection-kernel PASS, BDD/catalog/CI evidence, and explicit acceptance or rejection of the `PROJ-EQ-VT2F-001` trusted projection risk.

## Stale Downstream Invalidation Note (State 4 review input)

**Artifact**: `.beads/vb-vt2f/contract-verification-review.md` (REJECTED)

The `contract-verification-review.md` (State 6) was REJECTED with findings about Kani obligation text misalignment and projection-equivalence risk. However, the contract itself (lines 157-160) explicitly resolves the first finding by authorizing projection kernels as the required proof target. The second finding (projection-equivalence not discharged) is addressed by requiring `PROJ-EQ-VT2F-001` as a mandatory compensating obligation before `WAIVER-VERUS-VT2F-002` can be approved.

**Additionally**: The following State 6 reviewer findings identify stale downstream boundary documents that should be repaired by the responsible owner before being cited as current:

1. `.beads/vb-vt2f/tla-spec.md` still says TLA+ is waived/non-applicable and no current TLA+ command is required, while the current `verification-layers.md` and `proof-obligations.jsonl` supersede those waivers and require TLA lifecycle/strict-admission models. **This document is stale and must not be cited as the current TLA boundary.**
2. `.beads/vb-vt2f/lean-contract.md` says Verus/TLA are waived unless implementation changes, which is stale against the current compensating proof architecture. **This document is stale for the current proof architecture.**
3. The `proof-review.md` and `blocker-report.md` may contain reviewer language that pre-dates the owner-authorized projection kernel contract terms; State 6 review must re-evaluate against the current contract language.

State 4 does not edit production code, tests, or State 5+ artifacts. This stale invalidation note is recorded here so downstream reviewers do not rely on boundary documents that contradict the current contract terms.

## Reviewer Questions

1. Does the planned JSONL correctly mirror the 40-row primary ledger?
2. Are the TLA rows preserved with exact commands and bounded model expectations (no unbounded Nat)?
3. Are the Kani projection-kernel rows preserved with exact commands, owner_state 5, and explicit trusted-boundary/limitations/expiry language matching contract lines 157-160?
4. Is `PROJ-EQ-VT2F-001` preserved as a required manual review obligation with owner_state 6 and full projection-equivalence mapping?
5. Does `WAIVER-VERUS-VT2F-002` remain candidate_only with the full compensating evidence gate intact?
6. Are superseded waiver rows retained for audit only and not counted as approval evidence?
7. Does the stale downstream invalidation note correctly flag `tla-spec.md` and `lean-contract.md` as pre-date contract-amendment boundary documents?
8. Are BDD/catalog/public-surface/static/CI rows still present and valid for later states?

## Rejection Criteria

Reject the plan if any of these are true:

- Planned JSONL row set is not byte-identical to the 40-row primary ledger.
- A Kani row is absent, or its command does not target the owner-authorized projection kernel harness.
- `PROJ-EQ-VT2F-001` is absent or does not require manual projection-equivalence mapping.
- `WAIVER-VERUS-VT2F-002` is treated as anything other than candidate_only before State 6 approval.
- Superseded waiver rows are used as approval paths.
- The stale downstream invalidation note is absent or misidentifies the stale boundary documents.

## Expected State 5 Inputs

- `.beads/vb-vt2f/proof-strategy.md`
- `.beads/vb-vt2f/proof-obligations.jsonl`
- `.beads/vb-vt2f/proof-obligations.planned.jsonl`
- `verification/tla/Vt2fRuntimeLifecycle.tla`
- `verification/tla/Vt2fRuntimeLifecycle.cfg`
- `verification/tla/Vt2fStrictAdmission.tla`
- `verification/tla/Vt2fStrictAdmission.cfg`
- `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs`
- `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs`

## State 4 Evidence

- JSONL parse validity: PASS (`jq -c . .beads/vb-vt2f/proof-obligations.jsonl >/dev/null` and same for planned).
- Row counts: primary 40, planned 40, traceability 32.
- Key obligation IDs confirmed present: `TLA-VT2F-LIFECYCLE-001`, `TLA-VT2F-STRICT-ADMISSION-001`, `KANI-VT2F-RUNTIME-FACADE-001`, `KANI-VT2F-SHARD-LOWER-001`, `PROJ-EQ-VT2F-001`, `WAIVER-VERUS-VT2F-002`.
- No proof execution performed in this state.
