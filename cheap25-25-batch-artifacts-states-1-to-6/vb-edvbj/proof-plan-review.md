# Proof Plan Review — vb-edvbj

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
**STRONG-coupled with:** vb-cib14 (must land together)
**Reviewer skill:** proof-plan-reviewer
**Review state:** proof-plan-review
**Review state number:** State 4b
**Date:** 2026-07-01
**Reviewer invocation ID:** proof-plan-reviewer-vb-edvbj-state4b-p4b

---

## 1. Provenance

| Field | Value |
|-------|-------|
| `reviewer_skill` | proof-plan-reviewer |
| `reviewer_invocation_id` | proof-plan-reviewer-vb-edvbj-state4b-p4b |
| `review_state` | proof-plan-review |
| `planner_invocation_id` (synthesized) | proof-planner-vb-edvbj-state4-jj:psylkkztqxkxllllzukuzqnpstsnnsxn |
| Independent review | Yes — reviewer invocation ID differs from synthesized planner invocation ID; planner artifacts do not self-stamp `reviewer_disposition`; all 14 `verifier-lane-review/v1` rows carry `planner_invocation_id ≠ reviewer_invocation_id` |

Note: `agent-invocation-ledger.jsonl` contains only State 1 (go-skill) and State 2 (explore) entries. State 3 (rust-contract) and State 4 (proof-planner) invocation rows are absent. This is a process gap logged as `E_REVIEW_PROVENANCE_INCOMPLETE` (low, `owner_approved_no_action`) — it does not block State 4b approval because the State 4 plan artifacts are present, schema-valid, and reviewable on their own merits. Future states should extend the ledger.

## 2. Reviewed Artifacts

| Artifact | Path | Status |
|----------|------|--------|
| Proof strategy | `.beads/vb-edvbj/proof-strategy.md` | reviewed, accepted |
| Verifier lane decisions | `.beads/vb-edvbj/verifier-lane-decisions.jsonl` | reviewed, accepted (14 rows) |
| Proof obligations planned | `.beads/vb-edvbj/proof-obligations.planned.jsonl` | reviewed, accepted (10 rows) |
| Trusted base plan | `.beads/vb-edvbj/trusted-base-plan.md` | reviewed, accepted (10 TB-* entries) |
| Waiver candidates | `.beads/vb-edvbj/waiver-candidates.jsonl` | reviewed, accepted (1 row, `behavior_affecting: false`) |
| Proof seeds (upstream) | `.beads/vb-edvbj/proof-seeds.jsonl` | reviewed, 8 seeds |
| Contract (upstream) | `.beads/vb-edvbj/contract.md` | reviewed, invariants I-1..I-14 + non-goals |
| Verifier lane matrix (narrative) | `.beads/vb-edvbj/verifier-lane-matrix.md` | reviewed, narrative drift logged (low) |
| Proof coverage matrix (narrative) | `.beads/vb-edvbj/proof-coverage-matrix.md` | reviewed, narrative drift logged (low) |
| Agent invocation ledger | `.beads/vb-edvbj/agent-invocation-ledger.jsonl` | reviewed, provenance gap logged (low) |
| Verifier production-binding gate script | `scripts/check-verus-production-binding.sh` | exists |
| Verus mirror | `verification/verus/extern_storage_kind_family.rs` | exists, lines 670-695 contain `prod_methods_drift_check_mirror` |
| Mirror production stub | `verification/verus/production_inner/storage_kind_family_production.rs` | exists |
| Production target — chunk_002.rs | `crates/vb_runtime/src/journal/chunk_002.rs` | exists; pre-fix body at lines 270-303 (storage_event) and 342-346 (append_sequenced) confirmed |
| Production target — diagnostics.rs | `crates/vb_runtime/src/error/diagnostics.rs` | exists; lines 165-198 contain `symbolic_code`, `registered_symbolic_code`, `legacy_unregistered_symbolic_code` |
| Production target — events.rs | `crates/vb_storage/src/events.rs` | exists (mirror anchor) |

## 3. Schema Validation

Every `proof-obligation/v1` row passes schema validation:

- `schema_version = proof-obligation/v1` for all 10 rows
- `id`, `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`, `verifier`, `artifact`, `target`, `command`, `workdir`, `expected_evidence`, `assumptions`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `required`, `behavior_affecting`, `mode`, `owner_state`, `rerun_from`, `status` all present
- No legacy alias fields (`layer`, `checker`) — `target` is the canonical field used
- `command` and `workdir` are present and concrete; `workdir` resolves to the isolated workspace `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj`
- `expected_evidence` is non-empty and verifier-specific for every row

Every `verifier-lane-decision/v1` row passes schema validation:

- `schema_version = verifier-lane-decision/v1` for all 14 rows
- `id`, `requirement_id`, `contract_clause`, `proof_seed_id`, `verifier`, `applicability`, `decision_reason`, `required_obligation_ids`, `non_applicability_evidence_refs`, `limitation_kind`, `owner_state`, `status` all present
- `required` rows name concrete obligation IDs that exist in `proof-obligations.planned.jsonl`
- `not_applicable` rows each carry 3 concrete `non_applicability_evidence_refs` and a typed `limitation_kind` (`surface_absent` or `superseded_by_other_lane_with_evidence`)

## 4. Production Binding (MANDATORY Verus gate)

Every Verus obligation has a valid `production_binding` (no `EXPLICITLY_ALLOWED` / no allowlist / no escape mechanism used):

| Obligation | Mechanism | production_path | production_lines | Required fields | Verdict |
|------------|-----------|-----------------|------------------|-----------------|---------|
| PO-EDVBJ-001-VERUS | STRONG | crates/vb_runtime/src/journal/chunk_002.rs | 270-303 | `assume_specification_targets` (5 items) ✓, `exec_wrapper_required: true` ✓ | valid |
| PO-EDVBJ-005-VERUS | STRONG | crates/vb_runtime/src/journal/chunk_002.rs | 342-346 | `assume_specification_targets` (4 items) ✓, `exec_wrapper_required: true` ✓ | valid |
| PO-EDVBJ-007-VERUS | WEAK_MIRROR | crates/vb_storage/src/events.rs | 23-350 | `mirror_path` exists (verification/verus/extern_storage_kind_family.rs) ✓, `drift_gate_script` exists (scripts/check-verus-production-binding.sh) ✓, `drift_threshold: zero` ✓ | valid |
| PO-EDVBJ-009-VERUS | STRONG | crates/vb_runtime/src/error/diagnostics.rs | 165-198 | `assume_specification_targets` (3 items) ✓, `exec_wrapper_required: true` ✓ | valid |

All 4 Verus obligations are production-bound. No Verus row is missing `production_binding`. No `EXPLICITLY_ALLOWED` or `ALLOWED_EXCEPTIONS` or `OFFLOAD` mechanism is invoked.

Production path existence verified for all four: chunk_002.rs (15K), events.rs (20K), diagnostics.rs (210 lines total, lines 165-198 contain symbolic_code/registered_symbolic_code/legacy_unregistered_symbolic_code), extern_storage_kind_family.rs (26K), check-verus-production-binding.sh (exists).

## 5. Lane Profile Compliance (Default Rust Behavior Profile)

Default required lanes per `references/verification-lane-policy.md`: `kani`, `verus`, `flux-rs`, `proptest`.

| Lane | Status | Lane decisions | Obligations |
|------|--------|----------------|-------------|
| `verus` | covered (required) | VLD-EDVBJ-001, 005, 007, 009 (4 rows) | PO-EDVBJ-001, 005, 007, 009 (4 obligations) |
| `kani` | covered (required, split-harness) | VLD-EDVBJ-002, 006 (2 rows) | PO-EDVBJ-002 (6 harnesses: kani_run_layer_no_fabricate, kani_action_layer_no_fabricate, kani_boundary_layer_no_fabricate, kani_dispatch_no_fabricate, kani_layer_consistency, kani_event_kind_enumeration), PO-EDVBJ-006 (2 harnesses: kani_append_sequenced_propagation, kani_queued_strict_gate) |
| `flux-rs` | covered (required on diagnostic_code; n/a on storage_event) | VLD-EDVBJ-008 (required), 014 (not_applicable) | PO-EDVBJ-008 |
| `proptest` | covered (required) | VLD-EDVBJ-003, 004, 010 (3 rows) | PO-EDVBJ-003, 004, 010 |
| `loom` | n/a (sync codepath, no concurrency surface) | VLD-EDVBJ-011 | — |
| `miri` | n/a (`forbid(unsafe_code)` in vb_runtime) | VLD-EDVBJ-012 | — |
| `cargo-fuzz` | n/a (no parser/codec/hostile-input boundary on dispatcher) | VLD-EDVBJ-013 | — |

All required lanes are present. The split-harness Kani requirement is satisfied by PO-EDVBJ-002-KANI (6 harnesses). The temporal-replay proptest is satisfied by PO-EDVBJ-004-PROPTEST (Fjall-backed StorageRuntimeJournal, replay-equivalence). The Verus production-binding gate (mandatory) is PO-EDVBJ-007-VERUS (WEAK_MIRROR) plus the post-implementation gate re-run of `bash scripts/check-verus-production-binding.sh`.

## 6. Required Lane Acceptance (14 of 14)

| Lane decision | Verifier | Applicability | Disposition | Justification |
|---------------|----------|---------------|-------------|---------------|
| VLD-EDVBJ-001-VERUS | verus | required | accepted | PO-EDVBJ-001-VERUS is STRONG-bound, model-bound set empty (post-fix match is exhaustively enumerated), command and expected_evidence are concrete |
| VLD-EDVBJ-002-KANI | kani | required | accepted | PO-EDVBJ-002-KANI uses `kani::any()` over an Arbitrary impl of RuntimeJournalEvent (not hardcoded), 6 harnesses (split-harness per GOD RULE 1), `kani::cover!` for reachability, `kani::assert!` for post-fix contract |
| VLD-EDVBJ-003-PROPTEST | proptest | required | accepted | PO-EDVBJ-003-PROPTEST uses `proptest::sample::select` over the 21 declared variants (exhaustive enumeration, not proptest::any for the runtime event), 10000 cases |
| VLD-EDVBJ-004-PROPTEST | proptest | required | accepted | PO-EDVBJ-004-PROPTEST (temporal-replay) constructs a Fjall-backed StorageRuntimeJournal per case and asserts (a) Err(UNMAPPED), (b) zero records in events_for_run, (c) zero RunFailedEvent records; prop_assume!(event is Resumed) is the non-vacuity guard |
| VLD-EDVBJ-005-VERUS | verus | required | accepted | PO-EDVBJ-005-VERUS is STRONG-bound at chunk_002.rs:342-346 (append_sequenced), proves Err(UNMAPPED) propagates via `?` through three sites; companion exec fn for the Strict-profile guard covers I-4 |
| VLD-EDVBJ-006-KANI | kani | required | accepted | PO-EDVBJ-006-KANI exercises the `?` chain with symbolic RuntimeJournalEvent; companion kani_queued_strict_gate covers I-4 |
| VLD-EDVBJ-007-VERUS | verus | required | accepted | PO-EDVBJ-007-VERUS is the WEAK_MIRROR binding; the existing `verification/verus/extern_storage_kind_family.rs:670-695` `prod_methods_drift_check_mirror` is unchanged by the fix and the gate re-run is mandatory |
| VLD-EDVBJ-008-FLUX | flux-rs | required | accepted | PO-EDVBJ-008-FLUX refines `diagnostic_code()` to a finite enum; H-2 collision guard is the paired negative target `diagnostic_code_unmapped_returns_0x2020_negative` |
| VLD-EDVBJ-009-VERUS | verus | required | accepted | PO-EDVBJ-009-VERUS is STRONG-bound at diagnostics.rs:165-198 (symbolic_code, registered_symbolic_code, legacy_unregistered_symbolic_code), companion to Flux |
| VLD-EDVBJ-010-PROPTEST | proptest | required | accepted | PO-EDVBJ-010-PROPTEST fuzzes the 0x2020 constant against the 33 currently-defined DiagnosticCode constants and asserts zero collision; 10000 cases |
| VLD-EDVBJ-011-LOOM | loom | not_applicable | accepted | 3 evidence refs (boundary-map.md §"Async / sync cross-check", hazard-analysis.md H-7, workflow-model.md §"Cancellation path"); limitation_kind: surface_absent |
| VLD-EDVBJ-012-MIRI | miri | not_applicable | accepted | 3 evidence refs (boundary-map.md §"Unsafe", AGENTS.md §"Engineering Rules", codebase-map.md); limitation_kind: surface_absent |
| VLD-EDVBJ-013-CARGO_FUZZ | cargo-fuzz | not_applicable | accepted | 3 evidence refs (boundary-map.md §"Parser / codec", codebase-map.md, references/verifier-trigger-matrix.md); limitation_kind: surface_absent |
| VLD-EDVBJ-014-FLUX-ON-DISPATCH | flux-rs | not_applicable | accepted | 3 evidence refs (verifier-trigger-matrix.md Flux row, cross-lane-coverage-matrix.md Pure Invariant archetype, VLD-EDVBJ-008-FLUX as superseding lane); limitation_kind: superseded_by_other_lane_with_evidence |

All 14 `verifier-lane-review/v1` rows are written in `.beads/vb-edvbj/verifier-lane-review.jsonl` with `reviewer_disposition: accepted`.

## 7. Waiver Validation

- `waiver-candidates.jsonl` contains 1 row: `WVR-EDVBJ-001-NONE` with `behavior_affecting: false`, `review_status: approved`, owner `proof-planner`, expiry `2099-12-31T23:59:59Z`. This is an "absence of waiver" attestation, not a behavior-affecting waiver. The validator gate `E_BEHAVIOR_WAIVER` is not triggered.
- The H-3 mirror-drift mitigation is a mandatory gate re-run (not a waiver): failure of `bash scripts/check-verus-production-binding.sh` blocks the merge.
- The H-2 diagnostic-code 0x201F duplicate is a pre-existing deferred finding (logged as `E_TRUSTED_BASE_DEFERRED_FINDING` in `proof-plan-findings.jsonl`); resolution path is documented in `trusted-base-plan.md` §5 (State 12 surface as separate finding OR open a separate bead).

## 8. Non-Vacuity Validation

- **Kani**: `kani::any()` is used over an Arbitrary impl of `RuntimeJournalEvent` (NOT hardcoded dummy data) per GOD RULE 1. `kani::cover!` is used for reachability (RunFailed is reachable, unmapped variants are reachable); `kani::assert!` enforces the post-fix contract.
- **Verus**: every `exec fn` uses `requires(true)` and concrete `ensures` clauses that distinguish the OK-arm from the Err-arm. No `external_body`, no `axiom`, no broad `requires` that encodes the desired result. Validator gate `E_VERUS_DISCONNECTED_SPEC` is not triggered.
- **Flux**: refines `diagnostic_code()` return type to a finite enum (not a refinement on the dispatcher body). Paired negative target guards against the pre-existing 0x201F duplicate.
- **proptest**: `proptest::sample::select` is used for exhaustive enumeration of the 21 declared variants (not `proptest::any` for the runtime event); the temporal-replay harness uses `prop_assume!(event is Resumed)` so the strategy is non-vacuous; the diagnostic-code harness uses `prop_assume!(matches!(variant, UnmappedRuntimeJournalEvent))`.
- No Kani `cover!` is used as the sole satisfaction evidence for any obligation.

## 9. Bridge Plan Validation

The plan bridges proof claims to implementation in two directions:

1. **proof-strategy.md §3** maps each proof seed to specific harness/artifacts (e.g., `kani_run_layer_no_fabricate` → `harnesses/kani/vb_edvbj_storage_event_no_fabricate.rs`).
2. **proof-strategy.md §11 / proof-coverage-matrix.md §8** documents the STRONG coupling with `vb-cib14` and the failure mode (rename of `JournalEvent::RunResumed` → mandatory re-plan).
3. **proof-to-implementation** (State 7) is the next downstream step; the plan provides concrete `target` field on every obligation pointing to the production file and the function/method.

## 10. Findings

| ID | Severity | Code | Disposition | Status |
|----|----------|------|-------------|--------|
| F-EDVBJ-001 | low | E_SCHEMA_DRIFT_NARRATIVE | owner_approved_no_action | acknowledged |
| F-EDVBJ-002 | low | E_REVIEW_PROVENANCE_INCOMPLETE | owner_approved_no_action | acknowledged |
| F-EDVBJ-003 | medium | E_TRUSTED_BASE_DEFERRED_FINDING | owner_approved_debt | tracked for State 12 |

No `blocker` findings. No `fixed_with_evidence` findings (the plan as authored is the reviewed artifact; this review does not re-author the plan). The plan is approval-ready.

## 11. Coupling with vb-cib14 (STRONG)

The plan is STRONG-coupled with `vb-cib14`. The proof obligations here assume vb-cib14's implementation lands (or is landing in the same JJ change). Specifically:

- PO-EDVBJ-001-VERUS's `exec_storage_event` mirrors the post-fix body, which requires vb-cib14's signature changes (if any) to `RuntimeError`.
- PO-EDVBJ-003-PROPTEST's exhaustive 21-variant strategy assumes vb-cib14's `UnmappedRuntimeJournalEvent` variant is declared in `crates/vb_runtime/src/error/mod.rs`.
- PO-EDVBJ-007-VERUS's mirror gate assumes vb-cib14 has not introduced any rename of `JournalEvent::RunResumed` (which would break the existing `prod_methods_drift_check_mirror`).

The proof-to-implementation (State 7) and the formal-verifier (State 12) MUST re-validate the coupling. If vb-cib14 diverges, the plan must re-plan (rerun_from=4 on every obligation). The mandatory re-run of `bash scripts/check-verus-production-binding.sh` after implementation lands is the gate that surfaces coupling drift.

## 12. Approval Conditions

The plan is approved. proof-writer (State 5) may proceed with:

1. Authoring `verification/verus/extern_vb_edvbj_storage_event.rs`, `verification/verus/extern_vb_edvbj_propagation.rs`, `verification/verus/extern_vb_edvbj_symbolic_code.rs`, and `verification/verus/extern_vb_edvbj_mirror_bind.rs` (or the consolidated mirror companion).
2. Authoring Kani harnesses under `harnesses/kani/` for PO-EDVBJ-002 and PO-EDVBJ-006.
3. Authoring proptest properties under `crates/vb_runtime/src/journal/tests/` and `crates/vb_runtime/src/error/tests_diagnostics/` for PO-EDVBJ-003, 004, 010.
4. Authoring Flux refinement annotations on `crates/vb_runtime/src/error/diagnostics.rs` for PO-EDVBJ-008.
5. Recording trusted-base-ledger rows for each TB-* entry documented in `trusted-base-plan.md`.
6. Re-running `bash scripts/check-verus-production-binding.sh` after the implementation lands to validate PO-EDVBJ-007.

---

## STATUS: APPROVED

(Independent reviewer invocation; all 14 lane decisions accepted; 10 proof obligations schema-valid; 4 Verus obligations production-bound via STRONG or WEAK_MIRROR; Kani split-harness present (6 harnesses); temporal-replay proptest present; no behavior-affecting waivers; 3 non-blocking findings logged; STRONG coupling with vb-cib14 documented and gated by the Verus production-binding gate re-run.)
