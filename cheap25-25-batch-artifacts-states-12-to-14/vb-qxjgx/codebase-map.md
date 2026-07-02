# Codebase Map — vb-qxjgx

Bead: vb-qxjgx — Events: stop encoding StepSucceeded as SlotWritten record kind (P1 bug)

## Summary of the bug

`JournalEvent::StepSucceeded` and `JournalEvent::SlotWrittenEvent` currently
collapse to the **same** storage discriminant `RecordKind::SlotWritten` (wire id
12) in `JournalEvent::record_kind()`. The `StepSucceeded` semantic (a step
lifecycle completion + output-slot binding) is distinct from a raw slot write,
so readers/indexers/durability-matrix consumers cannot tell them apart from the
envelope tag alone (they must inspect the postcard payload variant). The fix:
add a dedicated `RecordKind::StepSucceeded` and route `StepSucceeded` through it.

## Root-cause site (must change)

- crates/vb_storage/src/events.rs:406
  `Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten`
  → split so `StepSucceeded` maps to the new `RecordKind::StepSucceeded`.

## RecordKind definition + wire ids

- crates/vb_storage/src/records.rs:139  `pub enum RecordKind` (`#[repr(u16)]`, `#[non_exhaustive]`)
  - Journal-family ids in use: 10..=29, plus WaitResolved=31, ActionAbandoned=32.
  - Note: `StepFailed = 20` (records.rs:167) is defined but has NO `JournalEvent`
    variant mapping — precedent that a `RecordKind` can exist without a 1:1 event.
  - Next free id: **33** (Snapshot=30, Blob=40, IndexUpdate=50 are non-journal families).
- crates/vb_storage/src/records.rs:207-242  `RecordKind::id()` match — add new arm.

## Codec validation gates keyed on the id ranges (must extend for new id)

- crates/vb_storage/src/codec/validation.rs:23-25  `is_known_record_kind` = `1|2|3|10..=29|30|31|32|40|50` — add 33.
- crates/vb_storage/src/codec/validation.rs:42-60  `validate_kind_family` MAGIC_JOURNAL_EVENT = `10..=29 || WaitResolved(31) || ActionAbandoned(32)` — add 33.
- crates/vb_storage/src/codec/mod.rs:97-111  `validate_journal_event_record_kind` (envelope==payload parity).
- crates/vb_storage/src/codec/kind_parity.rs:50-64  `impl EnforceKindParity for JournalEvent` — enforces `envelope.record_kind == payload.record_kind().id()`.
- crates/vb_storage/src/codec/flux_validation.rs:14,33  Flux refinement of known/journal kind sets (module currently DISABLED — mod.rs:183-186 comments out `pub mod flux_validation`; vb-b8i8f: flux_rs not in workspace). Keep in sync anyway.

## Formal / property gates keyed on the id set (must update)

- crates/vb_storage/src/kani_record_kind.rs:265-289  `check_journal_family_exhaustive` hardcodes valid set `10..=29 | 31 | 32` — extend to include 33.
- crates/vb_storage/src/kani_record_kind.rs:273  duplicate range literal.
- crates/vb_storage/src/proptests.rs:62,148  id→RecordKind generator maps (12 => SlotWritten).
- crates/vb_storage/src/proptest_storage.rs:126  id→RecordKind generator map.

## Durability matrix (semantic model the bead is really about)

- crates/vb_runtime/src/durability_matrix.rs  encodes, per node type, the ordered
  `&[RecordKind]` a successful step emits. Step-success is currently modeled as
  `RecordKind::SlotWritten` at lines 75,100,110,120,133,147,171,186 (and the
  StepStarted+SlotWritten pairs). This is the human-facing confusion; update the
  step-completion entries to `RecordKind::StepSucceeded` where they represent a
  StepSucceeded event rather than a raw slot write.
- crates/vb_runtime/src/durability_matrix/tests.rs:50-51,63,73,84,94  asserts matrix contents — update in lock-step.

## Runtime → storage event boundary (context; likely NO change)

- crates/vb_runtime/src/journal/chunk_002.rs:81-83  maps `RuntimeJournalEvent::StepSucceeded` → `JournalEvent::StepSucceeded`. record_kind is derived downstream in vb_storage, so this file itself needs no id change, but verify the mapping still round-trips.
- Emission sites: crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:444, chunk_002.rs:70, impl_parts/chunk_001.rs:676.

## Recovery / summary (variant-keyed, NOT record_kind-keyed — should be unaffected)

- crates/vb_storage/src/recovery/replay/summary/apply.rs:32-52  StepSucceeded increments both `steps_succeeded` and `slots_written`; SlotWrittenEvent increments `slots_written`. Keyed on the enum variant, so unaffected by the record_kind split. Confirm no consumer switches on record_kind for counting.
- crates/vb_storage/src/recovery/replay/summary/accumulator.rs:114,166
- crates/vb_storage/src/recovery/replay/observation/normalize.rs:62,145
- crates/vb_storage/src/journal/incident.rs:181-207  `event_to_lifecycle` (variant-keyed) — unaffected.

## Write path (uses record_kind() for the envelope tag)

- crates/vb_storage/src/queue/writer/stage.rs:61-68  `encode_record(.., event.record_kind(), ..)`.
- crates/vb_storage/src/batch/append_event.rs:73
- crates/vb_storage/src/journal/internal.rs:67
- crates/vb_storage/src/public_api.rs:50 / test_helpers.rs:72  `append_journal_event`.

## Existing tests directly asserting the current (buggy) mapping — MUST update

- crates/vb_storage/src/codec/tests.rs:1617-1630  `step_succeeded_event_maps_to_slot_written_kind` — invert to expect `RecordKind::StepSucceeded`.
- crates/vb_storage/src/tests.rs:3318-3325 and 3361-3370  record_kind() assertions for StepSucceeded (expect SlotWritten today).
- crates/vb_storage/src/tests.rs:2076-2097  per-kind `id()` golden table — add StepSucceeded id.
- crates/vb_storage/src/tests.rs:3885-3915  distinct-id set (add new id, keep uniqueness).
- crates/vb_storage/src/record_tests.rs:48,71
- crates/workspace_tests/tests/postcard_envelope_wire_tests.rs:471-490  id→RecordKind decode table; also line 89,366 golden.
- crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:509-531  full RecordKind enumeration list.
- crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs  postcard-enum golden bytes for RecordKind (adding a variant may shift serde enum encoding — VERIFY golden bytes).
- crates/vb_runtime/src/journal/tests/chunk_002.rs:194  record_kind() assertion (WaitResolved) — sanity check pattern.

## Wire-compat / migration surfaces

- crates/vb_storage/src/constants.rs:58  `CURRENT_SCHEMA_VERSION: u16 = 1`.
- crates/vb_storage/src/tests.rs:3925  asserts `CURRENT_SCHEMA_VERSION == 1`.
- crates/vb_storage/src/tests.rs:4223  asserts MASTER doc says schema "remains 1".
- crates/vb_storage/src/codec/validation.rs:10-21  `validate_schema_version` (MigrationRequired path).

## Open questions / risks for downstream owners

1. **BACK-COMPAT (highest risk).** Old journals persisted `StepSucceeded` with
   envelope tag `SlotWritten`(12). After the split, decode re-derives payload
   kind = new id 33; `EnforceKindParity` (kind_parity.rs:52-53) and
   `validate_journal_event_record_kind` (codec/mod.rs:102-103) will reject them
   as `RecordKindPayloadMismatch`. Contract/impl owner MUST decide: (a) bump
   `CURRENT_SCHEMA_VERSION` + migration, or (b) accept legacy envelope-12 for
   StepSucceeded payloads in the parity check, or (c) declare dev-stage journals
   ephemeral (no on-disk migration). Note tests at tests.rs:3925/4223 pin schema=1,
   so option (a) also touches the MASTER doc + those tests. UNKNOWN which is intended.
2. **postcard enum wire bytes.** Adding a `RecordKind` variant may change serde
   postcard enum discriminant encoding for RecordKind if it is ever serialized as
   a bare enum (restate_postcard_newtype_compat_tests.rs). The envelope stores
   `id()` as u16 LE (independent of serde order), but confirm no path serializes
   `RecordKind` directly with postcard in a position-sensitive way.
3. **Chosen id.** Recommend `StepSucceeded = 33` (first free journal-family id).
   Confirm no external migration artifact reserves 33.
4. **Kani/Flux hardcoded ranges** (kani_record_kind.rs:273, flux_validation.rs:14,33)
   duplicate the valid-id literal set; all copies must be updated or the Kani
   family harness will produce a counterexample.

## Recommended downstream owners

- rust-contract: model StepSucceeded vs SlotWritten as distinct record kinds; decide back-compat contract (Q1).
- proof-planner / proof-writer: update kani_record_kind family harness + proptest id maps; add parity/round-trip obligation for the new kind and (if chosen) legacy tolerance.
- test-planner / test-writer: invert codec/tests.rs:1617; extend id golden tables; durability_matrix tests.
- holzman-rust (impl): records.rs enum+id(), events.rs:406 routing, validation.rs ranges, durability_matrix.rs entries, migration decision.

## Verification anchor points

- `bd show vb-qxjgx`
- rg "RecordKind::SlotWritten" crates/vb_runtime/src/durability_matrix.rs
- rg "StepSucceeded" crates/vb_storage/src/events.rs
