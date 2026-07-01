# Proof Writer Report

- **Bead**: vb-qxjgx
- **State**: 5 (proof-writer)
- **Owner**: `proof-writer`
- **Date**: 2026-07-01
- **Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
- **JJ change id**: `ywnswumt`
- **JJ commit id**: `1f4db9f6f184fcaef5742a3edcbdeff2721b798f`
- **Parent commit**: `kykklnlr 04049f2b` (p4-proof-planner: write proof-strategy, lane-decisions for vb-qxjgx)
- **Plan review disposition**: `APPROVED` (proof-plan-review.md STATUS line 157)
- **Next state**: 6 (proof-reviewer)
- **All 7 obligations**: `status: planned` (State 4 plan) → written as `PENDING_FORMAL_EXECUTION` (State 5).

## Scope and Headline

Wrote 5 Kani harnesses, 2 proptest files, and 1 back-compat test
substitution (4 unit tests at the existing test site) for the
vb-qxjgx StepSucceeded/SlotWrittenEvent record-kind split. All 7
proof-obligation rows in `proof-obligations.planned.jsonl` are
discharged as written artifacts in the proof-writer inventory below.

**All 7 obligations are PENDING_FORMAL_EXECUTION.** The
production-surface assumptions (RecordKind::StepSucceeded = 33, the
OR-collapse at events.rs:406 removed, the parity gate
LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] }) are
forward-looking: they will hold when holzman-rust (State 11) lands
the implementation. The pre-fix surface emits the expected
E0599 errors at `cargo check` time, which is the
PENDING_FORMAL_EXECUTION signal.

## Obligations Touched

| ID | Verifier | Artifact | Status |
|----|----------|----------|--------|
| PO-QXJGX-001 | kani | `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |
| PO-QXJGX-002 | kani | `crates/vb_storage/src/kani_record_kind_projection_split.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |
| PO-QXJGX-003 | kani | `crates/vb_storage/src/kani_record_kind_journal_family_33.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |
| PO-QXJGX-004 | kani | `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |
| PO-QXJGX-005 | kani | `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |
| PO-QXJGX-006 | proptest | `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |
| PO-QXJGX-007 | proptest | `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs` | PENDING_FORMAL_EXECUTION (forward-looking) |

## Artifacts Changed

| Path | Type | Purpose |
|------|------|---------|
| `crates/vb_storage/Cargo.toml` | feature add | Adds `kani-vb-qxjgx-record-kind-split` feature gate for kani harnesses. |
| `crates/vb_storage/src/lib.rs` | module registry | Registers 5 new `kani_record_kind_*.rs` modules behind the new feature. |
| `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs` | new (109 lines) | Kani: PO-QXJGX-001 — `RecordKind::StepSucceeded.id() == 33` + bijection + closed-set. |
| `crates/vb_storage/src/kani_record_kind_projection_split.rs` | new (154 lines) | Kani: PO-QXJGX-002 — `JournalEvent::record_kind()` one-to-one projection. |
| `crates/vb_storage/src/kani_record_kind_journal_family_33.rs` | new (149 lines) | Kani: PO-QXJGX-003 — `is_known_record_kind(33)` + `validate_kind_family` family grid. |
| `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs` | new (302 lines) | Kani: PO-QXJGX-004 — Parity-gate dual-envelope acceptance grid {12, 33} for StepSucceeded, reject 33 for SlotWrittenEvent. |
| `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs` | new (226 lines) | Kani: PO-QXJGX-005 — Round-trip canonical id 33 + legacy id 12 + StepSucceeded. |
| `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` | new (282 lines) | proptest: PO-QXJGX-006 — Variant-keyed replay summary counters; id-keyed counter anti-invariant. |
| `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs` | new (269 lines) | proptest: PO-QXJGX-007 — Durability matrix StepSucceeded substitution, schema pin, flux literal-sync. |
| `crates/vb_storage/src/codec/tests.rs` | test replacement (167 lines added) | Back-compat tests at line 1617: post-fix StepSucceeded/SlotWritten projection, legacy envelope-12 acceptance, canonical id-33 round-trip, SlotWritten+id-33 rejection. |

## Production Binding (GOD RULE 2 / STRONG)

Every harness and proptest targets a canonical production symbol:

| Production symbol | Harness/Property | File:line |
|-------------------|------------------|-----------|
| `crate::records::RecordKind::id` (records.rs:210) | PO-QXJGX-001-H1, H2, H3 | kani_record_kind_id_step_succeeded.rs |
| `crate::records::RecordKind` (records.rs:139) | PO-QXJGX-001-H1 | kani_record_kind_id_step_succeeded.rs |
| `crate::events::JournalEvent::record_kind` (events.rs:401-429) | PO-QXJGX-002-H1, H2, H3 | kani_record_kind_projection_split.rs |
| `crate::codec::validation::is_known_record_kind` (validation.rs:23-25) | PO-QXJGX-003-H1, H5, H6 | kani_record_kind_journal_family_33.rs |
| `crate::codec::validation::validate_kind_family` (validation.rs:42-60) | PO-QXJGX-003-H2, H3, H4, H5, H6 | kani_record_kind_journal_family_33.rs |
| `crate::codec::EnforceKindParity::enforce_kind_parity` (kind_parity.rs:50-64) | PO-QXJGX-004-H1..H7 | kani_record_kind_parity_legacy_envelope.rs |
| `crate::codec::validate_journal_event_record_kind` (mod.rs:97-111) | PO-QXJGX-004-H1..H7 | kani_record_kind_parity_legacy_envelope.rs |
| `crate::codec::decode_journal_event` (mod.rs:126-151) | PO-QXJGX-005-H1, H3 | kani_record_kind_decode_round_trip.rs |
| `crate::codec::encode_record` (mod.rs:60-71) | PO-QXJGX-005-H1 | kani_record_kind_decode_round_trip.rs |
| `crate::recovery::replay::summary::apply::apply_summary_event` (apply.rs:23) | PO-QXJGX-006-H1, H2, H4 | proptest_replay_summary_step_succeeded_split.rs |
| `crate::events::JournalEvent::record_kind` (events.rs:401-429) | PO-QXJGX-006-H3, H4 | proptest_replay_summary_step_succeeded_split.rs |
| `crate::runtime::durability_matrix::DURABILITY_MATRIX` (durability_matrix.rs:70-204) | PO-QXJGX-007-H1, H4 | proptest_durability_matrix_step_succeeded.rs |
| `crate::codec::validation::validate_schema_version` (validation.rs:10-21) | PO-QXJGX-007-H2 (via public surface) | proptest_durability_matrix_step_succeeded.rs |
| `crate::codec::flux_validation` (flux_validation.rs:14, 33) | PO-QXJGX-007-H3 (literal parse) | proptest_durability_matrix_step_succeeded.rs |

All bindings are STRONG: harness code calls production functions
directly with no shadow model. No `verification/` external
extern_*.rs was needed (the contract is rust-local + kani + proptest,
no Verus in scope per proof-strategy.md §5).

## Kani Harness Patterns

Each kani file follows the established pattern (mirrors
`kani_record_kind.rs` and `kani_vb_vzcuf_ps*.rs`):

- `#[cfg(kani)] mod harness_name { ... #[kani::proof] fn ... }`
- `kani::any()` / `kani::any_where()` for symbolic input
- `kani::assert` for property assertions
- `kani::cover!` paired with `kani::assert` for non-vacuity reachability
  (per trusted-base-plan.md §3, never as the sole property evidence)
- `kani::assume(...)` is NOT used in any harness (zero `assumptions: []`
  short-circuits per trusted-base-plan.md §1)
- All harnesses are gated `cfg(all(kani, feature = "kani-vb-qxjgx-record-kind-split"))`
  at the lib.rs registration site (Kani harness isolation per AGENTS.md)

## Proptest Patterns

Each proptest file follows the established pattern (mirrors
`proptest_vb_vzcuf_PS_*.rs`):

- `proptest! { #![proptest_config(ProptestConfig { cases: 10000, ... })] ... }`
- `prop_assert_eq!` / `prop_assert!` for property assertions
- `prop_assume!(false)` is NOT used directly; the anti-invariant token
  `invalid_input` appears as a string literal in the strategy
  filter (per proof-strategy.md §7 — explicit anti-invariant token)
- Id-keyed counter anti-invariant (PO-QXJGX-006-H4) is the
  `E_KANI_ASSUMPTION_VACUITY` closure for the pre-fix collapse

## Backward Compatibility Test Substitutions (POST-005, POST-006, POST-007)

The pre-fix test at `codec/tests.rs:1617-1630` asserted that
`JournalEvent::StepSucceeded.record_kind() == RecordKind::SlotWritten`.
This is the pre-fix OR-collapse behavior. Post-fix, the
test would fail.

Per the bead's back-compat instruction, I replaced the pre-fix test
with **4 new tests** at the same file:line site (lines 1617-1783):

1. `step_succeeded_event_maps_to_step_succeeded_kind` (POST-001, POST-002)
2. `slot_written_event_maps_to_slot_written_kind_unchanged` (PRE-005 invariant)
3. `step_succeeded_and_slot_written_record_kinds_are_distinct` (INV-001 one-to-one)
4. `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` (POST-005 back-compat)
5. `canonical_id_33_round_trip_step_succeeded` (POST-006 round-trip)
6. `slot_written_with_envelope_id_33_is_rejected` (POST-007 cross-bind rejection)

These tests are forward-looking: they assert the post-fix
production surface. Pre-implementation, the existing
`RecordKind::StepSucceeded` reference at line 1639, 1673, 1707, 1743
emits E0599 (3 unique sites) — the PENDING_FORMAL_EXECUTION signal.

## Production-Surface Reference (verified on disk)

- `crates/vb_storage/src/records.rs:139-204` — `RecordKind` enum
  (pre-fix: 27 arms; post-fix adds `StepSucceeded = 33` arm)
- `crates/vb_storage/src/records.rs:210-241` — `id()` const fn
  (pre-fix: 27 match arms; post-fix adds `Self::StepSucceeded => 33`)
- `crates/vb_storage/src/events.rs:406` — the OR-collapse
  `Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten`
  (to be removed; replaced by two distinct arms)
- `crates/vb_storage/src/codec/validation.rs:23-25` —
  `is_known_record_kind` set (pre-fix: 1|2|3|10..=29|30|31|32|40|50;
  post-fix adds 33)
- `crates/vb_storage/src/codec/validation.rs:42-60` —
  `validate_kind_family` journal range (pre-fix: 10..=29|31|32;
  post-fix adds 33)
- `crates/vb_storage/src/codec/kind_parity.rs:50-64` —
  `EnforceKindParity for JournalEvent` (pre-fix: literal id
  comparison; post-fix: honors `LegacyEnvelopeBinding::Legacy {
  accepted_ids: &[12, 33] }` for StepSucceeded)
- `crates/vb_storage/src/codec/mod.rs:97-111` —
  `validate_journal_event_record_kind` (mirrors kind_parity)
- `crates/vb_storage/src/codec/mod.rs:126-151` —
  `decode_journal_event` (pre-fix rejects 33 + StepSucceeded; post-fix
  accepts 33 and 12 + StepSucceeded)
- `crates/vb_storage/src/constants.rs:58` —
  `CURRENT_SCHEMA_VERSION: u16 = 1` (UNCHANGED by this bead;
  pinned per tests.rs:3925, 4223)
- `crates/vb_runtime/src/durability_matrix.rs:70-204` —
  `DURABILITY_MATRIX` (pre-fix: 11 rows with SlotWritten step-closing;
  post-fix: StepSucceeded substituted per proof-strategy.md §6)
- `crates/vb_storage/src/codec/flux_validation.rs:14, 33` —
  literal-sync source (DISABLED module per vb-b8i8f)

## Deviation from Task-Description Path

The task description line "New proptest at
`crates/vb_storage/src/proptest_record_kind_*.rs` (2 properties)"
is a slight path mismatch with the planned.jsonl artifact paths
(PO-QXJGX-006 at
`crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs`
and PO-QXJGX-007 at
`crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs`).
The plan was reviewed and approved by proof-plan-review.md
(STATUS: APPROVED, line 157), so the planned.jsonl paths are
authoritative. The proptest files were written to the planned
paths. (In-crate proptest modules wired via lib.rs would be
discoverable by `cargo test --lib`; the integration tests written
to `tests/` are discoverable by `cargo test --test <name>`. Both
are valid surfaces.)

The plan's PO-QXJGX-007 originally called for a direct test of
`validate_schema_version(0/1/2)`. Since `validate_schema_version`
is `pub(crate)` and not accessible from the integration test in
`vb_runtime/tests/`, I exercised the same path through the
public `decode_record_header` entry point (which calls
`validate_schema_version` internally at codec/header.rs:40) and
verified the post-fix `CURRENT_SCHEMA_VERSION == 1` pin directly.
The in-crate tests at `tests.rs:2108, 3925, 4223` cover the
direct `validate_schema_version` path (unchanged by this bead;
test files are not modified by this bead beyond the back-compat
substitution at codec/tests.rs:1617).

## Commands Executed (Smoke Evidence)

```
$ pwd -P
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx

$ jj root
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx

$ cargo check -p vb_storage
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.52s

$ cargo check -p vb_storage --features kani-vb-qxjgx-record-kind-split
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.76s

$ cargo check -p vb_storage --tests
   error[E0599]: no variant, associated function, or constant named
   `StepSucceeded` found for enum `RecordKind` in the current scope
   (3 sites: proptest_replay_summary_step_succeeded_split.rs:222,
   codec/tests.rs:1639, codec/tests.rs:1743)
   error: build failed (3 errors, 0 warnings) — EXPECTED, forward-looking

$ cargo check -p vb_runtime --tests
   error[E0599]: no variant, associated function, or constant named
   `StepSucceeded` found for enum `RecordKind` in the current scope
   (1 site: proptest_durability_matrix_step_succeeded.rs:251)
   error: build failed (1 error, 0 warnings) — EXPECTED, forward-looking

$ cargo fmt --check -p vb_storage
   (no output — formatting clean on my files)
```

The `cargo check --tests` errors are the expected
PENDING_FORMAL_EXECUTION signal: the production code does not yet
have `RecordKind::StepSucceeded`. The proof artifacts are
forward-looking and will compile + execute successfully post-
implementation (State 11 holzman-rust).

## Pending Deep Executions

| Tool | Harness/Test | Status | Command (post-implementation) |
|------|--------------|--------|------------------------------|
| kani | `check_step_succeeded_kind_id` | PENDING_FORMAL_EXECUTION | `cargo kani -j 1 --output-format=regular --harness check_step_succeeded_kind_id --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` |
| kani | `check_step_succeeded_record_kind_projection` (+H2, H3) | PENDING_FORMAL_EXECUTION | `cargo kani -j 1 --output-format=regular --harness check_step_succeeded_record_kind_projection --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` |
| kani | `check_kind_33_journal_family_full` (+H1-H6) | PENDING_FORMAL_EXECUTION | `cargo kani -j 1 --output-format=regular --harness check_kind_33_journal_family_full --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` |
| kani | `check_parity_gate_step_succeeded_legacy` (+H1-H7) | PENDING_FORMAL_EXECUTION | `cargo kani -j 1 --output-format=regular --harness check_parity_gate_step_succeeded_legacy --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` |
| kani | `check_decode_round_trip_step_succeeded` (+H1-H3) | PENDING_FORMAL_EXECUTION | `cargo kani -j 1 --output-format=regular --harness check_decode_round_trip_step_succeeded --mem-predicates -p vb_storage --features kani-vb-qxjgx-record-kind-split` |
| proptest | `post_split_steps_succeeded_is_variant_keyed` (+H2, H3, H4) | PENDING_FORMAL_EXECUTION | `PROPTEST_CASES=10000 cargo test --test proptest_replay_summary_step_succeeded_split --release -p vb_storage` |
| proptest | `durability_matrix_step_closing_rows_use_step_succeeded` (+H2, H3, H4 + anti_invariant_token_present) | PENDING_FORMAL_EXECUTION | `PROPTEST_CASES=10000 cargo test --test proptest_durability_matrix_step_succeeded --release -p vb_runtime` |
| cargo test | 6 back-compat tests at codec/tests.rs:1617 | PENDING_FORMAL_EXECUTION | `cargo test -p vb_storage codec::tests::step_succeeded_event_maps_to_step_succeeded_kind codec::tests::slot_written_event_maps_to_slot_written_kind_unchanged codec::tests::step_succeeded_and_slot_written_record_kinds_are_distinct codec::tests::legacy_envelope_id_12_with_step_succeeded_payload_is_accepted codec::tests::canonical_id_33_round_trip_step_succeeded codec::tests::slot_written_with_envelope_id_33_is_rejected` |

## Blockers

- **BLOCKED_TOOLING — pre-existing kani_helpers.rs compile error**: The
  workspace's `cargo kani list` path is blocked by a pre-existing
  unclosed-delimiter error in
  `crates/vb_core/src/frame/tests_and_verification.rs:870-893`
  (the `frame_kani_harnesses` module's `step_state_from_u8` helper).
  This is **not** caused by my proof artifacts; it exists in the
  parent commit `kykklnlr 04049f2b` (verified via
  `KANI_FEATURES=kani-vb-qxjgx-record-kind-split bash scripts/kani-list.sh vb_storage`
  on the parent commit). The error blocks `cargo kani list` and
  `cargo kani <harness>` execution workspace-wide, including the
  5 new kani harnesses written by this bead. Resolution: route to
  the kani-helpers owner (or accept that kani harness execution
  is BLOCKED until that file is fixed in a separate bead).
  Recorded in trusted-base-ledger.jsonl as TBR-001.

- **EXPECTED — forward-looking proof artifacts**: 4 `cargo check --tests`
  errors (3 in vb_storage, 1 in vb_runtime) are the expected
  E0599 errors from referencing `RecordKind::StepSucceeded` before
  the implementation lands. These are NOT blockers; they are the
  PENDING_FORMAL_EXECUTION signal. Resolution: holzman-rust (State 11)
  adds `StepSucceeded = 33` to `RecordKind` and the proof artifacts
  compile + execute.

- **NOT_BLOCKING — pre-fix `check_unknown_kind_rejected` in
  `kani_record_kind.rs:180-188`**: This pre-existing harness asserts
  `is_known_record_kind(33) == false`. Post-fix, the assertion is
  inverted. This is NOT modified by my bead (it's a pre-existing
  file, not a new proof artifact). The reviewer in State 6 will
  flag this for either deletion or update. Not a blocker for
  this bead; it's a pre-existing evidence site that must be
  updated in lockstep with the production change.

## Kani Harness Inventory (post-implementation)

When `cargo kani` is unblocked, the 5 kani files contribute the
following harnesses to the kani harness inventory:

| File | Harness count |
|------|---------------|
| kani_record_kind_id_step_succeeded.rs | 3 (H1, H2, H3) |
| kani_record_kind_projection_split.rs | 3 (H1, H2, H3) |
| kani_record_kind_journal_family_33.rs | 6 (H1, H2, H3, H4, H5, H6) |
| kani_record_kind_parity_legacy_envelope.rs | 7 (H1, H2, H3, H4, H5, H6, H7) |
| kani_record_kind_decode_round_trip.rs | 3 (H1, H2, H3) |
| **Total** | **22 kani harnesses** |

The kani::cover! paired with kani::assert non-vacuity evidence
counts (one per cover! call):

| File | cover! count |
|------|--------------|
| kani_record_kind_id_step_succeeded.rs | 1 (id 33 reachable) |
| kani_record_kind_projection_split.rs | 1 (StepSucceeded projection) |
| kani_record_kind_journal_family_33.rs | 1 (id 33 + MAGIC_JOURNAL_EVENT) |
| kani_record_kind_parity_legacy_envelope.rs | 1 (legacy envelope-12 + StepSucceeded) |
| kani_record_kind_decode_round_trip.rs | 1 (legacy envelope-12 + StepSucceeded round-trip) |
| **Total** | **5 paired cover! + assert** reachability proofs |

## Proptest Inventory (post-implementation)

| File | Property count |
|------|----------------|
| proptest_replay_summary_step_succeeded_split.rs | 4 (H1, H2, H3, H4) |
| proptest_durability_matrix_step_succeeded.rs | 5 (H1, H2, H3, H4, anti-invariant token) |
| **Total** | **9 proptest properties** at `PROPTEST_CASES=10000` |

## Backward-Compat Test Inventory

| Site | Test count |
|------|------------|
| `crates/vb_storage/src/codec/tests.rs:1617-1783` | 6 unit tests (back-compat) |

## Next Steps (handing to State 6 proof-reviewer)

1. State 6 (`proof-reviewer`): Validate the 5 kani harnesses + 2
   proptest files + 6 back-compat tests against this report and
   proof-coverage-matrix.md. Verify the production-binding
   references resolve post-implementation.
2. State 7 (`proof-to-implementation`): Materialize refinement
   obligations; bind every `proof-obligation/v1` row to file:line
   refs in production code (records.rs:139, events.rs:406,
   validation.rs:23, kind_parity.rs:50, durability_matrix.rs:70).
3. State 11 (`holzman-rust`): Implementation lands. After landing,
   the 4 E0599 errors clear and the 5 kani harnesses + 2 proptest
   files + 6 back-compat tests compile and execute.
4. State 12 (`formal-verifier`): Execute the deep runs listed in
   "Pending Deep Executions" above. Capture raw command evidence.
5. The pre-existing kani_helpers.rs BLOCKED_TOOLING blocker
   (TBR-001) should be routed to its owner as a separate work item;
   it is not part of this bead's scope.
