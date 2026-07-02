# Domain Model — vb-qxjgx

> Split `StepSucceeded` from `SlotWritten` in the durable journal envelope-tag
> dimension. Stop collapsing two semantically distinct events onto one wire
> discriminant.

## Ubiquitous Language

The terms below are the contract vocabulary used throughout every other
artifact in this bead. They are the only terms downstream proof, test, and
implementation agents may introduce without an explicit amendment to this
list. Reuse them verbatim.

### Entities

| Term                         | Definition                                                                                                                                                                          |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Journal Record**           | A persistable storage record written by the runtime into a per-run keyspace. Has a 60-byte envelope (`RecordEnvelope`) plus a postcard payload. Identified by `(magic, record_kind)`. |
| **Record Kind**              | The wire id (`u16`) that tags a record's envelope and asserts its family. `RecordKind` is the closed Rust enum at `crates/vb_storage/src/records.rs:139`.                          |
| **Journal Event**            | A semantically meaningful state transition emitted by the runtime into the journal. Modeled by `JournalEvent` at `crates/vb_storage/src/events.rs:32`.                             |
| **Step Lifecycle Event**     | A `JournalEvent` that marks the boundary of a single step attempt (started, succeeded, failed). Distinguished from slot-binding events by intent.                                   |
| **Slot Binding Event**       | A `JournalEvent` that marks a write to a workflow output slot (`SlotWrittenEvent`). Distinct from step-completion events.                                                          |
| **Envelope / Payload**       | Pairing of `(RecordEnvelope, JournalEvent)`. The envelope stores `record_kind`; the payload's own `record_kind()` derives the expected id. They MUST match (with the legacy exception for StepSucceeded). |

### Value Objects

| Term                              | Definition                                                                                                                       |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **`RecordKind::StepSucceeded`**   | New kind id **33** (`u16`). One of the journal family (`MAGIC_JOURNAL_EVENT`). Distinguishes step completion from a raw slot write.|
| **`RecordKind::SlotWritten`**     | Existing kind id **12** (`u16`). Unchanged. Carries `SlotWrittenEvent` payloads only.                                            |
| **Wire id 12 (legacy envelope)**  | The previously shared id for both `StepSucceeded` and `SlotWrittenEvent`. Persisted in dev-stage journals. Read-side ONLY.        |
| **Wire id 33 (new envelope)**     | The split id for `StepSucceeded`. The ONLY id the writer emits for `StepSucceeded` after the fix.                               |
| **Accepted envelope ids for StepSucceeded payload** | The set `{12, 33}`. Any other id is rejected as `RecordKindPayloadMismatch`.                                                |
| **Accepted envelope id for SlotWrittenEvent payload** | The singleton `{12}`.                                                                                                       |
| **Accepted envelope id for any other payload variant** | The singleton equal to that payload's own `record_kind().id()`.                                                              |
| **Journal kind family range**     | `10..=29 ∪ {31, 32, 33}` — what `validate_kind_family(MAGIC_JOURNAL_EVENT, _)` accepts.                                          |
| **Known record-kind set**         | `{1, 2, 3} ∪ {10..=29} ∪ {30, 31, 32, 33, 40, 50}` — what `is_known_record_kind` accepts.                                       |
| **`CURRENT_SCHEMA_VERSION`**     | Pinned at `1` (`crates/vb_storage/src/constants.rs:58`). The schema is NOT bumped by this bead.                                  |
| **Durability Matrix Row**         | A per-primitive declaration of its emitted journal kinds, ack point, and replay assertion (`crates/vb_runtime/src/durability_matrix.rs`). |

### Commands and Operations

| Term                          | Definition                                                                                                                                |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **Encode + Append**           | Write path: select id via `event.record_kind().id()` → stamp envelope → append to Fjall batch. Writes ALWAYS use the new id (33).          |
| **Decode + Validate**         | Read path: parse envelope → parse postcard payload → enforce parity + is_valid. Reads accept legacy id (12) for `StepSucceeded` payloads. |
| **Replay (Recovery)**         | Reading a journal from disk or a streaming input to rebuild run state. Read-side parity is the gate.                                       |
| **Replay Summary**            | A pure aggregation of step-success and slot-write counts keyed on `JournalEvent` variants (`recovery/replay/summary/apply.rs:32-52`).      |
| **Schema Migration**          | Translating a legacy envelope into the new id model. THIS BEAD DOES NOT INTRODUCE A MIGRATION. Legacy tolerance is the read-side path.   |

### Policies (Invariants)

1. **INV-1 (Distinguishability).** After decode, observers MUST be able to
   determine whether a record represents a step completion or a slot binding
   from the envelope tag alone. This is the bug the bead fixes.
2. **INV-2 (Forward-only writer).** All NEW journal writes emitted for a
   `StepSucceeded` MUST encode envelope id **33**. Encoders MUST NOT emit id
   **12** for `StepSucceeded` (it is reserved, with the runtime logic at the
   decoder end, for compatibility with already-persisted records).
3. **INV-3 (Pinned schema).** `CURRENT_SCHEMA_VERSION` remains **1**. The
   `validate_schema_version` function rejects higher/lower versions per
   `JournalError::MigrationRequired | UnsupportedSchemaVersion`. The fix is a
   compat policy inside the codec, NOT a schema bump.
4. **INV-4 (Closed kind set).** `RecordKind` is `#[non_exhaustive]` but the
   public id set is treated as closed. Any new id requires a paired decision
   on (a) admission into `is_known_record_kind`, (b) family admission into
   `validate_kind_family(MAGIC_JOURNAL_EVENT, _)`, (c) a `record_kind()` arm
   in `JournalEvent`, and (d) updates to the Kani/Flux/proptest generators.
5. **INV-5 (Variant-keyed aggregation).** Recovery summary counters increment
   keyed on the `JournalEvent` variant, not the wire id. The id split MUST
   NOT change the count semantics of `steps_succeeded` or `slots_written`
   (see `recovery/replay/summary/apply.rs:32-52`).
6. **INV-6 (Back-compat tolerance is bounded).** Dual acceptance of `{12, 33}`
   applies ONLY to the `StepSucceeded` payload. `SlotWrittenEvent` accepts id
   12 only. No other variant enjoys a legacy envelope exception.

## Forbidden States (Illegal If Representable)

A well-typed domain makes the following states unrepresentable. They are
listed here as the invariants downstream code MUST enforce.

| State                                                                  | Why it is illegal                                                                           |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `RecordKind { id: 12 }` paired with payload variant `StepSucceeded` on a NEW write | INV-2; forward-only writer must emit 33. Idempotent encode must reject this.               |
| `RecordKind { id: 33 }` paired with payload variant `SlotWrittenEvent` | INV-1; 33 is exclusively the StepSucceeded kind. Mixed pairing triggers parity mismatch.    |
| `RecordKind { id: n }` with `n ∉ {1, 2, 3, 10..=29, 30, 31, 32, 33, 40, 50}` | INV-4; out-of-family id. `is_known_record_kind` returns `false`.                          |
| `RecordKind { id: 33 }` under magic ≠ `MAGIC_JOURNAL_EVENT`            | INV-4; family mismatch. `validate_kind_family` returns `Err`.                              |
| A `JournalEvent` whose `record_kind()` is not its declared wire id    | Type-level invariant; the `record_kind()` function is the canonical authority.             |
| `record_kind()` returning `StepSucceeded` for any non-`StepSucceeded` variant | Type-level invariant; each variant has exactly one matching `RecordKind`.              |
| Open-enum addition without paired updates at all five surface files  | Breaks INV-4; surfaces: `records.rs`, `events.rs:406`, `validation.rs`, `kind_parity.rs`, plus each proptest/kani/flux generator. |

## Open Domain Questions

These are decisions the contract holds OPEN until downstream owners confirm:

1. **Do dev-stage journals exist on disk and need a one-shot rewriter?**
   The contract assumes dual-tag tolerance is sufficient and that legacy
   journals are dev-stage-ephemeral. If a non-ephemeral journal with mixed
   StepSucceeded/SlotWritten envelopes is discovered, a migration MUST be
   added and `CURRENT_SCHEMA_VERSION` MUST be reconsidered (this requires
   the bead's contract to be amended — not silently broadened).
2. **Should a `StepSucceededDecoder` distinguish "decoded from legacy id"
   from "decoded from canonical id"?** The contract keeps parity permissive
   but does not surface provenance. If durability tooling needs the
   distinction, expose a typed `source_envelope_kind` accessor on a parser
   helper, or write the source kind into the parquet-like projection
   downstream. Deferred to proof-writer.
3. **Is id 33 free of legacy reservation by external migration
   artifacts?** Recommend `bd show vb-qxjgx` cross-check before any
   implementation PR is merged. If 33 is reserved by an external migration,
   the contract must be amended.
