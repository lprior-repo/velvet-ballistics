# Workflow Model — vb-qxjgx

The split of `StepSucceeded` from `SlotWritten` introduces a focused
read-side workflow with a single ambient condition (the legacy-tolerance
predicate). It is NOT a state-machine in the long-running sense; it is the
decision graph of the codec decoder when it inspects an envelope/payload
pair. We model it explicitly so that the parity predicate can be reasoned
about, fuzzed, and unit-tested without losing its branches.

## 1. Workflow: Decode a Journal Record (Read Path)

### 1.1 Actors

- **Caller** — any code that submits bytes to `decode_journal_event` (replay,
  ingestion, snapshot hydration).
- **Parser** — `decode_record::<JournalEvent>` (codec/mod.rs:80-95).
- **Parity Gate** — `EnforceKindParity for JournalEvent`
  (codec/kind_parity.rs:50-64).
- **Family Gate** — `validate_kind_family` (codec/validation.rs:42-60).
- **Well-Formedness Gate** — `JournalEvent::is_valid()` (events.rs:...) and
  `validate_replayed_event` (codec/mod.rs:160-178).

### 1.2 Initial State

```
Input: bytes &[u8], expected_magic u32, max_payload_len u32
Parser holds: (bytes, expected_magic, max_payload_len)
```

### 1.3 States (typed by `Result<(RecordEnvelope, JournalEvent),
JournalError>`)

| State ID | State                                 | Predicate                                                      |
| -------- | ------------------------------------- | -------------------------------------------------------------- |
| S0       | `Pending` (initial)                   | Bytes received but not parsed.                                 |
| S1       | `HeaderParsed`                        | Envelope decoded with magic, schema, kind, sequence.           |
| S2       | `MagicRejected`                       | Envelope magic ≠ expected_magic. Terminal: `Err(MagicMismatch)`. |
| S3       | `SchemaRejected`                      | Envelope schema version ∈ {`UnsupportedSchemaVersion`, `MigrationRequired`}. Terminal. |
| S4       | `FamilyRejected`                      | `validate_kind_family(magic, kind)` returns `Err(RecordKindFamilyMismatch)`. Terminal. |
| S5       | `UnknownKind`                         | `validate_known_kind(kind)` returns `Err(UnknownRecordKind)`. Terminal. |
| S6       | `PayloadParsed`                       | Postcard payload decoded; pre-parity object built.             |
| S7       | `ParityRejected`                      | `EnforceKindParity` returns `Err(RecordKindPayloadMismatch)`. Terminal. |
| S8       | `SequenceMismatch`                    | `envelope.sequence != event.seq().get()`. Terminal: `Err(ReplayEnvelopeSequenceMismatch)`. |
| S9       | `InvalidEvent`                        | `!event.is_valid()`. Terminal: `Err(InvalidEvent)`.            |
| S10      | `WrongRun`                            | Replay-only: `event.run_id() != expected_run`. Terminal: `Err(WrongRun)`. |
| S11      | `SequenceGap`                         | Replay-only: `event.seq() != expected_seq`. Terminal: `Err(SequenceGap)`. |
| S12      | `Accepted (terminal)`                 | All gates passed; value returned.                              |
| S13      | `DecodeFailure`                       | Postcard decoding returned `Err(PostcardDecodeFailed)`. Terminal. |
| S14      | `PayloadTooLarge`                     | Length exceeded `max_payload_len`. Terminal: `Err(PayloadTooLarge)`. |

### 1.4 Transitions

```
S0 ──decode_record──▶ S1
S1 ──magic mismatch──▶ S2 (terminal)
S1 ──schema mismatch─▶ S3 (terminal)   // CURRENT_SCHEMA_VERSION == 1 is pinned
S1 ──family mismatch─▶ S4 (terminal)   // e.g. magic=JE and kind=1
S1 ──unknown kind────▶ S5 (terminal)   // id outside the closed set
S1 ──payload too big─▶ S14 (terminal)
S1 ──postcard fail───▶ S13 (terminal)
S1 ──payload ok──────▶ S6
S6 ──parity reject───▶ S7 (terminal)   // RecordKindPayloadMismatch
S6 ──parity accept───▶ S12 (terminal) [replay path forks below]
```

The replay path is layered on top of S12:

```
S12 (initial replay) ──run mismatch──▶ S10 (terminal)
S12 (initial replay) ──seq gap───────▶ S11 (terminal)
S12 (initial replay) ──all pass──────▶ S12 (terminal: Accepted)
```

### 1.5 Parity Gate Sub-Workflow (S6 → S7 | S12)

This is the only branching surface changed by vb-qxjgx.

```text
Receive (envelope_kind, payload_kind).

If payload is StepSucceeded variant:
    accept iff envelope_kind ∈ {12, 33}.
Else:
    accept iff envelope_kind == payload_kind.

Also accept iff value.is_valid() [sic — is_valid is checked after].

If accept:
    return S12.
Else:
    return S7 with envelope_kind, payload_kind intact.
```

- Idempotent: parsing the same bytes twice produces the same outcome.
- Determinism gate: no time, no I/O, no randomness, no thread state. The
  codec MUST be a pure function of bytes.
- The legacy tolerance is bounded: ONLY `StepSucceeded` accepts a
  multi-id set. Every other variant enforces exact parity.

### 1.6 Guards

| Guard                  | Where                                       | What it forbids                                                                |
| ---------------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| `magic == expected`    | `decode_record_payload`                     | Records misrouted across storage families.                                     |
| `kind ∈ known set`     | `validate_known_kind`                       | Out-of-range ids.                                                              |
| `kind ∈ family(magic)` | `validate_kind_family`                      | Family/kind collision.                                                         |
| `envelope ↔ payload`   | `EnforceKindParity`                         | Mismatched envelope vs payload discriminant.                                   |
| `envelope.seq == payload.seq` | `codec/mod.rs:143-148`                | Forged or corrupted envelope — protects replay identity.                      |
| `event.is_valid()`     | events.rs:..., kind_parity.rs:59            | Records with structurally impossible field values (`run_id == 0`, etc.).     |
| (replay) `event.run_id == expected` | `validate_replayed_event`         | Cross-run replay injection.                                                    |
| (replay) `event.seq == expected`   | `validate_replayed_event`         | Replay ordinal contiguity.                                                     |

### 1.7 Terminal Outcomes

| Outcome                          | Variant                                                        | Meaning                            |
| -------------------------------- | -------------------------------------------------------------- | ---------------------------------- |
| Accepted (terminal)               | `Ok((RecordEnvelope, JournalEvent))`                          | Record is well-formed and consistent. |
| Rejected — family mismatch        | `Err(JournalError::RecordKindFamilyMismatch { magic, kind })` | Record wandered across families.   |
| Rejected — unknown kind           | `Err(JournalError::UnknownRecordKind { kind })`               | Future id; not in the closed set.  |
| Rejected — parity mismatch        | `Err(JournalError::RecordKindPayloadMismatch { envelope_kind, payload_kind })` | Cross-variant or non-legacy mismatch. |
| Rejected — decode failure         | `Err(JournalError::PostcardDecodeFailed(_))`                  | Bytes corrupt or truncated.        |
| Rejected — payload too large      | `Err(JournalError::PayloadTooLarge)`                           | Length bound exceeded.             |
| Rejected — invalid event          | `Err(JournalError::InvalidEvent)`                              | `is_valid()` returned `false`.     |
| Rejected — migration required     | `Err(JournalError::MigrationRequired { from, to })`            | Schema below `CURRENT_SCHEMA_VERSION` (this bead is NOT in this path because schema is pinned at 1). |
| Rejected — unsupported schema     | `Err(JournalError::UnsupportedSchemaVersion { version })`       | Schema above `CURRENT_SCHEMA_VERSION`. |
| Rejected — magic mismatch         | `Err(JournalError::MagicMismatch { expected, actual })`         | Record routed to wrong decoder.   |
| Rejected — replay envelope seq    | `Err(JournalError::ReplayEnvelopeSequenceMismatch { run, envelope_seq, payload_seq })` | Forged envelope. |
| Rejected — wrong run              | `Err(JournalError::WrongRun { expected, actual })`              | Cross-run replay injection.       |
| Rejected — sequence gap           | `Err(JournalError::SequenceGap { expected, actual })`           | Replay contiguity violation.      |

## 2. Workflow: Encode + Append a Journal Event (Write Path)

The write path is unchanged in branching; its only effect of this bead is
that the `record_kind().id()` value for `StepSucceeded` is **33** instead
of **12**.

```text
Receive event: JournalEvent.

Compute kind = event.record_kind()              // RecordKind arm
Compute id   = kind.id()                       // u16 (33 for StepSucceeded)
Compute envelope = RecordEnvelope { magic, schema=1, record_kind=id, sequence }
Encode postcard payload.
Submit (envelope, payload) to the Fjall batch appender.
```

- Determinism: same event in → same `(envelope, payload)` out, byte-for-byte.
- No legacy envelope tag (id 12) is ever emitted by the writer for
  `StepSucceeded`. (This is the forward-only policy; the dual-bind
  tolerance is read-side only.)

## 3. Workflow: Recovery Summary (projection, no model change)

`recovery/replay/summary/apply.rs:32-52` increments:

- `steps_succeeded` for the `StepSucceeded` variant.
- `slots_written` for `StepSucceeded` (carries output slot) and for the
  `SlotWrittenEvent` variant (raw write).

This is variant-keyed and is unaffected by the id split. The contract's
hazard here is "an aggregator MUST NOT switch to id-keyed counting"; see
`hazard-analysis.md` H6.

## 4. Workflow: Schema Migration (NOT IN SCOPE)

A schema migration workflow is OUT OF SCOPE for vb-qxjgx. The contract
explicitly forbids adding one. The justification:

- `CURRENT_SCHEMA_VERSION = 1` is pinned at both source-level
  (`constants.rs:58`) and at multiple test-golden assertions
  (`tests.rs:3925, 4223`).
- The bead tasking explicitly forbids bumping the schema version.
- The legacy tolerance is bounded and is read-side only.

If a non-ephemeral journal with mixed StepSucceeded/SlotWritten envelopes
is later discovered, the workflow model MUST be amended before the fix can
ship. This contract DOES NOT anticipate that discovery.
