# Error Taxonomy — vb-qxjgx

All errors in this taxonomy are `JournalError` variants (already defined
in `crates/vb_storage/src/error.rs`). The bead does NOT introduce a new
error variant. It MAY surface existing variants under refined inputs —
specifically, `RecordKindPayloadMismatch { envelope_kind, payload_kind }`
gets new envelope/payload pairs.

## 1. Existing Variants in Scope

| Code     | Variant                                                                                | Origin                                                | Cause in this bead                                                                                                                         |
| -------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| ERR-001  | `JournalError::MagicMismatch { expected, actual }`                                     | Codec header parse                                    | Not directly caused by this bead; defensive coverage continues to require it.                                                              |
| ERR-002  | `JournalError::UnsupportedSchemaVersion { version }`                                    | `validate_schema_version`                              | Future schema bump; this bead pins schema=1 so it does not produce this variant.                                                          |
| ERR-003  | `JournalError::MigrationRequired { from, to }`                                         | `validate_schema_version`                              | Old journals with schema < 1 are NOT in scope (schema is pinned). Bead does not produce this.                                               |
| ERR-004  | `JournalError::UnknownRecordKind { kind }`                                             | `validate_known_kind`                                  | Id outside the closed set. After the fix, kinds ≥ 60 that aren't in the set still produce this.                                              |
| ERR-005  | `JournalError::RecordKindFamilyMismatch { magic, kind }`                               | `validate_kind_family`                                | Family/kind collision; after the fix, id 33 under non-journal magic also produces this.                                                    |
| ERR-006  | `JournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind }`               | Parity gate (`EnforceKindParity`)                      | NEW: post-fix combinations like `(12, SlotWritten's id)` vs `(33, SlotWrittenEvent)` and `(33, non-StepSucceeded non-12)`.               |
| ERR-007  | `JournalError::PostcardDecodeFailed(_)`                                                 | `postcard::from_bytes`                                | Unchanged.                                                                                                                                |
| ERR-008  | `JournalError::PayloadTooLarge`                                                        | Length decoder                                         | Unchanged.                                                                                                                                |
| ERR-009  | `JournalError::InvalidEvent`                                                           | `JournalEvent::is_valid()`                             | Unchanged.                                                                                                                                |
| ERR-010  | `JournalError::ReplayEnvelopeSequenceMismatch { run, envelope_seq, payload_seq }`       | `codec/mod.rs:143-148`                                 | Unchanged.                                                                                                                                |
| ERR-011  | `JournalError::WrongRun { expected, actual }`                                          | `validate_replayed_event`                              | Unchanged.                                                                                                                                |
| ERR-012  | `JournalError::SequenceGap { expected, actual }`                                        | `validate_replayed_event`                              | Unchanged.                                                                                                                                |
| ERR-013  | `JournalError::SequenceOverflow`                                                       | `next_seq`                                              | Unchanged.                                                                                                                                |

### 1.1 Post-Fix Parity Cases (ERR-006)

The following are the only `(envelope_kind, payload_kind)` pairs that
trigger `RecordKindPayloadMismatch` AFTER the fix. Every other valid
combination round-trips.

| Envelope id | Payload variant     | Outcome                                                                                |
| ----------- | ------------------- | -------------------------------------------------------------------------------------- |
| 12          | `SlotWrittenEvent`  | **Accepted** (legacy codepath; same as before).                                        |
| 33          | `StepSucceeded`     | **Accepted** (canonical).                                                              |
| 12          | `StepSucceeded`     | **Accepted** (legacy tolerance — pre-fix journals).                                    |
| 33          | `SlotWrittenEvent`  | **ERR-006** with `(envelope_kind=33, payload_kind=12)`.                                 |
| 12          | any non-`StepSucceeded` non-`SlotWrittenEvent` | **ERR-006** as before (existing behavior unchanged). |
| 33          | any non-`StepSucceeded`              | **ERR-006** with envelope_kind=33, payload_kind=variant.id().                         |
| any id > 60 or id ∉ known set | any payload | **ERR-004** before parity; never reaches ERR-006.                                       |

## 2. Errors This Bead Forbids

The bead's contract MUST NOT introduce the following:

- A new `JournalError` variant for the legacy-tolerance path. The legacy
  tolerance is silent: it accepts the bytes and produces the same
  `JournalEvent` value the writer would have produced under id 33.
- A new error variant for "writer tried to emit envelope id 12 for
  StepSucceeded". The contract forbids that emission at the type level
  (the canonical `record_kind()` returns `StepSucceeded = 33`). There is
  no runtime error; the encoder contract is captured by the type system.
- A new variant for "schema bump required". The schema is pinned at 1.
- Any error variant that bundles envelope provenance ("decoded from
  legacy id 12"). If consumers need provenance, they must derive it from
  the envelope they passed in.

## 3. Error Surface Stability

- All public `JournalError` variants are stable. The only signature
  changes ripple through `RecordKindPayloadMismatch { envelope_kind,
  payload_kind }` where post-fix inputs may take previously-unused pairs.
  This is non-breaking: the variant already takes any `u16`.
- No `From`-impl changes required by the contract.
- No `Display`, `Debug`, or `std::error::Error` behavior changes required
  by the contract.

## 4. Defensive Errors (NOT in `JournalError`)

The contract expects the following taxonomy-level defensive checks at
test/audit time, not at the codec layer:

| Defensive Error Class     | Where it surfaces                                                                  | Resolution                                                          |
| ------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Codec parity drift        | `proptests.rs`, `proptest_storage.rs` generators fail to enumerate id 33.          | Update id→RecordKind mapping tables at the indicated lines.        |
| Kani family counterexample| `kani_record_kind.rs:265-289` enumerates a u16 kind that is rejected despite being in the model. | Update `is_valid_journal_kind` predicate to include 33.            |
| Flux refinement drift     | `flux_validation.rs:14,33` refinement literals do not include id 33.                | Update literals in lockstep with `validation.rs`.                  |
| Durability matrix drift   | Matrix tests in `durability_matrix/tests.rs` reference `SlotWritten` for what is semantically a `StepSucceeded`. | Replace `RecordKind::SlotWritten` with `RecordKind::StepSucceeded` in the matrix rows that close a step. |
| Recovery summary drift    | `recovery/replay/summary/apply.rs` accidentally switches to id-keyed counting.       | The contract holds counting on the variant. Id-based counters are forbidden. |
| RecordKind wire-byte drift | `restate_postcard_newtype_compat_tests.rs` golden bytes shift because the enum grew an arm. | Investigate; the contract requires the wire bytes for RecordKind-as-newtype to remain stable (serde order may shift, MUST re-baseline). |

## 5. Caller Contract

The caller of any decode path MUST treat every variant listed in §1 as a
recoverable error: retry-after-replay is NOT applicable; the proper
response is fail-closed and emit a typed diagnostic. None of the variants
in this taxonomy may be silently discarded (`let _ = decode(...)` is a
contract violation in replay paths).

The runtime shell MAY translate these errors into opaque Fjall write
failures for closed-keyspace consumers, but the storage layer itself
never silently coerces them.
