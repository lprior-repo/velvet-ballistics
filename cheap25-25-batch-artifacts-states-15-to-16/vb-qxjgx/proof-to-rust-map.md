# Proof-to-Rust Map: vb-qxjgx State 7

bridge_skill: proof-to-implementation
bridge_invocation_id: p7-proof-to-implementation-attempt1
proof_review_invocation_id: vb-qxjgx-state6-proof-review-attempt1
proof_review_status: APPROVED (5 findings, 0 blocker, 4 fixed_with_evidence + 1 owner_approved_no_action)
mapping_status: planned

## Provenance

- Reviewer skill: `proof-to-implementation`
- Bridge invocation ID: `p7-proof-to-implementation-attempt1` (this map)
- Proof-reviewer invocation ID: `vb-qxjgx-state6-proof-review-attempt1` (parent state 6, STATUS: APPROVED)
- Proof-writer invocation: `p5-proof-writer: Kani x5 + proptest x2 for vb-qxjgx` (state 5)
- Proof-plan-reviewer invocation: `vb-qxjgx-state4-proof-plan-review-attempt1` (state 4, STATUS: APPROVED)
- Workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
- State: 7 (proof-to-implementation)
- Bead: `vb-qxjgx` (StepSucceeded / SlotWritten record-kind split)

## Production Binding Classification

All 7 obligations bind **STRONG** to production code via canonical `crate::...` paths.
No shadow models, no `verification/` extern_*.rs, no `production_inner/*_production.rs` mirrors.
Verus is out of scope per `contract.md` NON-GOALS and `proof-strategy.md §5` — no `binding_classification` required.
Kani is the primary verifier for PO-QXJGX-001..005; proptest for PO-QXJGX-006..007.

## Proof-to-Rust Matrix (7 obligations)

| Proof ID | Contract Clause | Behavior Affecting | Rust Source Refs (anchor in **bold**) | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Mapping Status |
|---|---|---|---|---|---|---|---|---|
| **PO-QXJGX-001** | POST-001 (StepSucceeded=33; closed-set bijection) | true | **crates/vb_storage/src/records.rs::RecordKind (line 139, enum)**; records.rs::RecordKind::id (line 210, const fn); records.rs::StepSucceeded (post-fix arm) | codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630); codec/tests.rs::slot_written_event_maps_to_slot_written_kind_unchanged (line 1650); codec/tests.rs::step_succeeded_and_slot_written_record_kinds_are_distinct (line 1672) | kani_record_kind_id_step_succeeded.rs::{check_step_succeeded_kind_id, check_no_other_kind_collides_with_33, check_closed_set_includes_33} | kani | `cargo kani -j 1 --output-format=regular --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split --harness check_step_succeeded_kind_id` | planned |
| **PO-QXJGX-002** | POST-002 (JournalEvent::record_kind one-to-one; OR-collapse removed) | true | **crates/vb_storage/src/events.rs::JournalEvent::record_kind (lines 401-429, match expression)**; events.rs (line 406, pre-fix OR-collapse arm to split); records.rs::RecordKind::id (line 210) | codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630); codec/tests.rs::slot_written_event_maps_to_slot_written_kind_unchanged (line 1650); codec/tests.rs::step_succeeded_and_slot_written_record_kinds_are_distinct (line 1672) | kani_record_kind_projection_split.rs::{check_step_succeeded_record_kind_projection, check_slot_written_event_record_kind_projection, check_projection_is_one_to_one_on_split_pair} | kani | `cargo kani -j 1 --output-format=regular --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split --harness check_step_succeeded_record_kind_projection` | planned |
| **PO-QXJGX-003** | POST-003 (is_known(33)=true) + POST-004 (validate_kind_family MAGIC_JOURNAL_EVENT 33 = Ok) | true | **crates/vb_storage/src/codec/validation.rs::is_known_record_kind (line 23, predicate; line 24 matches! to extend)**; **codec/validation.rs::validate_kind_family (lines 42-60, family admit/reject matrix; line 47 MAGIC_JOURNAL_EVENT arm to extend)**; records.rs::RecordKind (line 139, source of truth for id 33) | codec/tests.rs::canonical_id_33_round_trip_step_succeeded (line 1734); codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702); codec/tests.rs::slot_written_with_envelope_id_33_is_rejected (line 1765); proptest_durability_matrix_step_succeeded.rs::schema_version_is_pinned_at_one (line 130) | kani_record_kind_journal_family_33.rs::{check_kind_33_known, check_kind_33_journal_family_admit, check_kind_33_snapshot_family_rejected, check_kind_33_blob_family_rejected, check_journal_family_exhaustive_includes_33, check_id_known_family_lockstep} | kani | `cargo kani -j 1 --output-format=regular --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split --harness check_kind_33_journal_family_full` | planned |
| **PO-QXJGX-004** | POST-005 (parity {12,33} for StepSucceeded, {12} for SlotWrittenEvent) + POST-007 (SlotWritten+33 rejected) + INV-004 | true | **crates/vb_storage/src/codec/kind_parity.rs::EnforceKindParity::enforce_kind_parity (lines 50-64, trait impl override)**; **codec/mod.rs::validate_journal_event_record_kind (lines 97-111, impl parity pair)**; events.rs::JournalEvent::record_kind (lines 401-429) | codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702); codec/tests.rs::canonical_id_33_round_trip_step_succeeded (line 1734); codec/tests.rs::slot_written_with_envelope_id_33_is_rejected (line 1765); codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630) | kani_record_kind_parity_legacy_envelope.rs::{check_parity_accepts_step_succeeded_kind_33, check_parity_accepts_step_succeeded_legacy_kind_12, check_parity_rejects_step_succeeded_other_envelope_kinds, check_parity_rejects_slot_written_kind_33, check_parity_accepts_slot_written_kind_12, check_parity_rejects_slot_written_other_envelope_kinds, check_parity_impls_agree_on_full_grid} | kani | `cargo kani -j 1 --output-format=regular --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split --harness check_parity_gate_step_succeeded_legacy` | planned |
| **PO-QXJGX-005** | POST-006 (decode_journal_event round-trips canonical id-33 and legacy id-12+StepSucceeded) + POST-013 (envelope.sequence == payload.seq identity) | true | **crates/vb_storage/src/codec/mod.rs::encode_record (lines 60-71, canonical encoder)**; **codec/mod.rs::validate_journal_event_record_kind (lines 97-111, parity impl called by decode_journal_event)**; **codec/mod.rs::decode_journal_event (lines 126-151, decoder entry point)**; codec/mod.rs (lines 143-148, typed ReplayEnvelopeSequenceMismatch identity check); codec/kind_parity.rs::EnforceKindParity (lines 50-64, called via decode_record at mod.rs:93) | codec/tests.rs::canonical_id_33_round_trip_step_succeeded (line 1734, POST-006 canonical round-trip); codec/tests.rs::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted (line 1702, POST-005 back-compat) | kani_record_kind_decode_round_trip.rs::{check_decode_round_trip_canonical_kind_33, check_decode_round_trip_legacy_kind_12_step_succeeded, check_decode_rejects_envelope_seq_mismatch} | kani | `cargo kani -j 1 --output-format=regular --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split --harness check_decode_round_trip_step_succeeded` | planned |
| **PO-QXJGX-006** | POST-009 (recovery summary counters variant-keyed) + INV-008 | true | **crates/vb_storage/src/recovery/replay/summary/apply.rs::apply_summary_event (lines 23-85, variant-keyed counter incrementer)**; apply.rs (lines 32-34 StepSucceeded arm; lines 51-53 SlotWrittenEvent arm); events.rs::JournalEvent::record_kind (lines 401-429); events.rs::JournalEvent::is_valid (lines 514-550) | proptest_replay_summary_step_succeeded_split.rs::{post_split_steps_succeeded_is_variant_keyed (H1), post_split_slots_written_does_not_include_step_succeeded (H2), post_split_record_kind_projection_is_bijective (H3, INV-001 closure), id_keyed_counter_would_diverge_from_variant_keyed (H4, E_KANI_ASSUMPTION_VACUITY anti-invariant)} | (the proptest file IS the verification artifact; anti-invariant token `invalid_input` at line 38 + prop_filter at line 112 close the pre-fix-collapse vacuity path; TBR-009) | proptest | `PROPTEST_CASES=10000 cargo test --test proptest_replay_summary_step_succeeded_split --release -p vb_storage` | planned |
| **PO-QXJGX-007** | POST-008 (durability matrix step-closing rows use StepSucceeded) + POST-011 (flux_validation literals include id 33) + PRE-005 (CURRENT_SCHEMA_VERSION=1) + POST-004 (kind 33 family admit) | true | **crates/vb_runtime/src/durability_matrix.rs::DURABILITY_MATRIX (lines 70-204, const matrix)**; durability_matrix.rs::REQUIRED_PRIMITIVES (lines 51-63); durability_matrix.rs::DurabilityRow::journal_events (line 39); **codec/validation.rs::validate_kind_family (lines 42-60, family admit/reject matrix used by H2)**; codec/mod.rs::validate_record_kind_family (lines 55-57, public re-export surface); constants.rs::CURRENT_SCHEMA_VERSION (line 58, schema-version pin); codec/flux_validation.rs (lines 14, 33, DISABLED flux-rs literal source for POST-011 literal-sync) | proptest_durability_matrix_step_succeeded.rs::{durability_matrix_step_closing_rows_use_step_succeeded (H1), schema_version_is_pinned_at_one (H2), flux_validation_literals_include_id_33 (H3, parses flux_validation.rs source via std::fs::read_to_string), durability_matrix_storage_and_ack_invariants (H4), anti_invariant_token_present (line 263-268)}; codec/tests.rs::step_succeeded_event_maps_to_step_succeeded_kind (line 1630, INV-001 cross-crate) | (the proptest file IS the verification artifact; anti-invariant token `invalid_input` at line 45 + explicit unit test `anti_invariant_token_present` at lines 263-268 close the pre-fix collapse vacuity path; TBR-009) | proptest | `PROPTEST_CASES=10000 cargo test --test proptest_durability_matrix_step_succeeded --release -p vb_runtime` | planned |

## Source-Ref Anchors vs. Task Description

The task description enumerates 6 anchor paths:

| Task anchor | Primary obligation | Primary row |
|---|---|---|
| records.rs:139 | PO-QXJGX-001 (RecordKind enum) | RRO-vb-qxjgx-001 |
| events.rs:406 | PO-QXJGX-002 (record_kind projection OR-collapse arm) | RRO-vb-qxjgx-002 |
| validation.rs:23-60 | PO-QXJGX-003 (is_known + validate_kind_family) | RRO-vb-qxjgx-003 |
| kind_parity.rs:50 | PO-QXJGX-004 (EnforceKindParity impl; primary parity gate) | RRO-vb-qxjgx-004 |
| codec/mod.rs:97 | PO-QXJGX-004 + PO-QXJGX-005 (validate_journal_event_record_kind; impl parity + called by decode_journal_event) | RRO-vb-qxjgx-004 + RRO-vb-qxjgx-005 (both span codec/mod.rs:97) |
| durability_matrix.rs | PO-QXJGX-007 (DURABILITY_MATRIX const) | RRO-vb-qxjgx-007 |

The 7th obligation (PO-QXJGX-006, recovery summary counters) anchors to `recovery/replay/summary/apply.rs:23-34` (the apply_summary_event seam), not enumerated in the task description anchor list. The proptest file `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` is the verification artifact AND the behavior test (single harness, anti-invariant token + prop_filter close the vacuity path).

## Behavior Test Set (Independent of Verifier Harnesses)

The 6 back-compat unit tests at `crates/vb_storage/src/codec/tests.rs:1617-1791` are the canonical behavior test set for PO-QXJGX-001, PO-QXJGX-002, PO-QXJGX-004, and PO-QXJGX-005. They are **NOT** verifier harnesses — they are executable `#[test]` functions that run under `cargo test -p vb_storage` and would FAIL pre-fix and PASS post-fix:

| Test | Line | POST/INV Closure | Primary obligation |
|---|---|---|---|
| step_succeeded_event_maps_to_step_succeeded_kind | 1630 | POST-001 + POST-002 + INV-001 | PO-QXJGX-001, PO-QXJGX-002 |
| slot_written_event_maps_to_slot_written_kind_unchanged | 1650 | PRE-005 (SlotWritten wire id 12 unchanged) | PO-QXJGX-001, PO-QXJGX-002 |
| step_succeeded_and_slot_written_record_kinds_are_distinct | 1672 | INV-001 (bijection on partition) | PO-QXJGX-001, PO-QXJGX-002 |
| legacy_envelope_id_12_with_step_succeeded_payload_is_accepted | 1702 | POST-005 (back-compat: legacy envelope-12 tolerance) | PO-QXJGX-004, PO-QXJGX-005 |
| canonical_id_33_round_trip_step_succeeded | 1734 | POST-006 (canonical id-33 round-trip) | PO-QXJGX-004, PO-QXJGX-005 |
| slot_written_with_envelope_id_33_is_rejected | 1765 | POST-007 (cross-bind rejection) | PO-QXJGX-004 |

**NOTE: BACKWARD COMPATIBILITY = LEGACY ENVELOPE-12 TOLERANCE, NOT A SCHEMA BUMP.**
- `crates/vb_storage/src/constants.rs:58`: `pub const CURRENT_SCHEMA_VERSION: u16 = 1;` (verified on disk, UNCHANGED by this bead per PRE-005; pinned by proptest_durability_matrix_step_succeeded.rs:130 H2 prop_assert_eq!; in-crate tests at codec/tests.rs:3925 and 4223 enforce the pin)
- `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` (codec/tests.rs:1702-1728) directly exercises the back-compat path
- `kani_record_kind_parity_legacy_envelope.rs:H2` kani harness (line 99-121) proves the parity gate accepts envelope id 12 + StepSucceeded payload
- `kani_record_kind_decode_round_trip.rs:H2` kani harness (line 113-172) proves the decoder accepts the legacy envelope-12 + StepSucceeded pair
- `slot_written_event_maps_to_slot_written_kind_unchanged` (codec/tests.rs:1650) asserts SlotWrittenEvent still uses wire id 12 (PRE-005 invariant)
- Contract.md POST-005: parity gate accepts `StepSucceeded` payloads for envelope ids ∈ `{12, 33}`; accepts `SlotWrittenEvent` payloads for envelope id 12 only

## Trust Marker Inventory (10 TBR rows; all reviewer-accepted)

| TBR | trusted_kind | status | obligation_id(s) | reviewer_disposition |
|---|---|---|---|---|
| TBR-001 | block | blocked | PO-QXJGX-001..005 | accepted (pre-existing vb_core kani_helpers.rs unclosed delimiter; not caused by this bead) |
| TBR-002 | forward_looking | pending_formal_execution | PO-QXJGX-001..007 | accepted (4 forward-looking E0599 errors at known sites clear post-implementation) |
| TBR-003 | assume | accepted | PO-QXJGX-002, 004, 005 | accepted (kani::assume pre-conditions mirror JournalEvent::is_valid() at events.rs:514-550) |
| TBR-004 | const | accepted | PO-QXJGX-007 | accepted (CURRENT_SCHEMA_VERSION=1 pinned at constants.rs:58; UNCHANGED by this bead) |
| TBR-005 | deviation | accepted | PO-QXJGX-007 | accepted (validate_schema_version re-routed through public validate_record_kind_family surface; in-crate tests at tests.rs:3925, 4223 cover the direct call) |
| TBR-006 | deviation | accepted | PO-QXJGX-006, 007 | accepted (planned.jsonl artifact paths override task description paths) |
| TBR-007 | extern_spec | accepted | PO-QXJGX-001, 003 | accepted (hard-coded golden arrays paired with kani::assert on production function call; drift is caught by the assertion) |
| TBR-008 | model | accepted | PO-QXJGX-005-H2 | accepted (synthesized envelope mirrors pre-existing kani_record_kind.rs:107-134 pattern; needed because post-fix encoder emits id 33) |
| TBR-009 | non_vacuity | accepted | PO-QXJGX-006-H1..H4, PO-QXJGX-007-H1..H4 | accepted (invalid_input literal anti-invariant token at lines 38 + 45; explicit unit test at line 263-268) |
| TBR-010 | block | blocked | PO-QXJGX-001..005 | accepted (pre-existing kani_record_kind.rs:180-188 check_unknown_kind_rejected will FAIL post-implementation; must be deleted/updated in lockstep with the production change at validation.rs:23-25, 42-60) |

**TBR-010 is the critical routing item:** the pre-existing `check_unknown_kind_rejected` at `crates/vb_storage/src/kani_record_kind.rs:180-188` asserts `validate_kind_family(MAGIC_JOURNAL_EVENT, 33).is_err() == true`. After holzman-rust (state 11) lands the production change to admit id 33 in the journal family (per PO-QXJGX-003 / contract.md POST-001 + POST-003), this pre-existing harness will FAIL. The new `PO-QXJGX-003` harness `check_kind_33_journal_family_admit` at `kani_record_kind_journal_family_33.rs:H2` is the replacement and is correctly written. The deletion/update must happen in lockstep with the production change.

## Mapping Gaps and Routing Items

No mapping gaps. All 7 obligations have:
- Concrete `path::symbol` source refs (not file-only)
- Independent `behavior_test_refs` (6 back-compat tests + 2 proptest files; verifier harnesses do NOT count as behavior tests)
- Separate `refinement_harness_refs` (5 kani files with paired kani::cover!/kani::assert reachability witnesses; proptest rows note "the proptest file IS the verification artifact" per skill guidance for proptest-only obligations)
- Exact verifier command with `cargo kani`/`cargo test` invocation
- `evidence_workdir` set to the bead workdir
- `evidence_artifact` pointing to proof-evidence.md or the proptest source file

**Routing items deferred to State 11 (holzman-rust) implementation:**
1. records.rs:139 — add `StepSucceeded = 33` arm to the enum
2. records.rs:210 — add `Self::StepSucceeded => 33` arm to `RecordKind::id()`
3. events.rs:406 — split the OR-collapse into `Self::StepSucceeded { .. } => RecordKind::StepSucceeded` and `Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten`
4. validation.rs:24 — extend `matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | 40 | 50)` to include `33`
5. validation.rs:47 — extend `MAGIC_JOURNAL_EVENT` arm to admit `kind == 33`
6. kind_parity.rs:50 — extend `EnforceKindParity::enforce_kind_parity for JournalEvent` to accept envelope ids `{12, 33}` for StepSucceeded (LegacyEnvelopeBinding::Legacy)
7. codec/mod.rs:135 — already calls `validate_journal_event_record_kind` (UNCHANGED by this bead)
8. codec/mod.rs:143-148 — POST-013 sequence identity check (UNCHANGED by this bead)
9. durability_matrix.rs:70-204 — substitute `RecordKind::SlotWritten` → `RecordKind::StepSucceeded` in the closing position of the 10 step-closing rows (set, do, choose, for_each, parallel, collect, aggregate, repeat, wait, ask); finish row retains `RecordKind::RunFinished`
10. kani_record_kind.rs:180-188 — DELETE or UPDATE `check_unknown_kind_rejected` in lockstep with the production change (TBR-010)

## Independent Verifier Harnesses

The 5 new kani files are properly feature-gated (`kani-vb-qxjgx-record-kind-split`) and exercise production functions via canonical `crate::...` paths. They do NOT pollute the default build and do NOT pollute the `legacy-kani` feature group.

```
crates/vb_storage/Cargo.toml: adds kani-vb-qxjgx-record-kind-split feature (line 28)
crates/vb_storage/src/lib.rs:    registers each module with #[cfg(all(kani, feature = "kani-vb-qxjgx-record-kind-split"))] (lines 100-109)
```

Each kani harness body uses `#[cfg(kani)] mod harness_name { ... }` to keep the proof harness code out of the default build path.

## Verification Status (as of state 7)

- **5 Kani harnesses:** PENDING_FORMAL_EXECUTION (TBR-002 forward_looking; pre-fix production lacks `StepSucceeded` arm so the harnesses do not compile under `--cfg kani`; they compile cleanly under `cargo check -p vb_storage --features kani-vb-qxjgx-record-kind-split` without `--cfg kani`)
- **2 Proptest files:** PENDING_FORMAL_EXECUTION (TBR-002 forward_looking; 3 E0599 sites in vb_storage + 1 in vb_runtime emit the expected PENDING_FORMAL_EXECUTION signal at `cargo check --tests`)
- **6 Back-compat unit tests:** PENDING_FORMAL_EXECUTION (2 E0599 sites at codec/tests.rs:1639, 1743 emit the expected signal)
- **Production change:** NOT YET LANDED (state 11 holzman-rust owns the records.rs:139 / events.rs:406 / validation.rs / kind_parity.rs / durability_matrix.rs edits)

## Handoff Inputs for proof-reviewer

1. `proof-to-rust-map.md` (this file)
2. `rust-refinement-obligations.jsonl` (7 RRO rows: RRO-vb-qxjgx-001..007)
3. `agent-invocation-ledger.jsonl` (seq 6, this bridge invocation; ledger extended with 1 new row)
4. `trusted-base-ledger.jsonl` (UNCHANGED; TBR-001..010 cover all bridge rows; no new TBR needed)
5. `proof-review.md` (state 6 STATUS: APPROVED; 5 findings, 0 blocker)
6. `proof-findings.jsonl` (5 review findings; all minor or fixed_with_evidence)
7. `contract.md` (POST-001..013 + PRE-001..007 + INV-001..008)
8. `proof-obligations.planned.jsonl` (7 obligations; hash `59de78d1...`)
9. `proof-coverage-matrix.md` (cross-reference of contract clauses → proof seeds → lane decisions → obligations)