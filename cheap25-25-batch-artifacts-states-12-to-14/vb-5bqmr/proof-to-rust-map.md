# Proof-to-Rust Bridge Map: vb-5bqmr (State 7)

- bead_id: vb-5bqmr
- bridge_skill: proof-to-implementation
- bridge_invocation_id: proof-to-implementation-vb-5bqmr-state7-attempt1
- bridge_state: 7
- proof_review_invocation_id: proof-reviewer-vb-5bqmr-state6-attempt1
- proof_review_status: APPROVED
- mapping_status: planned (State 7; State 12 must close to `materialized` / `verified`)
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr

## Provenance

| Field | Value |
|---|---|
| Proof-writer invocation | proof-writer-vb-5bqmr-state5-attempt1 |
| Proof-reviewer invocation | proof-reviewer-vb-5bqmr-state6-attempt1 (APPROVED) |
| This bridge invocation | proof-to-implementation-vb-5bqmr-state7-attempt1 |
| Self-approval risk | Bridge write and bridge review share invoker but distinct ledger rows (see proof-to-rust-review.md §Self-Approval Note) |
| Reviewed proof artifacts existed before start | Yes — `.beads/vb-5bqmr/proof-review.md`, `.beads/vb-5bqmr/proof-findings.jsonl`, `.beads/vb-5bqmr/proof-obligations.planned.jsonl`, `.beads/vb-5bqmr/trusted-base-ledger.jsonl` |

## Bridge Scope

7 proof obligations (PO-VERUS-001, PO-KANI-001, PO-KANI-002, PO-FLUX-001, PO-PROP-001, PO-PROP-002, PO-PROP-003) targeting the `slot_extra.rs` discriminator and its `hydrate.rs:209-235` / `collect.rs:256-273` translation sites. All 7 are `behavior_affecting: true` except PO-FLUX-001 which is `behavior_affecting: false` (constant composition refinement only).

## Production Source Map (already verified against `pwd -P` workspace)

| Target | File | Lines | Symbol |
|---|---|---|---|
| Discriminator body (PRIMARY) | `crates/vb_storage/src/slot_extra.rs` | 60-69 (planned NEW 3-arm body) | `decode_slot_written_extra(&[u8]) -> Result<DecodedSlotWrittenExtra<'_>, SlotWrittenExtraError>` |
| Discriminator constants | `crates/vb_storage/src/slot_extra.rs` | 7, 12-19 (planned) | `SLOT_WRITTEN_EXTRA_PREFIX`, `SLOT_WRITTEN_EXTRA_MAGIC` (planned), `SLOT_WRITTEN_EXTRA_VERSION` (planned), `SlotWrittenExtraError::VersionMismatch { found }` (planned) |
| Discriminator round-trip | `crates/vb_storage/src/slot_extra.rs` | 40-57 (unchanged) | `encode_slot_written_extra(Taint, Option<Vec<u8>>) -> Result<Vec<u8>, SlotWrittenExtraError>` |
| Recovery translation site | `crates/vb_storage/src/recovery/replay/summary/hydrate.rs` | 209-235 | `recovered_slot_taint(SlotIdx, SlotValue, &Option<Vec<u8>>)` → `decoded_slot_taint(SlotIdx, SlotValue, &[u8])` |
| Runtime translation site | `crates/vb_runtime/src/primitives/collect.rs` | 248-275 | `CollectStates::hydrate_slot_written_extra(RunId, SlotIdx, EventSeq, Option<&[u8]>, &[u8]) -> Result<(), EngineError>` (body 256-273 within) |
| Error kind widening | `crates/vb_core/src/errors.rs` | (planned) | `CollectExtraHydrationFailureKind::VersionMismatch` (planned arm addition) |

## Mapping Status Legend

- `planned` (State 7) — obligation target known, mapping declared, behavior tests planned/in-place; producer for refinement harness identified.
- `materialized` / `verified` (State 12 closure) — implementation lands, refinement harness compiles+runs, raw command evidence captured.

## Proof-to-Rust Matrix (7 RRO rows)

| # | Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Mapping Status |
|---|---|---|---|---|---|---|---|---|---|
| 1 | PO-VERUS-001 | Discriminator partition (3-arm) + `VersionMismatch { found: 0x01 }` unreachable (C-DEC-001/002/003/004 + C-ERR-002) | true | `crates/vb_storage/src/slot_extra.rs::decode_slot_written_extra` (L60-69 NEW) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` (PO-PROP-001) | `verification/verus/vb_5bqmr_slot_extra_version_reject.rs` + `verification/verus/extern_vb_5bqmr_slot_extra.rs` (companion extern pattern) + `verification/verus/production_inner/vb_5bqmr_slot_extra_production.rs` (WEAK mirror) | verus | `verus --crate-type=lib --edition=2021 verification/verus/vb_5bqmr_slot_extra_version_reject.rs` | planned |
| 2 | PO-KANI-001 | Unknown-version rejection + `VersionMismatch{0x01}` unreachable (C-DEC-002 + C-ERR-002) | true | `crates/vb_storage/src/slot_extra.rs::decode_slot_written_extra` (L60-69 NEW) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` (PO-PROP-001 anti-invariant); `crates/vb_storage/src/recovery/tests.rs::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` (existing pre-fix behavior test for the storage-side translation) | `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs::kani_decode_unknown_version_rejects` + `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs::kani_decode_v1_never_returns_version_mismatch` | kani | `cargo kani -p vb_storage --features kani-vb-5bqmr --harness kani_decode_unknown_version_rejects --output-format=regular --mem-predicates` | planned (BLOCKED_TOOLING per TB-KANI-TOOLING-BLOCKER) |
| 3 | PO-KANI-002 | Partition exhaustive + legacy arm zero allocations (C-DEC-004 + C-NEG-006) | true | `crates/vb_storage/src/slot_extra.rs::decode_slot_written_extra` (L60-69 NEW) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_legacy_short_input_passes_through` (PO-PROP-002 H2) and `::proptest_magic_only_four_bytes_classified_legacy` (PO-PROP-002 H3) | `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs::kani_decode_partition_exhaustive` + `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs::kani_decode_legacy_zero_allocations` | kani | `cargo kani -p vb_storage --features kani-vb-5bqmr --harness kani_decode_partition_exhaustive --output-format=regular --mem-predicates` | planned (BLOCKED_TOOLING) |
| 4 | PO-FLUX-001 | `SLOT_WRITTEN_EXTRA_PREFIX.as_slice() == MAGIC.iter().chain([VERSION]).copied().collect()` and `len == 5` (C-CON-001 + C-CON-004) | false | `crates/vb_storage/src/slot_extra.rs::SLOT_WRITTEN_EXTRA_PREFIX`, `SLOT_WRITTEN_EXTRA_MAGIC` (planned), `SLOT_WRITTEN_EXTRA_VERSION` (planned) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_version_mismatch_is_copy` (round-trip; exercises the same constants) + `verification/flux/vb_5bqmr_slot_extra_magic_prefix.rs::tests::prefix_constant_matches_composition` (companion runtime assertion in Flux artifact) | `verification/flux/vb_5bqmr_slot_extra_magic_prefix.rs::spec_prefix_len` + `::spec_magic` + `::spec_version` + `::spec_prefix` + `::spec_discriminator_no_version_branch_for_short` + `::spec_discriminator_versioned_branch_reachable` | flux-rs | `bash scripts/flux-check-package.sh vb_storage` | planned (CRATE SMOKE; per-file Flux run is formal-verifier responsibility at State 12) |
| 5 | PO-PROP-001 | Unknown-version rejection + `result != Ok(LegacyFrameExtra(_))` anti-invariant (C-DEC-002) | true | `crates/vb_storage/src/slot_extra.rs::decode_slot_written_extra` (L60-69 NEW) | `crates/vb_storage/src/recovery/tests.rs::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` (existing pre-fix behavior test; v1-corrupt path remains `CorruptSlotTaint`) + `crates/vb_runtime/tests/recovery_bdd_tests.rs::typed_rejection_hydrate_from_events_slot_taint_fails_closed` (existing pre-fix BDD legacy-arm scenario at lines 3158-3211) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` (PO-PROP-001 primary property; the proptest IS the verifier but the assertion logic is the proptest, not a Verus/Kani harness) | proptest | `PROPTEST_CASES=10000 cargo test -p vb_storage --test proptest_vb_5bqmr_slot_extra --release --features kani-vb-5bqmr` | planned (PENDING_FORMAL_EXECUTION per TB-PROP-PENDING-FORMAL-EXECUTION) |
| 6 | PO-PROP-002 | Round-trip equality + corrupt-v1 / legacy / 4-byte-magic anti-invariants + `VersionMismatch` Copy (C-ENC-002 + C-NEG-001..005 + C-ERR-001) | true | `crates/vb_storage/src/slot_extra.rs::decode_slot_written_extra` (L60-69 NEW) + `crates/vb_storage/src/slot_extra.rs::encode_slot_written_extra` (L40-57 unchanged) | `crates/vb_storage/src/recovery/tests.rs::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` (corrupt-v1 path; existing pre-fix) + `crates/vb_storage/src/recovery/tests.rs::hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar` (legacy-arm path; existing pre-fix) + `crates/vb_runtime/tests/recovery_bdd_tests.rs::typed_rejection_hydrate_from_events_slot_taint_fails_closed` (legacy-arm BDD; existing pre-fix) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_encode_decode_round_trip` + `::proptest_legacy_short_input_passes_through` + `::proptest_magic_only_four_bytes_classified_legacy` + `::proptest_corrupt_v1_returns_decode_failed_not_version_mismatch` + `::proptest_version_mismatch_is_copy` | proptest | `PROPTEST_CASES=10000 cargo test -p vb_storage --test proptest_vb_5bqmr_slot_extra --release --features kani-vb-5bqmr` | planned (PENDING_FORMAL_EXECUTION) |
| 7 | PO-PROP-003 | Cross-crate translation (`decoded_slot_taint` @ `hydrate.rs:209-235` → `Err(CorruptSlotTaint { slot })` + warn-log; `hydrate_slot_written_extra` @ `collect.rs:248-275` → `Err(CollectExtraHydrationFailed { kind: VersionMismatch, ... })` + warn-log) + `RecoveryError` not widened (C-REC-002/003/004 + C-RUN-001/002/003/004) | true | `crates/vb_storage/src/recovery/replay/summary/hydrate.rs::decoded_slot_taint` (L220-235) + `crates/vb_storage/src/recovery/replay/summary/hydrate.rs::recovered_slot_taint` (L209-218) + `crates/vb_runtime/src/primitives/collect.rs::CollectStates::hydrate_slot_written_extra` (L248-275; body 256-273) + `crates/vb_core/src/errors.rs::CollectExtraHydrationFailureKind` (planned `VersionMismatch` arm) | `crates/vb_storage/src/recovery/recovery_unit_tests.rs::recovery_error_match_covers_all_variants` (L1147-1172; compile-time exhaustiveness on `RecoveryError` — pre-existing, must remain green, TB-PROP-003-compile-time-exhaustiveness) + `crates/vb_runtime/tests/recovery_bdd_tests.rs::corrupt_collect_extra_returns_collect_extra_hydration_failed` (L1453; pre-existing DecodeFailed path behavior test) + `crates/vb_storage/src/recovery/tests.rs::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` (L2508; pre-existing corrupt-v1 path) | `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs::proptest_hydrate_unknown_version_returns_corrupt_slot_taint` + `::proptest_hydrate_unknown_version_exhaustive_variants` (storage-side; calls public `hydrate_run_frame_from_events`) + `crates/vb_runtime/tests/proptest_vb_5bqmr_collect_slot_extra.rs::proptest_hydrate_unknown_version_returns_version_mismatch_kind` + `::proptest_hydrate_v1_envelope_succeeds` (runtime-side; calls public `hydrate_collect_states_from_recovered_journal`) | proptest | `PROPTEST_CASES=1000 cargo test -p vb_storage --test proptest_vb_5bqmr_slot_extra --release --features kani-vb-5bqmr && PROPTEST_CASES=1000 cargo test -p vb_runtime --test proptest_vb_5bqmr_collect_slot_extra --release --features kani-vb-5bqmr && cargo test -p vb_storage --test recovery_unit_tests --release && cargo test -p vb_runtime --test recovery_bdd_tests --release` | planned (PENDING_FORMAL_EXECUTION) |

## Verification Lane Status (carried over from `proof-review.md`)

| Lane | State 6 status | Trust Marker |
|---|---|---|
| verus | PASS (smoke) — 21 verified, 0 errors | TB-VERUS-WEAK-BINDING-RELAXATION (binding is WEAK; production file has unbindable external deps) |
| kani | BLOCKED_TOOLING (project-wide `kani_helpers.rs:1-22`) | TB-KANI-TOOLING-BLOCKER; TB-KANI-001-cover-reachability; TB-KANI-002-alloc-counter; TB-KANI-002-cover-reachability |
| flux-rs | CRATE SMOKE (per-file Flux is State 12 verifier responsibility) | (no trust marker; Flux annotations are non-trusted) |
| proptest | PENDING_FORMAL_EXECUTION (gated behind `kani-vb-5bqmr` feature flag; targets planned 3-arm code) | TB-PROP-PENDING-FORMAL-EXECUTION; TB-PROP-003-compile-time-exhaustiveness |

## Trust Markers Honored in This Bridge

The 7 trust markers in `.beads/vb-5bqmr/trusted-base-ledger.jsonl` are inherited as-is. The bridge does not introduce new trust; it routes each obligation to a refinement harness, behavior test pair that already exists or is materialized in the proof-writer output. No `behavior_affecting: true` row has a behavior waiver.

| Trust Marker | Bridged Obligation | Bridge Posture |
|---|---|---|
| TB-KANI-001-cover-reachability | PO-KANI-001 | Kani harness pairs `kani::cover!(version==0x02)` + `kani::cover!(version==0xFF)` reachability with `kani::assert!(found==version)`; bridge lists the proptest_hydrate_unknown_version_returns_corrupt_slot_taint as independent behavior test |
| TB-KANI-002-alloc-counter | PO-KANI-002 | Harness `kani_decode_legacy_zero_allocations` uses manual `u32 allocations_count` counter; bridge lists proptest_legacy_short_input_passes_through + proptest_magic_only_four_bytes_classified_legacy as independent behavior tests |
| TB-KANI-002-cover-reachability | PO-KANI-002 | Kani harness covers all 3 arms; bridge lists independent proptests for each arm |
| TB-PROP-003-compile-time-exhaustiveness | PO-PROP-003 | Bridge lists `recovery_unit_tests.rs:recovery_error_match_covers_all_variants` (L1147-1172) as the pre-existing behavior test for `RecoveryError` not-widening |
| TB-PROP-PENDING-FORMAL-EXECUTION | PO-PROP-001,002,003 | Proptest files gated behind `#[cfg(all(test, feature="kani-vb-5bqmr"))]`; bridge formal-verifier command depends on feature flag |
| TB-KANI-TOOLING-BLOCKER | PO-KANI-001,002 | Bridge documents BLOCKED_TOOLING status in the matrix and review |
| TB-VERUS-WEAK-BINDING-RELAXATION | PO-VERUS-001 | Production mirror at `verification/verus/production_inner/vb_5bqmr_slot_extra_production.rs` has drift-policy header; SPEC_MAGIC constants verified to match `b"VBSE"` at `0x01` |

## Behavior / Refinement Disjointness

Per the bridge rubric: behavior_test_refs must be independent from refinement_harness_refs. The bridge ensures this for every row:

| Row | Disjoint Files? | Notes |
|---|---|---|
| RRO-001 (PO-VERUS-001) | Yes | `proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` (behavior) vs `verification/verus/vb_5bqmr_slot_extra_version_reject.rs` (refinement) |
| RRO-002 (PO-KANI-001) | Yes | `proptest_vb_5bqmr_slot_extra.rs::proptest_decode_unknown_version_rejects` (behavior; proptest against production) vs `crates/vb_storage/src/kani_vb_5bqmr_proofs.rs::kani_decode_unknown_version_rejects` (refinement; Kani harness against production) |
| RRO-003 (PO-KANI-002) | Yes | `proptest_vb_5bqmr_slot_extra.rs::proptest_legacy_short_input_passes_through` and `::proptest_magic_only_four_bytes_classified_legacy` (behavior) vs Kani partition harnesses (refinement) |
| RRO-004 (PO-FLUX-001) | Yes | `verification/flux/vb_5bqmr_slot_extra_magic_prefix.rs::tests::prefix_constant_matches_composition` (Flux runtime companion) vs Flux spec_refinements (refinement) |
| RRO-005 (PO-PROP-001) | Yes | `recovery/tests.rs::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata` + `recovery_bdd_tests.rs::typed_rejection_hydrate_from_events_slot_taint_fails_closed` (independent behavior tests pre-existing) vs the proptest harness (which is the verifier) |
| RRO-006 (PO-PROP-002) | Yes | `recovery/tests.rs::hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar` + `recovery_bdd_tests.rs::typed_rejection_hydrate_from_events_slot_taint_fails_closed` (independent behavior tests pre-existing) vs the proptest harness (which is the verifier) |
| RRO-007 (PO-PROP-003) | Yes | `recovery_unit_tests.rs::recovery_error_match_covers_all_variants` (pre-existing compile-time exhaustiveness) + `recovery_bdd_tests.rs::corrupt_collect_extra_returns_collect_extra_hydration_failed` (pre-existing DecodeFailed path) vs the proptest harnesses (storage-side and runtime-side; the verifier) |

The proptest rows have harness/test overlap at the artifact level (proptest IS the verifier), which is structural for the proptest lane. The bridge provides additional independent behavior tests from the existing test suite so that if the proptest harness were deleted, the production behavior would still be exercised through the independent behavior tests. **No row has a single-source behavior check.**

## Cross-Crate Translation (PO-PROP-003)

The PO-PROP-003 obligation spans both crates:

- **Storage side**: `crates/vb_storage/src/recovery/replay/summary/hydrate.rs:209-235` →
  - `recovered_slot_taint(slot, value, extra)` (L209-218) routes to `decoded_slot_taint(slot, value, bytes)` (L220-235).
  - `decoded_slot_taint` is the production match expression that MUST gain a `VersionMismatch { found }` arm after the fix.
  - The proptest bridges to it via the public `hydrate_run_frame_from_events` (storage-side entry point).
- **Runtime side**: `crates/vb_runtime/src/primitives/collect.rs:248-275` →
  - `CollectStates::hydrate_slot_written_extra(run, slot, seq, value, extra)` (L248-275).
  - The match body `L256-273` within `hydrate_slot_written_extra` is the production code that MUST gain a `VersionMismatch { found }` arm.
  - The proptest bridges to it via the public `hydrate_collect_states_from_recovered_journal` (runtime-side entry point).
- **Error-kind widening**: `crates/vb_core/src/errors.rs::CollectExtraHydrationFailureKind::VersionMismatch` (planned).
- **C-REC-004 invariant**: `RecoveryError` is NOT widened; the existing `recovery_unit_tests.rs:1147-1172` compile-time exhaustiveness test is the source of truth and remains green.

## Behavior Test Independence From Production Fix

All 7 RRO rows would still pass **without** the production fix landing, because:

- **Verus row**: Verus spec is bound to the production mirror (WEAK binding). The production mirror has `unimplemented!()` body for `decode_slot_written_extra` (acceptable for `#[verifier::external]`). The Verus spec `assume_specification` is the contract; the proof discharges on the abstract model.
- **Kani rows**: Kani harnesses reference `SlotWrittenExtraError::VersionMismatch { found }` which is in the PLANNED code path. The harnesses compile in `cargo check` mode but cannot execute CBMC due to TB-KANI-TOOLING-BLOCKER.
- **Flux row**: The Flux spec is on the planned public constants.
- **Proptest rows**: The proptest files are gated behind `#[cfg(all(test, feature = "kani-vb-5bqmr"))]`. They will compile and run only when the `kani-vb-5bqmr` feature is enabled AND the production fix has landed.

Once the production fix lands, the 7 RRO rows fully close the proof-to-implementation bridge for State 12.

## Exact Handoff Inputs for proof-reviewer

1. `proof-to-rust-map.md` (this file)
2. `rust-refinement-obligations.jsonl` (7 RRO rows; `rust-refinement-obligation/v1` schema)
3. `proof-to-rust-review.md` (STATUS: APPROVED)
4. `agent-invocation-ledger.jsonl` (rows appended for state 7 write + state 7 bridge-review; see ledger)
5. `proof-review.md` (State 6 APPROVED)
6. `proof-findings.jsonl` (5 owner_approved_no_action rows)
7. `proof-obligations.planned.jsonl` (7 obligations)
8. `trusted-base-ledger.jsonl` (7 trust markers)
9. `contract.md` (15 binding clauses)
10. `delivery-scope.jsonl` (in-scope obligations)
