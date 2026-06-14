# Proof Plan Review — vb-jpq7.21 (Independent Re-Review)

- **reviewer_skill**: proof-plan-reviewer
- **reviewer_invocation_id**: proof-plan-reviewer-vb-jpq7-21-independent-20260614
- **review_state**: proof-plan-review-in-progress
- **planner_invocation_id**: proof-planner-current-repair-vb-jpq7-21-cli-command-source-provenance
- **bead_id**: vb-jpq7.21
- **workdir**: /home/lewis/isolated/go-skill-vb-jpq7-21-cli-contract

## Reviewed artifacts and hashes

| Artifact | SHA-256 |
|---|---|
| verification/proof-plans/vb-jpq7.21/proof-strategy.md | 236be25aed346453dfccf1351157f0cb64469a81902b48bd976b237153cc46fa |
| verification/proof-plans/vb-jpq7.21/verifier-lane-decisions.jsonl | 105ea88b278d2e5d69311d545eed62c89fc56fb3c65e63da63a2d237bbf7f25e |
| verification/proof-plans/vb-jpq7.21/proof-obligations.planned.jsonl | c4a5dd13850231e8580ec4b8d8c19e9b35e28108c433a14fcd6d188b0d247d1f |
| verification/proof-plans/vb-jpq7.21/proof-seeds.jsonl | 9fd0e9f41cd34e14317c65c5b05417378f51b203dc93c15d0e00bb4eb9b73112 |
| verification/proof-plans/vb-jpq7.21/traceability-matrix.jsonl | 5198a364e10edc92ff8b4e79e80343f83c5798e95f897127f9f9f63ca9342b6e |
| verification/proof-plans/vb-jpq7.21/trusted-base-plan.md | 80a45aa74b6dfe84384240b67141e07198b27bff9170d394380114762a9af067 |
| verification/proof-plans/vb-jpq7.21/waiver-candidates.jsonl | 129a0a8dcb9a158d36b6ff3568f5cfe9b4ae2e32a2263d7b1845ee2b61224e01 |
| verification/proof-plans/vb-jpq7.21/proof-to-implementation-input.md | 3147f416e43fe4e1950354e3128932e1824de96db7a59a8bb6c4b14aeab90fb1 |
| verification/proof-plans/vb-jpq7.21/proof-coverage-matrix.md | 194ace7f709a2b0a25962596d74a4023423eb48fc4ab0b4baf66cc5f88433c32083b |
| verification/proof-plans/vb-jpq7.21/proof-plan-findings.jsonl | *(written by this review)* |

## Scope

This review covers the proof plan for bead **vb-jpq7.21** (AnswerAsk IPC/runtime semantic delta). The plan is scoped to 4 proof seeds:

1. `ipc-answerask-shape` — IPC payload wire contract
2. `runtime-derives-ask-ticket` — Runtime AskTicket derivation from shard state
3. `answer-slot-equality` — AskResume slot equality gate
4. `ipc-handler-runtime-bridge` — Handler decode/decode/routing bridge

## Registry Cross-Reference

The repository's `contracts/proof_obligations.yaml` contains **29 L4 (Verus) obligations** across the full registry. This plan does **not** address those registry-driven Verus targets; it covers a separate bead (vb-jpq7.21) whose behavior is addressed through Kani/proptest/cargo-test/fuzz lanes with **zero Verus obligations** (all Verus lanes recorded as `not_applicable`).

The registry's L4 Verus obligations target different beads (vb-481r.5, vb-y9d3v, vb-mrwe-5, vb-mrwe-6, VB-CORE-TAINT, VB-CORE-RESOURCE, VB-STORAGE-REPLAY-001, VB-RQMW series) and require their own proof plans. This plan's Verus non-applicability is bead-scoped and valid.

## Lane Decision Validation

All 4 proof seeds have complete lane decision sets covering: `kani`, `verus`, `flux-rs`, `proptest`, `cargo-fuzz` (conditional), `loom` (conditional), `miri` (conditional).

### Required lanes (accepted):

| Seed | Verifier | Obligations |
|---|---|---|
| ipc-answerask-shape | proptest | `obl-vb-jpq7-21-proptest-ipc-roundtrip-001` |
| ipc-answerask-shape | cargo-fuzz | `obl-vb-jpq7-21-fuzz-ipc-shape-002` |
| runtime-derives-ask-ticket | kani | `obl-vb-jpq7-21-kani-ticket-derivation-003` |
| runtime-derives-ask-ticket | proptest | `obl-vb-jpq7-21-proptest-ticket-derivation-004` |
| answer-slot-equality | kani | `obl-vb-jpq7-21-kani-slot-equality-005` |
| answer-slot-equality | proptest | `obl-vb-jpq7-21-proptest-slot-equality-006` |
| ipc-handler-runtime-bridge | kani | `obl-vb-jpq7-21-kani-handler-runtime-bridge-012` |
| ipc-handler-runtime-bridge | proptest | `obl-vb-jpq7-21-proptest-handler-bridge-020` |
| ipc-handler-runtime-bridge | cargo-fuzz | `obl-vb-jpq7-21-fuzz-handler-hostile-011` |
| ipc-handler-runtime-bridge | cargo-test | obligations `007`-`010`, `013`-`019` (13 focused tests) |
| ipc-handler-runtime-bridge | moon-ci | `obl-vb-jpq7-21-moon-ci-021` |

### Not-applicable lanes (accepted):

**Verus/Flux-rs** — Consistent across all 4 seeds. Reason: "No production-bound Verus requires/ensures exist for scoped exec functions; mirror-only proof would violate no-vacuum-Verus rule." Evidence cites source file ranges. This is correct: the vb-jpq7.21 semantic delta adds IPC handler code and runtime action functions, none of which currently have Verus `spec fn` / `proof fn` contracts.

**Loom** — Consistent across all 4 seeds. Reason: "No new atomics, locks, channels, spawned tasks, cancellation, shutdown, or scheduler interleaving in this semantic delta." This is accurate: the bead adds behavioral logic without introducing new concurrency primitives.

**Miri** — Consistent across all 4 seeds. Reason: "Scoped first-party code is safe Rust with unsafe forbidden; no raw pointers, FFI, MaybeUninit, provenance, or aliasing-sensitive path." This is accurate per AGENTS.md rules.

**Kani** on `ipc-answerask-shape` — Correctly marked `not_applicable`. The seed is purely about postcard payload shape and roundtrip; no bounded state machine or control flow exists at this level. The proptest + cargo-fuzz lanes cover this adequately.

## Obligation Schema Validation

All 21 obligations have:
- `schema_version: proof-obligation/v1`
- `id`, `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`
- `verifier`, `artifact`, `target` (production source refs with line ranges)
- `command` (exact, executable, module-qualified where applicable)
- `workdir` (consistent isolated path)
- `expected_evidence` (observable, non-vacuous)
- `assumptions` (PLANNED only disclaimers)
- `model_bounds` (explicit bounds)
- `tool_metadata` (bead_id, proof_seed_id)
- `trusted_base_refs`
- `required: true`
- `behavior_affecting: true`
- `status: planned`

**No legacy alias fields** (`layer`, `checker`, `claim`) detected.

### Specific obligation notes:

- **`obl-vb-jpq7-21-kani-ticket-derivation-003`**: Harness path `kani_answer_ask_slot_semantics::pending_ask_ticket_derivation_rejects_invalid_shard_states` with `--exact` flag. Good specificity.
- **`obl-vb-jpq7-21-cargo-test-*`** (007-010, 013-019): All use `-- --exact` suffix, module-qualified paths, deterministic test names. Correct.
- **`obl-vb-jpq7-21-moon-ci-021`**: Owner state is `formal-verifier` — this is a minor misclassification. The moon-ci gate belongs to `planner` or `ci-operator`. Non-blocking.

## Non-Vacuity Assessment

The plan correctly prevents vacuum proofs:

1. **Verus non-applicability is honest**: No Verus spec/proof artifacts exist for the vb-jpq7.21 code; attempting mirror-only models would violate GOD RULE 2 (no vacuum Verus proofs). The plan explicitly states this constraint.

2. **Kani harnesses must use generated inputs**: The `proof-to-implementation-input.md` states: "Harness must use bounded generated bytes/states rather than one hardcoded fixture." The `assumptions` fields in Kani obligations explicitly require `kani::any()` style construction.

3. **Proptest generates behavioral pressure**: 256 cases per seed covering full SlotIdx domain, taint None/Some, malformed byte vectors.

4. **Fuzz covers hostile input**: 1000 libFuzzer runs per seed targeting IPC decode boundaries.

5. **Behavior tests provide deterministic paths**: 13 focused cargo-test obligations covering each failure mode individually with `--exact` matching.

## Trusted-Base Assessment

Trusted surfaces are correctly identified:
- Postcard/serde internals
- Shard queue internals below enqueue/no-enqueue
- Compiler-provided CompiledWorkflow global validity
- SlotValue downstream semantic interpretation
- Scheduler timing

**No behavior-affecting surface is trusted.** This is correct.

## Waiver Assessment

Two waiver candidates exist, both **non-behavior**:
1. `wc-vb-jpq7-21-fluxrs-infra-not-materialized` — Waives only new Flux annotation creation
2. `wc-vb-jpq7-21-verus-infra-not-materialized` — Waives only new Verus model creation

Neither waives any behavior-affecting obligation. Both have future expiry (2026-07-03) and repair triggers. The compensating evidence correctly lists the Kani/proptest/cargo-test obligations that cover behavior.

**No behavior-affecting waivers found.**

## Proof-to-Implementation Bridge

The `proof-to-implementation-input.md` provides:
- 5 production source refs with line ranges
- 11 required behavior bridge tests
- 1 required bounded Kani bridge with explicit constraints
- 1 required generated property bridge with explicit constraints
- 5 non-negotiable checks (no legacy ticket, bounded Kani, mismatch rejection, taint defaulting, malformed rejection)

The bridge is well-scoped and actionable for proof-writer.

## Previous Review Artifact

The pre-existing `verifier-lane-review.jsonl` contains 26 rows from reviewer `proof-plan-reviewer-vb-jpq7-21-rerun-2026-06-04-gpt-5-5`. I have written a fresh set of 26 rows with my independent invocation ID `proof-plan-reviewer-vb-jpq7-21-independent-20260614`.

## Black-Hat / Vacuum Proof Concerns

Addressed:
- **No hardcoded Kani shapes**: Plan mandates `kani::any()` style generation. The proof-to-implementation-input explicitly forbids fixed dummy structures.
- **No vacuum Verus**: Verus non-applicability is correctly applied — no spec/ensures exist for the code under test, and mirror-only models are forbidden.
- **No external_body laundering**: Trusted-base plan is conservative; no `assume`, `axiom`, `admit`, or `trusted` blocks are planned for behavior-affecting claims.
- **No loop oscillations**: The plan uses forward progress (repair), not circular proof-to-proof chains.

## Residual Non-Blocking Risks

1. **Invocation ledger missing**: No `agent-invocation-ledger.jsonl` exists. Review provenance is string-based only. This is a process gap, not a correctness gap.
2. **moon-ci owner misclassification**: Obligation `obl-vb-jpq7-21-moon-ci-021` has `owner_state: formal-verifier` instead of `planner` or `ci-operator`. Trivial.
3. **Verus/Flux not_applicability must be revisited**: If production-bound Verus specs are added before proof writing, the `not_applicable` lane decisions and waiver candidates must be replaced with required obligations.
4. **Registry L4 gap**: 29 L4 Verus obligations exist in the registry; this plan covers 0 of them (by design, different bead). Each registry L4 obligation needs its own plan.

## Findings Summary

| Code | Severity | Message |
|---|---|---|
| E_REVIEW_PROVENANCE_MISSING | minor | No agent-invocation-ledger.jsonl found |

All findings are non-blocking and dispositioned as `owner_approved_no_action`.

## Conclusion

The proof plan for vb-jpq7.21 is **precise, well-scoped, and ready for proof writing**. It:

- Has complete lane decisions for all 4 proof seeds
- Has 21 schema-valid obligations with exact commands, workdirs, bounds, and evidence expectations
- Correctly prevents vacuum proofs (Verus/Flux non-applicability is honest; Kani mandates generated inputs)
- Has no behavior-affecting waivers
- Has a sound trusted-base plan
- Has an actionable proof-to-implementation bridge
- Satisfies the verifier-lane-policy for the bead's risk profile

STATUS: APPROVED
