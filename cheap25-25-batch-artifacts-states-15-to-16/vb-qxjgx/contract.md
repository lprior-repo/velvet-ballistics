# Contract Specification: vb-qxjgx — StepSucceeded vs SlotWritten Record-Kind Split

## Context

- Feature: stop encoding `JournalEvent::StepSucceeded` as
  `RecordKind::SlotWritten`. Add a dedicated `RecordKind::StepSucceeded`
  at wire id 33. Read-side accepts legacy envelope id 12 for
  `StepSucceeded` payloads only; write-side ALWAYS emits id 33.
- Domain terms:
  - `RecordKind`: the closed enum at `crates/vb_storage/src/records.rs:139`.
    `#[repr(u16)]`, `#[non_exhaustive]`.
  - `JournalEvent`: the persistent journal variant enum at
    `crates/vb_storage/src/events.rs:32`.
  - `record_kind()`: the const fn that projects a `JournalEvent` to its
    canonical `RecordKind` (events.rs:401). Currently collapses
    `StepSucceeded` and `SlotWrittenEvent` onto the same arm.
  - Envelope / payload parity: the invariant that an envelope's
    `record_kind` matches the payload's projected `record_kind()`. The
    only exception is the new dual-bind tolerance for `StepSucceeded`.
  - `LegacyEnvelopeBinding`: typed relationship expressing whether a
    payload variant accepts a set of envelope ids (the new typed
    surface introduced by this bead).
  - `CURRENT_SCHEMA_VERSION: u16 = 1` (`crates/vb_storage/src/
    constants.rs:58`). Pinned by tests at `tests.rs:3925` and
    `tests.rs:4223`. NOT bumped by this bead.
- Assumptions:
  - Dev-stage journals with `StepSucceeded` payload + envelope id 12
    exist on disk and must continue to decode.
  - The runtime shell is the only writer of journal events; the
    `JournalEvent::record_kind()` function is the only encoder seam.
  - The schema-version pin is intentional and is NOT to be raised as a
    side effect of this bead.
  - The flux_validation module is gated off per `vb-b8i8f` (see
    `codec/mod.rs:184-186`). The literals in that file still need to
    stay in lockstep with `validation.rs`.
- Open questions:
  - **A1.** Are any non-ephemeral (production) journals present on disk
    in any test harness with `StepSucceeded` payload + envelope id 12?
    If yes, the contract's assumption is wrong and a migration must be
    added before the bead ships. Resolution: `bd show vb-qxjgx`
    cross-check before merge, plus a fixture-oracle search.
  - **A2.** Does any external migration artifact (e.g., a documentation
    reference in `docs/` or in another repo) reserve id 33? If yes,
    the chosen id must be amended. Resolution: contract search of
    `rg -n '\\b33\\b'` across `*.md` and `*.rs` in the workspace.
  - **A3.** Is the postcard envelope-bytes golden for `RecordKind`
    expected to remain stable on adding a variant? The contract does
    not assert this; verification must re-baseline
    `restate_postcard_newtype_compat_tests.rs` if any golden shifts.

## Preconditions

- PRE-001: `RecordKind` is `#[repr(u16)]` and `#[non_exhaustive]`. The
  contract does not relax this.
- PRE-002: `JournalEvent` is `#[non_exhaustive]`. The OR pattern at
  `events.rs:406` is the only allowed collapse, and it is the line
  being removed by this bead.
- PRE-003: `is_known_record_kind`, `validate_kind_family`,
  `validate_known_kind` are pure `const fn` (or `fn` for the result-
  returning variant) and admit only ids documented in the contract.
- PRE-004: `decode_journal_event` is the canonical entry point for
  parsing untrusted bytes into a `JournalEvent`. Any call site that
  bypasses it MUST be reviewed and is OUT OF SCOPE for this bead.
- PRE-005: `CURRENT_SCHEMA_VERSION = 1` is intentionally pinned. The
  contract forbids raising it as a side effect.
- PRE-006: All Holzman-Rust forbidden-construct anti-patterns (`unsafe`,
  `unwrap`, `expect`, `panic`, `dbg`, `todo`, `unimplemented`, unchecked
  indexing / slicing / casts / arithmetic) are forbidden in any new
  code path introduced by this bead.
- PRE-007: The durability matrix at `crates/vb_runtime/src/
  durability_matrix.rs:70-204` is the authoritative projection of the
  per-primitive emission contract. The contract updates it in lockstep
  with the wire change.

## Postconditions

- POST-001: `RecordKind::StepSucceeded = 33` is a public arm of the
  closed enum and `RecordKind::StepSucceeded.id() == 33`.
- POST-002: `JournalEvent::StepSucceeded` projects to
  `RecordKind::StepSucceeded` (NOT `SlotWritten`); `JournalEvent::
  SlotWrittenEvent` projects to `RecordKind::SlotWritten`.
- POST-003: `is_known_record_kind(33)` returns `true`.
- POST-004: `validate_kind_family(MAGIC_JOURNAL_EVENT, 33)` returns
  `Ok(())`; for any other magic the result is `Err(
  RecordKindFamilyMismatch)`.
- POST-005: The parity gate (the `EnforceKindParity for JournalEvent`
  impl) accepts `StepSucceeded` payloads for envelope ids ∈ `{12, 33}`;
  it accepts `SlotWrittenEvent` payloads for envelope id 12 only; it
  accepts every other payload variant for its own id only.
- POST-006: `decode_journal_event` round-trips a writer-emitted
  `StepSucceeded` event byte-for-byte using id 33; it also round-trips
  pre-fix bytes with envelope id 12 + `StepSucceeded` payload.
- POST-007: A `SlotWrittenEvent` payload paired with envelope id 33
  fails parity with `RecordKindPayloadMismatch { envelope_kind: 33,
  payload_kind: 12 }`.
- POST-008: The durability matrix at `vb_runtime/src/durability_matrix.rs`
  lists `RecordKind::StepSucceeded` in `journal_events` for every row
  that previously modeled a step-closing slot write (set/do/choose/
  for_each/parallel/collect/aggregate/repeat/wait/ask). Where the
  row genuinely emits a raw slot write (not present in the current
  matrix), `RecordKind::SlotWritten` is used and `id() == 12`.
- POST-009: Recovery summary counters (`steps_succeeded`,
  `slots_written` at `recovery/replay/summary/apply.rs:32-52`) are
  variant-keyed and unchanged in semantics; the wire-id split does not
  alter the count.
- POST-010: The Kani hard-coded valid journal kind set at
  `kani_record_kind.rs:265-289,273` and the golden array at lines
  252-255 include id 33.
- POST-011: The Flux refinement literals at `flux_validation.rs:14,33`
  include id 33 (DISABLED module; literal sync only).
- POST-012: The id→RecordKind generator arms at `proptests.rs:62,148`
  and `proptest_storage.rs:126` map id 33 to `RecordKind::StepSucceeded`;
  the selection array at `proptest_storage.rs:115` includes 33.
- POST-013: `decode_journal_event` continues to enforce the sequence
  identity check at `codec/mod.rs:143-148` and `JournalError::
  ReplayEnvelopeSequenceMismatch` continues to be the typed error.

## Invariants

- INV-001: Each `JournalEvent` variant has exactly one canonical
  `RecordKind`. The collapse at `events.rs:406` is gone.
- INV-002: There is exactly one `RecordKind` per wire id in the closed
  set `{1, 2, 3, 10..=29, 30, 31, 32, 33, 40, 50}`. Adding a variant
  MUST be paired with one new wire id from outside that set.
- INV-003: `is_known_record_kind` and `validate_kind_family` and
  `RecordKind::id()` MUST be in lockstep. They are derived from the
  same enum definition.
- INV-004: For a `StepSucceeded` payload, the parity gate accepts
  envelope ids `{12, 33}`. For a `SlotWrittenEvent` payload, the
  parity gate accepts envelope id 12 only. For every other payload
  variant, the parity gate accepts the singleton equal to that
  payload's `record_kind().id()`.
- INV-005: The writer emits the canonical id (33) for `StepSucceeded`
  and never the legacy id (12). The legacy id is read-side only.
- INV-006: `CURRENT_SCHEMA_VERSION` is `1`. The `validate_schema_version`
  function rejects `> 1` as `UnsupportedSchemaVersion` and `< 1` as
  `MigrationRequired`. Neither path is reached in normal operation.
- INV-007: Every `decode_record::<JournalEvent>` call site MUST be
  able to read both envelope ids for `StepSucceeded` without code
  change beyond the parity gate update. This bead does NOT
  introduce a configuration flag or a back-compat toggle.
- INV-008: Recovery summary counters are variant-keyed, not id-keyed.
  No consumer may switch from `match` on the variant to `match` on
  the id for counting purposes.
- INV-009: The forbidden-construct anti-patterns (`unsafe`, `unwrap`,
  `expect`, `panic`, `dbg`, `todo`, `unimplemented`, unchecked
  indexing, slicing, casts, arithmetic) are forbidden in any new code
  path introduced by this bead.
- INV-010: Behaviors documented as "out of scope" in this contract
  (schema migration, runtime support for non-ephemeral journals with
  mixed ids, compat toggle) MUST NOT be added by downstream
  implementation. If a discovery contradicts this, the contract
  amendment is the only valid response.

## Error Taxonomy

(See `error-taxonomy.md` for the full taxonomy.) The contract guarantees:

- ERR-006 (`RecordKindPayloadMismatch`) is the only error the parity
  gate emits for envelope/payload mismatch. No new variant is added.
- ERR-005 (`RecordKindFamilyMismatch`) is the only error the family
  gate emits for kind/magic drift.
- ERR-004 (`UnknownRecordKind`) is emitted for ids outside the closed
  set; id 33 is in the closed set, so it does not produce ERR-004.
- ERR-002 (`UnsupportedSchemaVersion`) and ERR-003 (`MigrationRequired`)
  are not produced by this bead; the schema is pinned at 1.
- All other errors are unchanged.

## Contract Signatures

```rust
// records.rs (NEW + CHANGED)
#[repr(u16)]
#[non_exhaustive]
pub enum RecordKind {
    // ... existing arms unchanged ...
    StepSucceeded = 33,
}

impl RecordKind {
    pub const fn id(self) -> u16 {
        match self {
            // ... existing arms unchanged ...
            Self::StepSucceeded => 33,
        }
    }
}

// events.rs (CHANGED at line 406)
pub const fn record_kind(&self) -> RecordKind {
    match self {
        // ...
        Self::StepSucceeded { .. } => RecordKind::StepSucceeded,
        Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
        // ...
    }
}

// codec/kind_parity.rs (NEW + CHANGED)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyEnvelopeBinding {
    Exact,
    Legacy { accepted_ids: &'static [u16] },
}

impl JournalEvent {
    pub const fn legacy_envelope_bindings(&self) -> LegacyEnvelopeBinding {
        match self {
            Self::StepSucceeded { .. } => LegacyEnvelopeBinding::Legacy {
                accepted_ids: &[12, 33],
            },
            _ => LegacyEnvelopeBinding::Exact,
        }
    }
}

impl EnforceKindParity for crate::JournalEvent {
    fn enforce_kind_parity(
        envelope: &RecordEnvelope,
        value: &Self,
    ) -> Result<(), JournalError>;
    // Accepts {12, 33} for StepSucceeded; exact for everything else.
    // Always calls value.is_valid() afterwards.
}

// codec/validation.rs (CHANGED)
pub(crate) const fn is_known_record_kind(kind: u16) -> bool;
pub(crate) fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError>;
// Both accept 33 in the journal family.

// codec/mod.rs (CHANGED)
pub fn validate_journal_event_record_kind(
    envelope: &RecordEnvelope,
    event: &JournalEvent,
) -> Result<(), JournalError>;
// Mirrors EnforceKindParity for the journal-event parity check.
```

## Non-Goals

- No production implementation in this artifact. The contract emits
  type-shape and signature specifications; downstream `holzman-rust`
  and `black-hat-reviewer` are responsible for the implementation.
- No proof obligations, no harness code, no Kani/Flux/Loom/proptest/
  fuzz artifacts beyond seeds in `proof-seeds.jsonl`.
- No test code or test plan.
- No schema bump. `CURRENT_SCHEMA_VERSION = 1` is intentional.
- No compat-mode flag or runtime config.
- No migration of legacy journals to canonical ids (dev-stage journals
  are ephemeral; if non-ephemeral ones are discovered, the contract is
  wrong and must be amended).
- No review approval. The contract is the contract; independent
  reviewers in proof-plan-reviewer / proof-reviewer / black-hat-reviewer
  must write their own artifacts with their own disposition.
- No change to `runtime-skill-provenance.json`, `routing-ledger.jsonl`,
  or `agent-invocation-ledger.jsonl` from this artifact.
