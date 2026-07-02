---
bead_id: vb-qxjgx
bead_title: "p7-proof-to-implementation + bridge review for vb-qxjgx (StepSucceeded/SlotWritten record-kind split)"
phase: 7
updated_at: 2026-07-01T22:35:00Z
attempt: 1
reviewer_skill: proof-to-implementation (combined bridge + review for femdation batch cheap25)
reviewer_invocation_id: p7-proof-to-implementation-attempt1
---

# Proof-to-Rust Review — vb-qxjgx

## Bridge Adequacy Assessment

Reviewed `proof-to-rust-map.md` and `rust-refinement-obligations.jsonl` (7 RRO rows: RRO-vb-qxjgx-001..007). Reviewed production source files at the canonical anchor paths. Cross-checked against `proof-review.md` (state 6, STATUS: APPROVED, 5 findings, 0 blocker).

The bridge is **ADEQUATE**. All 7 behavior-affecting obligations map to production code via concrete `path::symbol` source refs (not file-only). All 7 obligations have independent behavior test refs (NOT verifier harnesses). Kani-backed obligations have separate refinement harness refs (5 kani files with paired kani::cover!/kani::assert reachability witnesses). Proptest-backed obligations note that the proptest file IS the verification artifact per skill guidance, with the anti-invariant token `invalid_input` closing the pre-fix collapse vacuity path.

## RRO-vb-qxjgx-001: RecordKind::StepSucceeded wire id bijection

- **Source refs**: `crates/vb_storage/src/records.rs::RecordKind (line 139, enum declaration)`, `records.rs::RecordKind::id (line 210, const fn)`, `records.rs::StepSucceeded (post-fix arm to be added at state 11)`
- **Test refs**: `crates/vb_storage/src/codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630, POST-001 witness)`, `codec/tests.rs::slot_written_event_maps_to_slot_written_kind_unchanged (line 1650, PRE-005)`, `codec/tests.rs::step_succeeded_and_slot_written_record_kinds_are_distinct (line 1672, INV-001)`
- **Kani harness**: `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs::{check_step_succeeded_kind_id, check_no_other_kind_collides_with_33, check_closed_set_includes_33}` — STRONG production binding to `crate::records::RecordKind::id`
- **Bridge adequacy**: ADEQUATE. The 3 back-compat tests at codec/tests.rs:1630-1692 are independent executable `#[test]` functions (NOT verifier harnesses) that would FAIL pre-fix and PASS post-fix. The kani harness file is properly feature-gated under `kani-vb-qxjgx-record-kind-split` and registered in `crates/vb_storage/src/lib.rs:100-109`. Pre-fix E0599 at `codec/tests.rs:1639` (TBR-002 forward_looking) clears post-implementation.

## RRO-vb-qxjgx-002: JournalEvent::record_kind one-to-one projection

- **Source refs**: `crates/vb_storage/src/events.rs::JournalEvent::record_kind (lines 401-429, match expression)`, `events.rs (line 406, pre-fix OR-collapse arm to be split)`, `records.rs::RecordKind::id (line 210)`
- **Test refs**: `codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630, POST-002)`, `codec/tests.rs::slot_written_event_maps_to_slot_written_kind_unchanged (line 1650)`, `codec/tests.rs::step_succeeded_and_slot_written_record_kinds_are_distinct (line 1672, INV-001)`
- **Kani harness**: `crates/vb_storage/src/kani_record_kind_projection_split.rs::{check_step_succeeded_record_kind_projection, check_slot_written_event_record_kind_projection, check_projection_is_one_to_one_on_split_pair}` — STRONG production binding to `crate::events::JournalEvent::record_kind` and `crate::records::RecordKind::id`
- **Bridge adequacy**: ADEQUATE. The pre-fix OR-collapse at events.rs:406 is the canonical seam for the split; post-fix the match arm must separate `Self::StepSucceeded => RecordKind::StepSucceeded` and `Self::SlotWrittenEvent => RecordKind::SlotWritten`. kani::cover! at line 68 paired with kani::assert at line 54 proves the StepSucceeded projection arm is reachable AND projects to RecordKind::StepSucceeded. Pre-conditions `kani::assume(run != 0)` and `kani::assume(seq != u64::MAX)` mirror `JournalEvent::is_valid()` at events.rs:514-550 (TBR-003 accepted; not a property short-circuit).

## RRO-vb-qxjgx-003: Kind 33 family admit

- **Source refs**: `crates/vb_storage/src/codec/validation.rs::is_known_record_kind (line 23, predicate; line 24 matches! to extend with 33)`, `validation.rs::validate_kind_family (lines 42-60, family admit/reject matrix; line 47 MAGIC_JOURNAL_EVENT arm to extend)`, `records.rs::RecordKind (line 139)`
- **Test refs**: `codec/tests.rs::canonical_id_33_round_trip_step_succeeded (line 1734)`, `codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702)`, `codec/tests.rs::slot_written_with_envelope_id_33_is_rejected (line 1765)`, `proptest_durability_matrix_step_succeeded.rs::schema_version_is_pinned_at_one (line 130, family admit/reject grid)`
- **Kani harness**: `crates/vb_storage/src/kani_record_kind_journal_family_33.rs::{check_kind_33_known, check_kind_33_journal_family_admit, check_kind_33_snapshot_family_rejected, check_kind_33_blob_family_rejected, check_journal_family_exhaustive_includes_33, check_id_known_family_lockstep}` — STRONG production binding to `crate::codec::validation::is_known_record_kind` and `crate::codec::validation::validate_kind_family`
- **Bridge adequacy**: ADEQUATE. The 6 kani harnesses cover the closed-set extension (id 33 admit), the family admit (MAGIC_JOURNAL_EVENT + 33 = Ok), the family rejects (MAGIC_SNAPSHOT/BLOB + 33 = Err), the u16 exhaustive sweep (mirrors pre-existing `kani_record_kind.rs:265-289`), and the lockstep witness (INV-003: id()/is_known_record_kind/validate_kind_family in lockstep). 2 paired kani::cover!/kani::assert (lines 41, 67). Hard-coded 29-entry golden array at line 84-87 paired with kani::assert inside the loop body — drift between the array and the production function is caught by the assertion (TBR-007 accepted).

**TBR-010 routing item acknowledged:** the pre-existing `check_unknown_kind_rejected` at `kani_record_kind.rs:180-188` hardcodes id 33 as the 'unknown kind' and asserts `validate_kind_family(MAGIC_JOURNAL_EVENT, 33).is_err() == true`. After holzman-rust (state 11) lands the production change to admit id 33 in the journal family, this pre-existing harness will FAIL. The new `check_kind_33_journal_family_admit` at `kani_record_kind_journal_family_33.rs:H2` is the replacement. Deletion/update must happen in lockstep with the production change. This is a State 11 routing item, not a bridge adequacy gap.

## RRO-vb-qxjgx-004: Parity gate dual-envelope acceptance grid

- **Source refs**: `crates/vb_storage/src/codec/kind_parity.rs::EnforceKindParity::enforce_kind_parity (lines 50-64, trait impl override)`, `codec/mod.rs::validate_journal_event_record_kind (lines 97-111, impl parity pair)`, `events.rs::JournalEvent::record_kind (lines 401-429)`
- **Test refs**: `codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702, POST-005 back-compat — direct call to validate_journal_event_record_kind)`, `codec/tests.rs::canonical_id_33_round_trip_step_succeeded (line 1734, POST-006 round-trip — exercises validate_journal_event_record_kind via decode_journal_event)`, `codec/tests.rs::slot_written_with_envelope_id_33_is_rejected (line 1765, POST-007)`, `codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630)`
- **Kani harness**: `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs::{check_parity_accepts_step_succeeded_kind_33, check_parity_accepts_step_succeeded_legacy_kind_12, check_parity_rejects_step_succeeded_other_envelope_kinds, check_parity_rejects_slot_written_kind_33, check_parity_accepts_slot_written_kind_12, check_parity_rejects_slot_written_other_envelope_kinds, check_parity_impls_agree_on_full_grid}` — STRONG production binding to `crate::codec::EnforceKindParity::enforce_kind_parity` AND `crate::codec::validate_journal_event_record_kind` (impl parity pair)
- **Bridge adequacy**: ADEQUATE. The 7 kani harnesses cover the full acceptance grid (StepSucceeded × {12, 33, others}, SlotWrittenEvent × {12, 33, others}) AND the impl-pair parity (H7 asserts EnforceKindParity and validate_journal_event_record_kind agree on every cell of the grid). kani::cover! at line 117 paired with kani::assert at line 106 proves the legacy envelope-12 + StepSucceeded branch is reachable AND the parity gate accepts it. The 4 back-compat tests at codec/tests.rs:1630-1791 are the independent executable behavior tests. Pre-conditions (kani::assume run!=0, seq!=u64::MAX, attempt!=0) mirror JournalEvent::is_valid() (TBR-003).

## RRO-vb-qxjgx-005: decode_journal_event round-trip

- **Source refs**: `crates/vb_storage/src/codec/mod.rs::encode_record (lines 60-71, canonical encoder)`, `codec/mod.rs::validate_journal_event_record_kind (lines 97-111, parity impl called by decode_journal_event)`, `codec/mod.rs::decode_journal_event (lines 126-151, decoder entry point)`, `codec/mod.rs (lines 143-148, typed ReplayEnvelopeSequenceMismatch identity check)`, `codec/kind_parity.rs::EnforceKindParity (lines 50-64, called via decode_record at mod.rs:93)`
- **Test refs**: `codec/tests.rs::canonical_id_33_round_trip_step_succeeded (line 1734, POST-006 canonical round-trip — calls encode_record(...,RecordKind::StepSucceeded,...) and decode_journal_event)`, `codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702, POST-005 back-compat)`
- **Kani harness**: `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs::{check_decode_round_trip_canonical_kind_33, check_decode_round_trip_legacy_kind_12_step_succeeded, check_decode_rejects_envelope_seq_mismatch}` — STRONG production binding to `crate::codec::decode_journal_event` and `crate::codec::encode_record`
- **Bridge adequacy**: ADEQUATE. H1 exercises the canonical round-trip via encode_record (emits id 33) + decode_journal_event. H2 synthesizes a legacy envelope (id 12 + StepSucceeded payload) and calls EnforceKindParity directly — this is the only way to exercise the legacy tolerance post-fix because the post-fix encoder emits id 33 (TBR-008 accepted, mirrors the pre-existing `kani_record_kind.rs:107-134` pattern). H3 exercises the sequence identity check (POST-013) — kani::assert at line 213 confirms `envelope.sequence == decoded.seq.get()`. The 2 back-compat tests at codec/tests.rs:1734, 1702 are the independent executable behavior tests.

## RRO-vb-qxjgx-006: Recovery summary variant-keyed counters

- **Source refs**: `crates/vb_storage/src/recovery/replay/summary/apply.rs::apply_summary_event (lines 23-85, variant-keyed counter incrementer)`, `apply.rs (lines 32-34 StepSucceeded arm; lines 51-53 SlotWrittenEvent arm)`, `events.rs::JournalEvent::record_kind (lines 401-429)`, `events.rs::JournalEvent::is_valid (lines 514-550)`
- **Test refs**: `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs::{post_split_steps_succeeded_is_variant_keyed (H1), post_split_slots_written_does_not_include_step_succeeded (H2), post_split_record_kind_projection_is_bijective (H3, INV-001 closure), id_keyed_counter_would_diverge_from_variant_keyed (H4, E_KANI_ASSUMPTION_VACUITY anti-invariant)}`
- **Proptest file IS the verification artifact** (no separate refinement harness per skill guidance for proptest-only obligations): the proptest source contains the literal `invalid_input` anti-invariant token at line 38 and `prop_filter(ANTI_INVARIANT_TOKEN, ...)` at line 112 closing the pre-fix-collapse vacuity path (TBR-009).
- **Bridge adequacy**: ADEQUATE. The proptest directly exercises `apply_summary_event` (the production variant-keyed counter incrementer) with a strategy that mixes StepSucceeded (envelope ids 12 and 33) and SlotWrittenEvent (envelope id 12). H4 is the E_KANI_ASSUMPTION_VACUITY closure: prop_assert_ne! at line 277 asserts the variant-keyed counter (1) differs from the id-keyed counter (0) when envelope_id_step=12, proving the divergence between the pre-fix collapse (id-keyed would be 0) and the post-fix contract (variant-keyed is 1). Pre-fix E0599 at proptest_replay_summary_step_succeeded_split.rs:224 (TBR-002 forward_looking; line 224 not 222 per FIND-QXJGX-R004 fixed_with_evidence) clears post-implementation.

## RRO-vb-qxjgx-007: Durability matrix, schema pin, flux literal-sync, family grid

- **Source refs**: `crates/vb_runtime/src/durability_matrix.rs::DURABILITY_MATRIX (lines 70-204, const matrix)`, `durability_matrix.rs::REQUIRED_PRIMITIVES (lines 51-63)`, `durability_matrix.rs::DurabilityRow::journal_events (line 39)`, `codec/validation.rs::validate_kind_family (lines 42-60, family admit/reject matrix)`, `codec/mod.rs::validate_record_kind_family (lines 55-57, public re-export surface)`, `constants.rs::CURRENT_SCHEMA_VERSION (line 58)`, `codec/flux_validation.rs (lines 14, 33, DISABLED flux-rs literal source)`
- **Test refs**: `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs::{durability_matrix_step_closing_rows_use_step_succeeded (H1), schema_version_is_pinned_at_one (H2), flux_validation_literals_include_id_33 (H3), durability_matrix_storage_and_ack_invariants (H4), anti_invariant_token_present (line 263-268)}`, `codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630, INV-001 cross-crate)`
- **Proptest file IS the verification artifact** (no separate refinement harness per skill guidance): the proptest source contains the literal `invalid_input` anti-invariant token at line 45 and the explicit unit test `anti_invariant_token_present` at lines 263-268 closes the pre-fix collapse vacuity path (TBR-009).
- **Bridge adequacy**: ADEQUATE. The proptest directly walks DURABILITY_MATRIX (the production const matrix at vb_runtime/src/durability_matrix.rs:70-204) and asserts StepSucceeded at the closing position of the 10 step-closing rows + no SlotWritten in any position + finish row retains RunFinished (H1). H2 pins CURRENT_SCHEMA_VERSION=1 directly via `prop_assert_eq!(CURRENT_SCHEMA_VERSION, 1u16, ...)` (PRE-005, INV-006) and asserts the family admit/reject grid for id 33 (POST-004 cross-crate from vb_runtime integration test). H3 parses the literal source of `crates/vb_storage/src/codec/flux_validation.rs` via `std::fs::read_to_string` (line 186) and asserts id 33 appears in both the literal known set AND the journal-family refinement literal (POST-011 literal-sync for the DISABLED flux-rs module per vb-b8i8f). H4 asserts every row has valid StoragePartition and AckPoint != BeforeJournalAppend (POST-008 side-conditions). TBR-005 deviation (validate_schema_version re-routed through the public validate_record_kind_family surface because the function is pub(crate)) is accepted; the in-crate tests at tests.rs:3925, 4223 cover the direct validate_schema_version call. Pre-fix E0599 at proptest_durability_matrix_step_succeeded.rs:96 (TBR-002 forward_looking) clears post-implementation.

## Behavior-Affecting Proof Claims Without Test Coverage

None. All 7 behavior-affecting obligations have:
- Independent executable behavior tests (6 back-compat tests at codec/tests.rs:1630-1791 for PO-QXJGX-001, 002, 004, 005; proptest files for PO-QXJGX-006, 007)
- Or, where the verification IS the behavior test (proptest-backed obligations), explicit prop_assert_eq!/prop_assert!/prop_assert_ne! assertions on production function calls

## Verifier-Only Waivers

None. All 7 obligations are behavior-affecting and have explicit behavior test paths.

## Backward Compatibility Coverage

**NOTE: BACKWARD COMPATIBILITY = LEGACY ENVELOPE-12 TOLERANCE, NOT A SCHEMA BUMP.**

The contract is explicit: `CURRENT_SCHEMA_VERSION` remains 1 (PRE-005, INV-006), and pre-fix journals with envelope id 12 + StepSucceeded payload continue to decode via the parity gate's LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] } (POST-005).

Verified by:
- `crates/vb_storage/src/constants.rs:58`: `pub const CURRENT_SCHEMA_VERSION: u16 = 1;` (verified on disk, UNCHANGED by this bead)
- `codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702-1728)`: direct call to `validate_journal_event_record_kind` with envelope id 12 + StepSucceeded payload
- `kani_record_kind_parity_legacy_envelope.rs:H2` (line 99-121): proves the parity gate accepts envelope id 12 + StepSucceeded payload
- `kani_record_kind_decode_round_trip.rs:H2` (line 113-172): proves the decoder accepts the legacy envelope-12 + StepSucceeded pair
- `codec/tests.rs::slot_written_event_maps_to_slot_written_kind_unchanged (line 1650)`: asserts SlotWrittenEvent still uses wire id 12 (PRE-005 invariant)
- `proptest_durability_matrix_step_succeeded.rs:H2 (line 130-168)`: directly pins `CURRENT_SCHEMA_VERSION=1`
- `codec/tests.rs:3925` and `codec/tests.rs:4223` enforce the CURRENT_SCHEMA_VERSION=1 pin (UNCHANGED by this bead)

## Trust Marker Inventory (10 TBR rows)

| TBR | trusted_kind | status | obligation_id(s) | reviewer_disposition |
|---|---|---|---|---|
| TBR-001 | block | blocked | PO-QXJGX-001..005 | accepted (pre-existing vb_core kani_helpers.rs unclosed delimiter; NOT caused by this bead) |
| TBR-002 | forward_looking | pending_formal_execution | PO-QXJGX-001..007 | accepted (4 forward-looking E0599 errors clear post-implementation) |
| TBR-003 | assume | accepted | PO-QXJGX-002, 004, 005 | accepted (kani::assume pre-conditions mirror JournalEvent::is_valid() at events.rs:514-550) |
| TBR-004 | const | accepted | PO-QXJGX-007 | accepted (CURRENT_SCHEMA_VERSION=1 pinned at constants.rs:58) |
| TBR-005 | deviation | accepted | PO-QXJGX-007 | accepted (validate_schema_version re-routed through public validate_record_kind_family surface) |
| TBR-006 | deviation | accepted | PO-QXJGX-006, 007 | accepted (planned.jsonl artifact paths override task description paths) |
| TBR-007 | extern_spec | accepted | PO-QXJGX-001, 003 | accepted (hard-coded golden arrays paired with kani::assert on production function call) |
| TBR-008 | model | accepted | PO-QXJGX-005-H2 | accepted (synthesized envelope mirrors pre-existing kani_record_kind.rs:107-134 pattern) |
| TBR-009 | non_vacuity | accepted | PO-QXJGX-006-H1..H4, PO-QXJGX-007-H1..H4 | accepted (invalid_input literal anti-invariant token) |
| TBR-010 | block | blocked | PO-QXJGX-001..005 | accepted (pre-existing kani_record_kind.rs:180-188 check_unknown_kind_rejected will FAIL post-implementation; routing to state 11 holzman-rust) |

**No new TBR rows needed for the bridge.** All 7 bridge rows map to existing TBR coverage.

## Source Refs vs. Bridge Standard

Every `source_refs` array names concrete `path::symbol` refs (not file-only). Examples:
- `crates/vb_storage/src/records.rs::RecordKind (line 139, enum declaration)` — names the enum AND the line
- `crates/vb_storage/src/events.rs::JournalEvent::record_kind (lines 401-429, match expression)` — names the function AND the lines
- `crates/vb_storage/src/codec/kind_parity.rs::EnforceKindParity::enforce_kind_parity (lines 50-64, trait impl override)` — names the trait method AND the lines
- `crates/vb_runtime/src/durability_matrix.rs::DURABILITY_MATRIX (lines 70-204, const matrix)` — names the const AND the lines

No file-only refs, no prose refs, no `verification/` extern_*.rs (Verus out of scope), no `production_inner/*_production.rs` mirrors.

## Behavior Test Refs vs. Verifier Harnesses

The 6 back-compat unit tests at `crates/vb_storage/src/codec/tests.rs:1617-1791` are independent executable `#[test]` functions that would FAIL pre-fix and PASS post-fix. They are NOT verifier harnesses.

The 5 kani files are VERIFIER HARNESSES, not behavior tests. They are listed under `refinement_harness_refs` for kani-backed obligations.

The 2 proptest files are BOTH verification artifacts AND behavior tests (per skill guidance for proptest-only obligations), with the `invalid_input` anti-invariant token closing the vacuity path. The 9 properties across the 2 files (H1-H4 in proptest_replay_summary_step_succeeded_split.rs + H1-H4 + anti_invariant_token_present in proptest_durability_matrix_step_succeeded.rs) all directly exercise production functions and would FAIL if the production behavior were deleted.

## Bridge Approval

**STATUS: APPROVED**

The proof-to-rust bridge is adequate. All 7 behavior-affecting obligations have:
- Concrete `path::symbol` source refs (not file-only)
- Independent behavior test refs (6 back-compat unit tests + 2 proptest files with prop_assert_eq! on production function calls)
- Separate refinement harness refs for kani-backed obligations (5 kani files with paired kani::cover!/kani::assert reachability witnesses)
- Proptest-backed obligations note the proptest file IS the verification artifact per skill guidance, with anti-invariant token + prop_filter closing the vacuity path
- Exact verifier command with `cargo kani`/`cargo test` invocation, `evidence_workdir` set to the bead workdir, and `evidence_artifact` pointing to proof-evidence.md or the proptest source file

No Rust-evidence waivers for behavior-affecting claims. No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in production paths. No VACUUM Verus artifacts (none in scope). No `cover!`-as-proof obligations. No `assert(true)` or `cover!`-as-acceptance in any harness body.

**State 11 (holzman-rust) routing items acknowledged:**
1. records.rs:139 — add `StepSucceeded = 33` arm to the enum
2. records.rs:210 — add `Self::StepSucceeded => 33` arm to `RecordKind::id()`
3. events.rs:406 — split the OR-collapse into two separate match arms
4. validation.rs:24 — extend `matches!(kind, ...)` to include 33
5. validation.rs:47 — extend `MAGIC_JOURNAL_EVENT` arm to admit `kind == 33`
6. kind_parity.rs:50 — extend `EnforceKindParity::enforce_kind_parity for JournalEvent` to accept envelope ids `{12, 33}` for StepSucceeded
7. durability_matrix.rs:70-204 — substitute `RecordKind::SlotWritten` → `RecordKind::StepSucceeded` in the closing position of the 10 step-closing rows
8. kani_record_kind.rs:180-188 — DELETE or UPDATE `check_unknown_kind_rejected` in lockstep with the production change (TBR-010)
9. constants.rs:58 — UNCHANGED (`CURRENT_SCHEMA_VERSION = 1` per PRE-005)
10. codec/mod.rs:97-111 — UNCHANGED (validate_journal_event_record_kind already accepts any envelope_kind; parity is in kind_parity.rs)
11. codec/mod.rs:126-151 — UNCHANGED at the seam; only the called parity impl changes
12. codec/mod.rs:143-148 — UNCHANGED (POST-013 sequence identity check is already in the decoder)

No repairs needed before State 11. Bridge is approved for the State 7 → State 11 → State 12 transition.