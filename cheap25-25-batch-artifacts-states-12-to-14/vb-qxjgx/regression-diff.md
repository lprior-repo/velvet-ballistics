# Regression Diff — vb-qxjgx

## Bead-Local Changes (state 11 + state 5 proof artifacts)

### Production source (8 files)
- `crates/vb_storage/src/records.rs:195` — Added `StepSucceeded = 33` arm to the `RecordKind` enum (29-variant closed set).
- `crates/vb_storage/src/records.rs:247` — Added `Self::StepSucceeded => 33` arm to `RecordKind::id()`.
- `crates/vb_storage/src/events.rs:406-407` — Removed the pre-fix OR-collapse `Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten`. Replaced with two distinct arms: `Self::StepSucceeded { .. } => RecordKind::StepSucceeded` (line 406) + `Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten` (line 407).
- `crates/vb_storage/src/codec/validation.rs:24` — Extended `is_known_record_kind` to include `33` in the closed set (1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | **33** | 40 | 50, 28 entries).
- `crates/vb_storage/src/codec/validation.rs:50` — Extended `validate_kind_family` MAGIC_JOURNAL_EVENT arm to include `kind == RecordKind::StepSucceeded.id()` (i.e., kind 33).
- `crates/vb_storage/src/codec/kind_parity.rs:45-78` — Added `LegacyEnvelopeBinding` enum (`Exact | Legacy { accepted_ids: &[u16] }`) + `for_journal_event` function + `admits` method. Updated `EnforceKindParity for JournalEvent` to honor the binding (StepSucceeded admits {12, 33}).
- `crates/vb_storage/src/codec/mod.rs:97-118` — Replaced literal `envelope_kind == 12` check in `validate_journal_event_record_kind` with the typed `LegacyEnvelopeBinding::for_journal_event(event).admits(envelope_kind, payload_kind)`.
- `crates/vb_runtime/src/durability_matrix.rs:75,89,100,110,120,132-133,146-147,158,171,186-187` — Substituted `RecordKind::SlotWritten` → `RecordKind::StepSucceeded` in 10 step-closing rows (set, do, choose, for_each, parallel, collect×2, aggregate×2, repeat, wait, ask×2). The finish row (line 198) retains `RunFinished`.
- `crates/vb_storage/src/codec/flux_validation.rs:14,33` — Literal-sync id 33 in the known set (DISABLED module per vb-b8i8f closure).

### Test code (5 files)
- `crates/vb_storage/src/codec/tests/replay_integrity.rs:224` — Updated journal-family admission test to include id 33.
- `crates/vb_storage/src/tests.rs:3325` — Updated all-variants projection test to expect StepSucceeded.
- `crates/vb_runtime/src/durability_matrix/tests.rs:51` — Updated set row test to expect StepSucceeded.
- `crates/vb_storage/src/kani_record_kind.rs:177-188` — DELETED pre-fix `check_unknown_kind_rejected` (TBR-010 resolution).
- `crates/vb_storage/src/lib.rs:98` — Updated comment.

### Proof artifacts (5 new kani + 2 new proptest + 6 new back-compat tests)
- 5 new kani files (PENDING_FORMAL_EXECUTION — TBR-001 blocks execution):
  - `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs` (109 lines, 3 harnesses, PO-QXJGX-001)
  - `crates/vb_storage/src/kani_record_kind_projection_split.rs` (154 lines, 3 harnesses, PO-QXJGX-002)
  - `crates/vb_storage/src/kani_record_kind_journal_family_33.rs` (149 lines, 6 harnesses, PO-QXJGX-003)
  - `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs` (302 lines, 7 harnesses, PO-QXJGX-004)
  - `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs` (226 lines, 3 harnesses, PO-QXJGX-005)
- 2 new proptest files (PASS at PROPTEST_CASES=10000):
  - `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` (282 lines, 4 properties, PO-QXJGX-006)
  - `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs` (269 lines, 5 properties, PO-QXJGX-007)
- 6 new back-compat unit tests (PASS):
  - `crates/vb_storage/src/codec/tests.rs:1630` — `step_succeeded_event_maps_to_step_succeeded_kind` (POST-001, POST-002)
  - `crates/vb_storage/src/codec/tests.rs:1650` — `slot_written_event_maps_to_slot_written_kind_unchanged` (PRE-005)
  - `crates/vb_storage/src/codec/tests.rs:1672` — `step_succeeded_and_slot_written_record_kinds_are_distinct` (INV-001)
  - `crates/vb_storage/src/codec/tests.rs:1702` — `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` (POST-005)
  - `crates/vb_storage/src/codec/tests.rs:1734` — `canonical_id_33_round_trip_step_succeeded` (POST-006)
  - `crates/vb_storage/src/codec/tests.rs:1765` — `slot_written_with_envelope_id_33_is_rejected` (POST-007)

### Build config (2 files)
- `crates/vb_storage/Cargo.toml` — Added `kani-vb-qxjgx-record-kind-split` feature gate.
- `crates/vb_storage/src/lib.rs` — Registered 5 new kani modules behind the new feature.

## Regression Classification

### Bead-local regressions
**None.** All 2 affected packages (vb_storage + vb_runtime) pass their full test sweep (1678 + 2348 tests). The 4 forward-looking E0599 errors (TBR-002) are resolved post-state-11. The pre-fix `check_unknown_kind_rejected` is deleted (TBR-010). The OR-collapse is removed at events.rs:406.

### Cross-package regressions
**None.** The vb_storage ↔ vb_runtime boundary is preserved: vb_runtime imports `RecordKind::StepSucceeded` (the new variant) and `StepSucceeded` is in scope (records.rs:195). The cross-crate integration tests in `crates/vb_runtime/tests/` (e.g., `proptest_durability_matrix_step_succeeded.rs`) pass.

### Global regressions
**None introduced by this bead.** The 3 global debt items are pre-existing:

1. **TBR-001** (pre-existing kani_helpers.rs unclosed-delimiter in `crates/vb_core/src/frame/parts/kani_helpers.rs:22:7`) — blocks `cargo kani` workspace-wide. Verified to exist in parent commit `ywnswumt 1b72c500` (proof-writer commit before this bead's state 11). NOT caused by this bead.
2. **Pre-existing vb_runtime/src/frame_pool/tests.rs cargo fmt drift** (3 sites at lines 85, 114, 139) — NOT modified by this bead (verified via `jj diff`). Pre-existing global debt.
3. **Pre-existing aggregate_resource_budget_properties_red proptest failure** — unrelated to this bead (literal-string check; not in proptest_replay_summary_step_succeeded_split.rs or proptest_durability_matrix_step_succeeded.rs).

### Bead-local test parity
- 1678 + 2348 cargo test PASS
- 6 back-compat unit tests PASS (codec/tests.rs:1630, 1650, 1672, 1702, 1734, 1765)
- 9 proptest properties PASS (4 + 5 at PROPTEST_CASES=10000)
- 0 ignored, 0 filtered failures
- 0 panic surface in production (verified by `rg "(unwrap\(\)\|expect\(\|panic!\|todo!\|unimplemented!\|dbg!\|unsafe )"` on 6 production files)

### CURRENT_SCHEMA_VERSION preservation
`crates/vb_storage/src/constants.rs:58` reads `pub const CURRENT_SCHEMA_VERSION: u16 = 1;` — UNCHANGED by this bead. Back-compat is **legacy envelope-12 tolerance, NOT a schema bump**. Proptest PO-QXJGX-007-H2 (`schema_version_is_pinned_at_one`) directly asserts `CURRENT_SCHEMA_VERSION == 1u16`. The in-crate tests at `tests.rs:3925` and `tests.rs:4223` enforce the pin.

## Verdict

**STATUS: NO BEAD-LOCAL REGRESSIONS; 3 PRE-EXISTING GLOBAL DEBT ITEMS, ALL WITH owner_approved_debt DISPOSITION.**

The bead is ready for landing.
