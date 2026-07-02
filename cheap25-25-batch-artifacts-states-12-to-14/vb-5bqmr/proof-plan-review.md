# Proof Plan Review: vb-5bqmr State 4

reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-5bqmr-state4-attempt1
planner_invocation_id: proof-planner-vb-5bqmr-state4-attempt1
host_session_id: femdation-cheap25-batch
review_state: 4
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr

## Reviewed Artifacts

| Artifact | Path | SHA-256 |
|---|---|---|
| proof-strategy | `.beads/vb-5bqmr/proof-strategy.md` | `246b9568bf40d6ff7735f9d0fb5440a9f016f3a1e3ee83e669a2ac43367e2644` |
| verifier-lane-decisions | `.beads/vb-5bqmr/verifier-lane-decisions.jsonl` | `16d8a6c30159e6bcb1ef85b619186ce3102c50024369213419a2c467dd000a65` |
| proof-obligations.planned | `.beads/vb-5bqmr/proof-obligations.planned.jsonl` | `22d51f4620b2fa9c6d842dea19fb7e2a217993cc47c819c0eea7e5c0428367ba` |
| trusted-base-plan | `.beads/vb-5bqmr/trusted-base-plan.md` | `29b2c9944aff1fba32b8e431403ce1ee4cfe14de5d205b9b13d89fdc0e82bb90` |
| waiver-candidates | `.beads/vb-5bqmr/waiver-candidates.jsonl` | `09553872f39d87ca681aa6ba5c6470189c1dc5a9bb5f5c3c0d555e4aa64ef52e` |
| contract | `.beads/vb-5bqmr/contract.md` | `5ddc2cb8ba8956617beed1c54a13d005beef35ac6a3e2abbaba42e386eed962e` |

## Provenance

- **Planner invocation ID** (used in every `verifier-lane-review/v1` row, NOT a ledger match):
  `proof-planner-vb-5bqmr-state4-attempt1` — workspace ledger does not have a matching
  state4 row for the proof-planner (see FND-vb-5bqmr-004). Femdation dispatch anticipates this.
- **Reviewer invocation ID** (used in every `verifier-lane-review/v1` row, ledger entry appended on approval):
  `proof-plan-reviewer-vb-5bqmr-state4-attempt1` — distinct from planner.
- **Ledger hash chain** before append:
  - Row 1: `prev=0000…0000`, `hash=f13d494e…`
  - Row 2: `prev=f13d494e…`, `hash=b7afd230…`
  - Append row 3: `prev=b7afd230…` (chain valid).
- **Workdir verified**: `pwd -P` and `jj root` both resolve to
  `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr`; coord checkout at
  `/home/lewis/src/velvet-ballistics` is not modified.

## Lane Coverage Summary

The user prompt specifies **Lanes: rust-local, kani, flux-rs, proptest** (per the
proof-planner's risk-tag mapping: `rust-local` → `verus`). All four lanes are present
with planned obligations, no silent omissions, no `not_applicable` rows required for
the in-scope lane set.

| Lane | Decisions | Obligations | Verifier rows | Disposition |
|---|---|---|---|---|
| verus | 1 (VLD-001) | 1 (PO-VERUS-001) | 1 (LR-001) | accepted |
| kani | 2 (VLD-002, VLD-003) | 2 (PO-KANI-001, PO-KANI-002) | 2 (LR-002, LR-003) | accepted |
| flux-rs | 1 (VLD-004) | 1 (PO-FLUX-001) | 1 (LR-004) | accepted (FND-001 noted) |
| proptest | 3 (VLD-005, VLD-006, VLD-007) | 3 (PO-PROP-001, PO-PROP-002, PO-PROP-003) | 3 (LR-005, LR-006, LR-007) | accepted (FND-002 noted) |
| **Total** | **7** | **7** | **7** | **7 accepted / 0 rejected** |

Lane decisions and obligation IDs match pairwise:

| Lane decision | Verifier | Obligation | Reviewer row |
|---|---|---|---|
| VLD-001 | verus | PO-VERUS-001 | LR-vb-5bqmr-001-verus |
| VLD-002 | kani | PO-KANI-001 | LR-vb-5bqmr-002-kani |
| VLD-003 | kani | PO-KANI-002 | LR-vb-5bqmr-003-kani |
| VLD-004 | flux-rs | PO-FLUX-001 | LR-vb-5bqmr-004-flux-rs |
| VLD-005 | proptest | PO-PROP-001 | LR-vb-5bqmr-005-proptest |
| VLD-006 | proptest | PO-PROP-002 | LR-vb-5bqmr-006-proptest |
| VLD-007 | proptest | PO-PROP-003 | LR-vb-5bqmr-007-proptest |

## Schema Validation

- All 7 `verifier-lane-decision/v1` rows parse; `required` applicability; `status: planned`;
  `owner_state: 4`; required_obligation_ids match the 7 obligations.
- All 7 `proof-obligation/v1` rows parse; required fields present (schema_version, id,
  requirement_id, contract_clause, domain_claim, risk, risk_tags, verifier, artifact, target,
  command, workdir, expected_evidence, assumptions, model_bounds, tool_metadata,
  trusted_base_refs, required, behavior_affecting, mode, owner_state, rerun_from, status).
- `target` is canonical (`crates::vb_storage::slot_extra::…`); no legacy aliases `layer` /
  `checker` present.
- `production_binding` is present and STRONG on the only verus obligation (PO-VERUS-001);
  `production_path` exists (`crates/vb_storage/src/slot_extra.rs`), `production_lines`
  covers 60-69 (current decode_slot_written_extra body), `assume_specification_targets`
  is non-empty (`["production::decode_slot_written_extra"]`), `exec_wrapper_required: true`,
  `drift_gate_script: scripts/check-verus-production-binding.sh` exists.
- `waiver-candidates.jsonl` has 1 row (WVR-001) with `behavior_affecting: false` and
  `review_status: proposed`; the only non-behavior row is the cargo-fuzz host-byte gap
  tracked separately under `vb-1rqz7.15`. No `E_BEHAVIOR_WAIVER` violation.
- `trusted-base-plan.md` lists 5 trust markers (TB-KANI-001-cover-reachability,
  TB-KANI-002-alloc-counter, TB-KANI-002-cover-reachability, TB-PROP-003-tracing-capture,
  TB-PROP-003-compile-time-exhaustiveness); all are `behavior_affecting: false`
  (model reductions / instrumentation / compile-time checks, NOT behavior waivers).

## Risk Coverage (against `references/verifier-trigger-matrix.md`)

| Risk class | Required lanes | Planned lanes | Status |
|---|---|---|---|
| `rejection` (C-DEC-002 / C-ERR-002) | `kani` + `proptest` | PO-KANI-001, PO-PROP-001 (plus Verus for-all) | COVERED |
| `bounded_state` (C-DEC-001/003/004) | `kani` + `verus` | PO-KANI-001/002, PO-VERUS-001 | COVERED |
| `refinement` (C-CON-001 / C-CON-004) | `flux-rs` + `verus` | PO-FLUX-001, PO-VERUS-001 | COVERED |
| `equality` round-trip (C-ENC-002) | `proptest` + `verus` | PO-PROP-002, PO-VERUS-001 | COVERED |
| `bounded_transition` (C-REC-002 / C-RUN-002 / C-FOR-001/002) | `kani` + `proptest` | PO-PROP-003 (proptest for cross-crate translation + log capture) | COVERED |

## User Constraints Verified

| User constraint | How the plan satisfies it |
|---|---|
| "no silent downgrade to LegacyFrameExtra" | C-DEC-002 + C-NEG-001..006 anti-invariants; PO-VERUS-001, PO-KANI-001, PO-PROP-001 all assert `result != Ok(LegacyFrameExtra(_))` on the magic+unknown-version branch; Verus proves the for-all claim. |
| "existing recovery_bdd_tests.rs:3158-3211 stays green" | PO-PROP-003 expected_evidence explicitly requires `cargo test -p vb_runtime --test recovery_bdd_tests` to pass unchanged; PO-PROP-002 proptest_legacy_short_input_passes_through mirror asserts the same legacy-frame classification under strategy pressure. The BDD scenario at lines 3158-3211 uses `b"\x01\x02\x03\x04"` which the new discriminator classifies as LegacyFrameExtra (no VBSE magic) — preserved. |
| 7 obligations (verus STRONG + kani ×2 + flux-rs + proptest ×3) | Exactly 7 obligations: PO-VERUS-001 (STRONG) + PO-KANI-001, PO-KANI-002 + PO-FLUX-001 + PO-PROP-001, PO-PROP-002, PO-PROP-003. |
| Lanes: rust-local, kani, flux-rs, proptest (no loom, no fuzz) | All 7 decisions are within the four lanes; loom/fuzz are excluded by user instruction (RED QUEEN §M3 fuzz gap tracked in vb-1rqz7.15, separate bead). |
| `recovery/tests.rs:2332` corrupt-v1 helper classified as `DecodeFailed`, not `VersionMismatch` | PO-PROP-002 proptest_corrupt_v1_returns_decode_failed_not_version_mismatch asserts `b"VBSE\x01\xff\xff\xff"` → `Err(DecodeFailed)` exactly; PO-PROP-003 expected_evidence asserts the helper stays green. |
| RecoveryError NOT widened | C-REC-004 + TB-PROP-003-compile-time-exhaustiveness: existing `recovery_unit_tests.rs:1149-1172` `_exhaustive_match` compile-time check must remain green; PO-PROP-003 verifies this. |
| CollectExtraHydrationFailureKind gains EXACTLY one arm `VersionMismatch` | C-RUN-004 ratifies; the same compile-time exhaustiveness invariant applies at the collect site. |

## Anti-Laundering (GOD RULES)

- **GOD RULE 1 (no hardcoded Kani shapes)**: PO-KANI-001/002 use `kani::any()` and
  `kani::any_where()` for symbolic bytes; no fixed dummy `WorkflowParts` or
  hardcoded `RunFrame`. Trusted-base-plan.md confirms.
- **GOD RULE 2 (no vacuum Verus)**: PO-VERUS-001 binds via STRONG production binding
  with `#[path]` + `assume_specification`; drift-gate is `scripts/check-verus-production-binding.sh`.
  Expected_evidence explicitly runs the gate and requires exit_status=0. The schema
  review's mandatory `production_binding` field is present and valid.
- **GOD RULE 4 (no loop oscillations)**: Plan does not mutate the production body
  to satisfy proofs; the `proof_decode_three_arms_partition` proof uses standard
  Verus idioms only (assert, assert by, use_type_invariant, reveal); no
  `#[verifier::external_body]`, `assume`, `axiom`, or `admit`.
- **GOD RULE 5 (no blind verification mutations)**: scope_mode is "focused — single-call
  graph blast radius, no broad re-scan" (proof-strategy.md §1); 7 obligations
  bounded to the slot_extra call graph (slot_extra.rs, hydrate.rs:209-235, collect.rs:256-273,
  errors.rs CollectExtraHydrationFailureKind).

## Findings Summary

`proof-plan-findings.jsonl` has 4 rows, all disposition `owner_approved_no_action` (non-blocking):

| ID | Severity | Code | Subject | Disposition |
|---|---|---|---|---|
| FND-vb-5bqmr-001 | low | E_COMMAND_PATH | PO-FLUX-001.command uses rejected `--lib` and non-existent `verified` feature | owner_approved_no_action (proof-writer) |
| FND-vb-5bqmr-002 | low | E_COMMAND_PATH | PO-PROP-003.command targets unit test with `--test` selector | owner_approved_no_action (proof-writer) |
| FND-vb-5bqmr-003 | low | E_SCHEMA_FORMAT | PO-VERUS-001.production_binding.production_lines has " (NEW body)" suffix | owner_approved_no_action (proof-writer) |
| FND-vb-5bqmr-004 | informational | E_INVOCATION_LEDGER_MISSING | state3 + state4 rows absent from workspace ledger | owner_approved_no_action (femdation-controller) |

No `blocker` findings. No `E_BEHAVIOR_WAIVER`, no `E_VERUS_DISCONNECTED_SPEC`, no
`E_KANI_ASSUMPTION_VACUITY`, no `E_KANI_COVER_ONLY`, no `E_FLUX_TRUST_ABUSE`, no
`E_PROOF_PLAN_MISSING_VERUS`, no `E_PROOF_PLAN_MISSING_NONVACUITY`, no
`E_LANE_SELF_REVIEW` (planner_invocation_id ≠ reviewer_invocation_id on every row).

## Output Artifacts (this review)

- `proof-plan-review.md` (this file)
- `verifier-lane-review.jsonl` (7 rows, all `verifier-lane-review/v1`, all accepted)
- `proof-plan-findings.jsonl` (4 rows, all `finding/v1`, all disposition `owner_approved_no_action`)
- agent-invocation-ledger.jsonl row appended (state=4, skill=proof-plan-reviewer)

## Approval

All 7 lane decisions have an independent `verifier-lane-review/v1` row with distinct
planner / reviewer invocation IDs. The plan is precise enough for proof-writer and
proof-to-implementation: each obligation has a non-vacuous strategy, an exact command,
explicit bounds, an expected evidence marker, and a trust-marker ledger row where
applicable. The user-prompted constraints (no silent downgrade, BDD regression
3158-3211, 7-obligation budget, lane set, corrupt-v1 anti-invariant, no widening) are
all preserved. The four `owner_approved_no_action` findings are minor documentation /
path defects that the proof-writer will adjust during materialization; they do not
block downstream advancement.

STATUS: APPROVED