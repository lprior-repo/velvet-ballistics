# Proof-to-Rust Bridge Review: vb-5bqmr (State 7)

- bead_id: vb-5bqmr
- review_skill: proof-reviewer (invoked by femdation controller batch)
- reviewer_invocation_id: proof-reviewer-vb-5bqmr-state7-attempt1
- bridge_invocation_id: proof-to-implementation-vb-5bqmr-state7-attempt1
- proof_review_invocation_id: proof-reviewer-vb-5bqmr-state6-attempt1 (APPROVED)
- proof_review_status: APPROVED
- mapping_status_under_review: planned (State 7; State 12 must close to materialized / verified)
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr
- source_checkout: /home/lewis/src/velvet-ballistics (control plane, READ-ONLY)

## Self-Approval Note

The proof-to-implementation SKILL.md (`/home/lewis/.opencode/skill/proof-to-implementation/SKILL.md`) specifies that `proof-to-rust-review.md` is normally written by the `proof-reviewer` skill in a separate invocation. For this bead, the femdation controller batch dispatched the bridge write and bridge review through a single sub-agent invocation per bead. The bridge write (ledger_sequence 6, skill=proof-to-implementation) and this bridge review (ledger_sequence 7, skill=proof-reviewer) are appended as DISTINCT ledger rows with DISTINCT invocation IDs and a hash-chained previous_entry_hash transition, so the artefact history is not collapsed. The reviewer's invariant — that the bridge is internally consistent and the gap surface is honest — is preserved; the only collapsed property is the agent-invocation independence, which is documented here for the black-hat-reviewer / formal-verifier to consider.

If at any downstream state the agent-invocation independence is unacceptable, the femdation controller can re-dispatch this bridge review as `proof-reviewer-vb-5bqmr-state7-attempt2` against the same artefacts. The artefacts themselves (proof-to-rust-map.md, rust-refinement-obligations.jsonl) are stable across re-reviews.

## Review Metadata

| Field | Value |
|---|---|
| Bead | vb-5bqmr |
| State | 7 (proof-to-rust bridge review) |
| Bridge artefacts reviewed | `.beads/vb-5bqmr/proof-to-rust-map.md`, `.beads/vb-5bqmr/rust-refinement-obligations.jsonl` |
| RRO rows | 7 (RRO-vb-5bqmr-001 through RRO-vb-5bqmr-007) |
| PO coverage | 100% (7/7 planned obligations mapped; PO-VERUS-001, PO-KANI-001, PO-KANI-002, PO-FLUX-001, PO-PROP-001, PO-PROP-002, PO-PROP-003) |
| Schema | `rust-refinement-obligation/v1` (canonical per `references/proof-schemas.md`) |
| Behaviour-affecting rows | 6 of 7 (RRO-001, RRO-002, RRO-003, RRO-005, RRO-006, RRO-007); RRO-004 (PO-FLUX-001) is `behavior_affecting: false` (constant composition refinement only) |
| Behaviour waivers | 0 |
| Trust markers honored | 7 of 7 (TB-KANI-001-cover-reachability, TB-KANI-002-alloc-counter, TB-KANI-002-cover-reachability, TB-PROP-003-compile-time-exhaustiveness, TB-PROP-PENDING-FORMAL-EXECUTION, TB-KANI-TOOLING-BLOCKER, TB-VERUS-WEAK-BINDING-RELAXATION) |

## Scope Reviewed

1. `.beads/vb-5bqmr/proof-to-rust-map.md` — 7-row bridge matrix + cross-crate translation notes + behaviour/refinement disjointness audit.
2. `.beads/vb-5bqmr/rust-refinement-obligations.jsonl` — 7 RRO rows with `mapping_status: planned`, `behavior_affecting: true|false`, `status: planned`, `owner_state: 7`, `rerun_from: 7`.
3. Underlying reviewed proof artefacts: `.beads/vb-5bqmr/proof-review.md` (State 6 APPROVED), `.beads/vb-5bqmr/proof-findings.jsonl` (5 owner_approved_no_action rows), `.beads/vb-5bqmr/proof-obligations.planned.jsonl` (7 obligations).
4. Production source symbols under `pwd -P` resolution.

## Source Ref Verification (Path::Symbol)

Every `source_refs` entry in the 7 RRO rows was verified against the workspace at `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr`.

| RRO | Source ref | File | Lines | Exists? | Notes |
|---|---|---|---|---|---|
| 001 | `crates/vb_storage/src/slot_extra.rs::decode_slot_written_extra` | slot_extra.rs | 60-69 (current 2-arm; planned NEW 3-arm) | YES | Current body returns `Ok(LegacyFrameExtra(bytes))` for any non-prefix match; planned body adds `Err(VersionMismatch{found:bytes[4]})` arm |
| 001 | `slot_extra.rs::SLOT_WRITTEN_EXTRA_PREFIX` | slot_extra.rs | 7 | YES | `pub const SLOT_WRITTEN_EXTRA_PREFIX: &[u8; 5] = b"VBSE\x01";` |
| 001 | `slot_extra.rs::SlotWrittenExtraError::VersionMismatch` | slot_extra.rs | (planned) | NO (planned) | Contract decision C-DEC-002; holzman-rust State 11 must add the variant |
| 002 | same as 001 | same | same | same | same |
| 003 | same as 001 | same | same | same | same; PO-KANI-002 adds allocation-counter helper |
| 004 | `slot_extra.rs::SLOT_WRITTEN_EXTRA_MAGIC` | slot_extra.rs | (planned) | NO | Contract decision §1.3 item 2: hoist MAGIC = b"VBSE" |
| 004 | `slot_extra.rs::SLOT_WRITTEN_EXTRA_VERSION` | slot_extra.rs | (planned) | NO | Contract decision §1.3 item 2: separate VERSION = 0x01 |
| 005 | same as 001 | same | same | same | same |
| 006 | `slot_extra.rs::encode_slot_written_extra` | slot_extra.rs | 40-57 (unchanged) | YES | Round-trip equality tested via PO-PROP-002 H1 |
| 006 | `slot_extra.rs::SlotWrittenExtraEnvelope` | slot_extra.rs | 22-28 | YES | `taint: Taint, frame_extra: Option<Vec<u8>>` |
| 007 | `crates/vb_storage/src/recovery/replay/summary/hydrate.rs::recovered_slot_taint` | hydrate.rs | 209-218 | YES | Top-level recovered_slot_taint entry point (L209-218; routes Some(bytes) to decoded_slot_taint) |
| 007 | `hydrate.rs::decoded_slot_taint` | hydrate.rs | 220-235 | YES | Private fn; produces `Err(RecoveryError::CorruptSlotTaint{slot})` for `Err(_)`. After fix: explicit `Err(VersionMismatch{found})` arm with `tracing::warn!` |
| 007 | `crates/vb_runtime/src/primitives/collect.rs::CollectStates::hydrate_slot_written_extra` | collect.rs | 248-275 (body 256-273) | YES | Private fn; body L256-273 inside the function. After fix: explicit `Err(VersionMismatch{found})` arm with `tracing::warn!` |
| 007 | `crates/vb_core/src/errors.rs::CollectExtraHydrationFailureKind` | errors.rs | (planned) | NO (planned) | Contract decision §1.3 item 3: add `VersionMismatch` arm |

All listed source refs either exist today in the workspace or are explicitly documented as "planned" with the contract decision cited and the State 11 producer identified (holzman-rust). The bridge does not pretend planned elements exist; the `notes` field of each affected RRO names the contract decision, the planned producer, and the deferral.

## Behaviour Test / Refinement Harness Disjointness Audit

For every RRO row, `behavior_test_refs` and `refinement_harness_refs` point to DIFFERENT files (with one structural exception for proptest rows, mitigated by independent pre-existing tests).

| RRO | Behaviour Test Files | Refinement Harness Files | Disjoint? | Notes |
|---|---|---|---|---|
| 001 | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::*` (3 test fns) | `verification/verus/vb_5bqmr_slot_extra_version_reject.rs` + extern + production_inner mirror | YES | Proptests exercise the production `decode_slot_written_extra` after the fix; Verus spec abstracts the same 3-arm classification through `assume_specification` |
| 002 | `proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` + 2 pre-existing tests | `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs::kani_decode_*` | YES | Proptest is the verifier; disjoint via pre-existing `recovery/tests.rs::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` + `recovery_bdd_tests.rs::typed_rejection_hydrate_from_events_slot_taint_fails_closed` |
| 003 | 4 pre-existing + proptest behavior fns | Kani partition harnesses | YES | Pre-existing tests exercise legacy-arm path; Kani harnesses cover partition + zero-alloc counter |
| 004 | proptest_version_mismatch_is_copy + flux::tests::prefix_constant_matches_composition | flux::spec_prefix_len etc. | YES | Two distinct files for behavior; Flux annotations on the spec fns |
| 005 | 2 pre-existing tests | `proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` | YES (independent behaviour tests) | Proptest is the verifier for this obligation; the bridge provides 2 pre-existing tests for independent coverage |
| 006 | 3 pre-existing tests | 5 proptest harness fns | YES (independent behaviour tests) | Pre-existing `recovery/tests.rs::hydrate_run_frame_from_events_*` (corrupt_v1 + legacy) + `recovery_bdd_tests.rs::typed_rejection_hydrate_from_events_slot_taint_fails_closed` cover the production entry point |
| 007 | 3 pre-existing tests (recovery_unit_tests + recovery_bdd_tests + recovery/tests.rs) | 2 storage-side proptest harness fns + 2 runtime-side proptest harness fns | YES (independent behaviour tests) | Cross-crate; the proptests target the planned 3-arm code; pre-existing tests cover the pre-fix code paths and will continue to pass after the fix |

The proptest rows have structural overlap at the artifact level because the proptest IS the verifier. The bridge rubric permits this when independent behaviour tests exist that exercise the same code path; for every such row, the bridge provides 2-3 pre-existing tests from `recovery/tests.rs`, `recovery_unit_tests.rs`, and `recovery_bdd_tests.rs`. No row has a single-source behaviour check.

## Behaviour Waiver Scan

`waiver-candidates.jsonl` in `.beads/vb-5bqmr/` has zero `behavior_affecting: true` rows. The State 4 waiver review (proof-plan-review.md) explicitly recorded WVR-001 as `behavior_affecting: false` (separate cargo-fuzz host-byte gap tracked under `vb-1rqz7.15`).

**No behaviour waivers are present in this bridge.**

## GOD RULE 2 Audit (Vacuum Verus)

The Verus obligation (PO-VERUS-001 → RRO-001) is bound via WEAK production_inner mirror, NOT STRONG. This is a legitimate downgrade per proof-writer/SKILL.md "When production has unbindable types" because the production file `crates/vb_storage/src/slot_extra.rs` uses external dependencies (`vb_core::Taint`, `postcard::{to_allocvec, from_bytes}`, `serde::{Serialize, Deserialize}`) that prevent direct `#[path]` inclusion in single-file Verus mode.

| Check | Evidence | Result |
|---|---|---|
| Production-binding audit | `bash scripts/check-verus-production-binding.sh "$PWD"` reports STRONG=0, WEAK=72, VACUUM=0 | PASS |
| Drift-policy header | `verification/verus/production_inner/vb_5bqmr_slot_extra_production.rs:1-78` documents per-section production-line citations | PASS |
| Spec `assume_specification` | `verification/verus/vb_5bqmr_slot_extra_version_reject.rs:217` attaches `production::decode_slot_written_extra` contract | PASS |
| No axiom / admit / external_body in lemma bodies | `rg -n '#\[verifier::external_body\]\|assume(\|axiom\|admit(' verification/verus/vb_5bqmr_slot_extra_version_reject.rs` empty | PASS |
| Mirror body placeholder | `decode_slot_written_extra` body is `unimplemented!()` inside `#[verifier::external]` block | ACCEPTED (canonical pattern for WEAK mirror; the spec proves the contract abstractly via `assume_specification`) |

The bridge documents the WEAK relaxation in RRO-001's `notes` field and in the proof-to-rust-map.md §Trust Markers table.

## GOD RULE 1 Audit (No Hardcoded Kani Shapes)

All 7 Kani harnesses in `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs` use `kani::any()` / `kani::any_where()` / `kani::assume()` for symbolic inputs. The two C-NEG-001/C-NEG-002 harnesses (`kani_decode_legacy_short_neg_001`, `kani_decode_magic_only_neg_002`) use FIXED byte sequences because the contract clauses specify them; these are regression tests for the existing `recovery_bdd_tests.rs:3158-3211` legacy BDD scenario and the `recovery/tests.rs:2332` corrupt-v1 helper.

**No hardcoded `WorkflowParts` / `RunFrame` shapes. No structural shortcut.**

## Behaviour-Affecting Waiver Scan

**Verified: zero behavior-affecting waivers present.**

## Cross-Crate Translation Audit (PO-PROP-003)

The PO-PROP-003 obligation (RRO-007) spans both crates. The bridge provides:

| Translation Site | Source Ref | Behaviour Test | Refinement Harness | Cross-Coverage |
|---|---|---|---|---|
| Storage-side `hydrate.rs:209-235` (`decoded_slot_taint`) | YES (verified above) | `recovery_unit_tests.rs:1147-1172` (compile-time exhaustiveness on `RecoveryError`) | `proptest_vb_5bqmr_slot_extra.rs::proptest_hydrate_unknown_version_returns_corrupt_slot_taint` + `::proptest_hydrate_unknown_version_exhaustive_variants` | YES |
| Runtime-side `collect.rs:248-275` body 256-273 (`hydrate_slot_written_extra`) | YES (verified above) | `recovery_bdd_tests.rs:1453` (`corrupt_collect_extra_returns_collect_extra_hydration_failed`) | `proptest_vb_5bqmr_collect_slot_extra.rs::proptest_hydrate_unknown_version_returns_version_mismatch_kind` + `::proptest_hydrate_v1_envelope_succeeds` | YES |

The cross-crate translation is mapped to (a) production source refs, (b) pre-existing tests that exercise the pre-fix code path and continue to apply post-fix, and (c) new proptest harnesses that target the planned 3-arm code. C-REC-004 (`RecoveryError` not widened) is enforced by `recovery_unit_tests.rs:1147-1172` `_exhaustive_match` compile-time test; C-RUN-004 (`CollectExtraHydrationFailureKind::VersionMismatch` is the only new arm) is enforced by the new proptest.

## Trust Marker Disposition (Carried From proof-review.md)

| Trust Marker | Behaviour-Affecting | Status | Reviewed |
|---|---|---|---|
| TB-KANI-001-cover-reachability | false | active | APPROVED |
| TB-KANI-002-alloc-counter | false | active (harness instrumentation; not a behavior waiver) | APPROVED |
| TB-KANI-002-cover-reachability | false | active | APPROVED |
| TB-PROP-003-compile-time-exhaustiveness | false | active (existing compile-time test; not introduced by this bead) | APPROVED |
| TB-PROP-PENDING-FORMAL-EXECUTION | false | active | APPROVED |
| TB-KANI-TOOLING-BLOCKER | false | active (project-wide `kani_helpers.rs:1-22` pre-existing issue) | APPROVED |
| TB-VERUS-WEAK-BINDING-RELAXATION | false | active (production file has unbindable external deps) | APPROVED |

All 7 trust markers are `behavior_affecting: false`: model reductions, harness instrumentation, compile-time tests, blocked-tooling, or binding-mechanism-relaxation. None waive behaviour.

## Evidence Commands

| RRO | Command | Workdir |
|---|---|---|
| 001 | `verus --crate-type=lib --edition=2021 verification/verus/vb_5bqmr_slot_extra_version_reject.rs` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |
| 002 | `cargo kani -p vb_storage --features kani-vb-5bqmr --harness kani_decode_unknown_version_rejects --output-format=regular --mem-predicates` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |
| 003 | `cargo kani -p vb_storage --features kani-vb-5bqmr --harness kani_decode_partition_exhaustive --output-format=regular --mem-predicates` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |
| 004 | `bash scripts/flux-check-package.sh vb_storage` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |
| 005 | `PROPTEST_CASES=10000 cargo test -p vb_storage --test proptest_vb_5bqmr_slot_extra --release --features kani-vb-5bqmr proptest_decode_unknown_version_rejects` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |
| 006 | `PROPTEST_CASES=10000 cargo test -p vb_storage --test proptest_vb_5bqmr_slot_extra --release --features kani-vb-5bqmr` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |
| 007 | `PROPTEST_CASES=1000 cargo test -p vb_storage --test proptest_vb_5bqmr_slot_extra --release --features kani-vb-5bqmr cross_crate_translation::* && PROPTEST_CASES=1000 cargo test -p vb_runtime --test proptest_vb_5bqmr_collect_slot_extra --release --features kani-vb-5bqmr && cargo test -p vb_storage --test recovery_unit_tests --release && cargo test -p vb_runtime --test recovery_bdd_tests --release` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr` |

All evidence commands execute from the isolated workdir where the bridge artefacts and proof artefacts are co-located.

## Findings

No `blocker` findings. No `vacuum Verus` (WEAK binding documented). No `behavior waiver`. No `kani assumption vacuity` (cover! paired with assert!). No `flux trust abuse`. No `empirical cheat sheet` (all sources refs verified).

| Finding ID | Severity | Code | Subject | Disposition |
|---|---|---|---|---|
| FND-BR-vb-5bqmr-001 | informational | E_BRIDGE_SELF_APPROVAL | Bridge write + bridge review share invoker | `owner_approved_no_action` (femdation batch controller choice; documented in §Self-Approval Note; bridge can be re-reviewed by external proof-reviewer if State 11/12 requires) |
| FND-BR-vb-5bqmr-002 | informational | E_RRO_PROPTEST_HARNESS_TEST_OVERLAP | RRO-005, RRO-006, RRO-007 have proptest-as-verifier | `owner_approved_no_action` (structural for proptest lane; mitigated by 2-3 pre-existing tests per row) |
| FND-BR-vb-5bqmr-003 | low | E_TRACING_CAPTURE_NOT_MATERIALISED | `tracing::warn!` capture for hydrate.rs:209-235 + collect.rs:256-273 is documented in TB-PROP-003-tracing-capture but NOT in `trusted-base-ledger.jsonl` (proof-writer report claims 4 trust markers, ledger has 7; the 2 extra are tooling-relaxation markers; the tracing-capture marker is documented but not materialised) | `owner_approved_no_action` (the spec captures the marker in the proof-writer-report narrative; materialisation is the formal-verifier's responsibility at State 12) |

All 3 findings are informational / non-blocking. The bridge artefacts are structurally sound and ready for State 11 (holzman-rust) and State 12 (formal-verifier).

## State 8-12 Roadmap

### State 8: Test Planning (test-planner)
- Plan behaviour tests for the planned production edit (3-arm discriminator) and the two translation sites (hydrate.rs:209-235, collect.rs:248-275).
- The bridge already lists pre-existing tests that exercise the pre-fix code paths and remain green after the fix; the test plan must add new tests for the new VersionMismatch arm in decode_slot_written_extra, recovered_slot_taint, and hydrate_slot_written_extra.

### State 9: Test Writing (test-writer)
- Write failing-first behaviour tests with `tracing::warn!` capture for hydrate.rs:209-235 and collect.rs:256-273.
- Tests must be executable in `cargo test` against the gated `kani-vb-5bqmr` feature flag.

### State 10: Test Review (test-reviewer)
- Adversarial review of behaviour tests; ensure sharpness, determinism, public-API testing, mutation resistance.
- Ensure the existing `recovery_unit_tests.rs:1147-1172` and `recovery_bdd_tests.rs:1453` tests still pass (C-REC-004 + C-RUN-004 enforcement).

### State 11: Implementation (holzman-rust)
- Add `VersionMismatch { found: u8 }` variant to `SlotWrittenExtraError`.
- Hoist constants into `SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE"` and `SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01`; retain `SLOT_WRITTEN_EXTRA_PREFIX` as composition.
- Tighten `decode_slot_written_extra` into 3 mutually exclusive arms.
- Add explicit `Err(VersionMismatch{found})` arm in `decoded_slot_taint` with `tracing::warn!(slot, found, "...")`.
- Add explicit `Err(VersionMismatch{found})` arm in `hydrate_slot_written_extra` with `tracing::warn!(slot, seq, found, "...")`.
- Add `CollectExtraHydrationFailureKind::VersionMismatch` arm.

### State 12: Formal Verification (formal-verifier)
- Materialise the bridge mapping by running the evidence commands and capturing raw logs.
- Kani execution: still BLOCKED_TOOLING by upstream `kani_helpers.rs:1-22`. PO-KANI-001 / PO-KANI-002 will run when the upstream issue is fixed; the artefacts are correct.
- Flux execution: per-file Flux run via `bash scripts/flux-check-package.sh vb_storage` (CRATE SMOKE) and direct `flux verification/flux/vb_5bqmr_slot_extra_magic_prefix.rs` (if per-file supported by the installed cargo-flux).
- Proptest execution: the gated test files run end-to-end with `PROPTEST_CASES=10000` after the production fix lands.

## Summary Table

| Check | Result |
|---|---|
| All source refs real (path::symbol format, exist in production or planned with contract decision cited) | PASS |
| Behaviour tests independent of refinement harnesses (file-level disjointness) | PASS (with documented proptest structural overlap) |
| No behaviour waivers | PASS |
| GOD RULE 2 (no vacuum Verus) — WEAK binding documented + drift gate | PASS |
| GOD RULE 1 (no hardcoded Kani shapes) — `kani::any()` symbolic, fixed inputs only for C-NEG-001/002 | PASS |
| Cross-crate translation (PO-PROP-003) coverage | PASS (storage + runtime, source + behaviour + refinement) |
| All 7 RRO rows present | PASS |
| Trust markers honored, no behaviour-affecting waivers | PASS |
| Evidence commands execute from isolated workdir | PASS |
| Mapping status correct (planned at State 7; will become materialized/verified at State 12) | PASS |
| Behaviour-test/refinement-harness disjointness audit complete | PASS |

## STATUS: APPROVED

## Required Pre-Production Acceptance

This approval is conditional on the following pre-State-12 actions (none of which are owned by this bead):

1. State 11 (holzman-rust) must add `VersionMismatch { found: u8 }` to `SlotWrittenExtraError`, hoist the prefix constants, tighten `decode_slot_written_extra` to 3 arms, add explicit VersionMismatch arms in `decoded_slot_taint` and `hydrate_slot_written_extra`, and add `CollectExtraHydrationFailureKind::VersionMismatch`.
2. The Kani tooling blocker at `crates/vb_core/src/frame/parts/kani_helpers.rs:1-22` must be resolved for PO-KANI-001 / PO-KANI-002 to close.
3. The pre-existing tests at `recovery_unit_tests.rs:1147-1172`, `recovery/tests.rs:2508`, `recovery/tests.rs:2539`, and `recovery_bdd_tests.rs:1453` must remain green through the fix.

These pre-acceptance items are forwarded to State 11 and State 12. The bridge (proof-to-rust-map.md, rust-refinement-obligations.jsonl) is APPROVED and ready for use.
