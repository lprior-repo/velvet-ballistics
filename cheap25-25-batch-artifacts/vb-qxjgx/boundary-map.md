# Boundary Map — vb-qxjgx

This artifact maps the surface of vb-qxjgx against the
functional-core / imperative-shell split and against each cross-cutting
boundary (storage, network, time, FFI, unsafe, parser/codec,
generated/handwritten). Everything in this bead sits at the parser/codec
boundary on the read side and at the storage boundary on the write side,
with pure logic in between.

## 1. High-Level Topology

```
                            ┌───────────────────────────────────────┐
   untrusted bytes  ──────▶ │ codec::decode_journal_event            │
                            │   (imperative shell — FFI/IO boundary)│
                            │   parses envelope + postcard payload  │
                            │                                       │
                            │   ──── calls into pure core ────────  │
                            │   1. validate_kind_family             │
                            │   2. validate_known_kind              │
                            │   3. EnforceKindParity::enforce_kind  │
                            │   4. JournalEvent::is_valid           │
                            │   5. validate_journal_event_record    │
                            │                                       │
                            │   ──── returns typed result ─────     │
                            └─────┬─────────────────────────────────┘
                                  ▼
                            ( RecordEnvelope, JournalEvent )
                                  │
                                  ▼
                            ┌───────────────────────────────────────┐
                            │ replay summary / observation          │
                            │ (pure core: variant-keyed counters)   │
                            │                                        │
                            │  apply.rs:32-52, accumulator.rs:114   │
                            │  normalize.rs:62,145                  │
                            └───────────────────────────────────────┘
```

```
                            ┌───────────────────────────────────────┐
   event: JournalEvent ───▶ │ append path                            │
                            │   compute record_kind via              │
                            │     event.record_kind().id()  (pure)   │
                            │   build RecordEnvelope (pure)          │
                            │   encode postcard payload (pure)       │
                            │                                        │
                            │   ──── imperative shell ───────        │
                            │   stage.rs:61-68, append_event.rs:73  │
                            │   internal.rs:67, public_api.rs:50    │
                            │   writes to Fjall keyspace (IO)        │
                            └───────────────────────────────────────┘
```

## 2. Boundaries and Their Handling

### 2.1 Parser Boundary (Untrusted Input)

The parser boundary is the outer wall of the codec. It accepts bytes from
disk, network, snapshot inputs, and test fixtures. The boundary function
`decode_journal_event` enforces the full pipeline:

1. `decode_record_payload` parses the 60-byte envelope into
   `RecordEnvelope` (header parse, magic check).
2. `validate_kind_family(Je_magic, kind)` rejects family drift.
3. `validate_known_kind(kind)` rejects unknown ids.
4. `postcard::from_bytes(payload)` parses the variable-sized body.
5. `T::enforce_kind_parity(&envelope, &value)` verifies the parity
   predicate (this bead's central change).
6. `validate_journal_event_record_kind` re-checks parity
   (defense-in-depth).
7. `JournalEvent::is_valid()` verifies domain validity.
8. `envelope.sequence == event.seq().get()` guards against envelope
   forgery.
9. Replay-only: `validate_replayed_event(run, expected_seq, event)`
   verifies run identity and ordinal contiguity.

At every step the boundary returns a typed `Result<(RecordEnvelope,
JournalEvent), JournalError>`. The internal pipeline does not panic, does
not `unwrap`, and does not tolerate truncated or malformed bytes. The
contract explicitly forbids `unwrap`/`expect`/`panic` even at the
boundary.

### 2.2 Storage Boundary (Durable Persistence)

The durable persistence boundary is the Fjall keyspace writer. The
writer-side call chain is:

```
caller
  └─▶ append_journal_event (public_api.rs:50 or test_helpers.rs:72)
        └─▶ append_event (batch/append_event.rs:73)
              └─▶ encode_record(.., event.record_kind(), ..)
                                          (queue/writer/stage.rs:61-68)
                    └─▶ internal.rs:67 sends to Fjall batch
```

The contract restricts the encoder to emit ids derived from
`JournalEvent::record_kind()`. There is no override seam. There is no
separate "compat mode" flag — once the type-level split is live, the
encoder naturally emits id 33 for `StepSucceeded` and id 12 for
`SlotWrittenEvent`.

### 2.3 Pure Core (No I/O, No Time, No Network)

These are the surfaces the contract demands be pure:

- `RecordKind::id() const fn`
- `JournalEvent::record_kind() const fn`
- `JournalEvent::legacy_envelope_bindings() const fn`
- `LegacyEnvelopeBinding` enum (no fields that require construction;
  only used in `match`)
- `validate_kind_family`, `validate_known_kind`, `is_known_record_kind`
  (all `const fn`)
- `EnforceKindParity::enforce_kind_parity` (the override on `JournalEvent`)
- `validate_journal_event_record_kind`
- The id→RecordKind generators in `proptests.rs`, `proptest_storage.rs`

These are at the pure-core layer: no I/O, no time, no FFI, no
randomness, no thread state.

### 2.4 Imperative Shell (I/O, Time, FFI)

- `decode_journal_event` (orchestration only; logic delegated to pure
  core).
- `encode_record` (orchestration only; pure-core encoded by
  `JournalEvent::record_kind`).
- The Fjall batch call paths.

### 2.5 Schema/Configuration Boundary

`CURRENT_SCHEMA_VERSION: u16` is a `const` in `crates/vb_storage/src/
constants.rs:58`. It is read at envelope validation time and at the
`RecordEnvelope::schema_version` round-trip. The contract forbids mutating
this constant; doing so would break the `tests.rs:3925` and
`tests.rs:4223` golden pins.

### 2.6 Cross-Cutting Boundaries Not Affected

| Boundary           | Why unaffected                                                                                       |
| ------------------ | ---------------------------------------------------------------------------------------------------- |
| Network            | No change. Journal events are not transmitted over the wire as part of this bead.                    |
| Time               | No change. The journal does not stamp wall-clock; the encoder does not introduce new time calls.    |
| FFI                | No change. The parity gate runs in pure Rust.                                                        |
| Unsafe             | This bead does not introduce unsafe; downstream implementation MUST NOT introduce unsafe.             |
| Async/runtime      | No change. The decoder is a sync `fn`, no `await` introduced.                                        |
| Concurrency        | No change. The decoder carries no `Send`/`Sync` or atomic state.                                     |
| Generated code     | No change. No maxperf-generated path encodes `RecordKind` via `compile-time` codegen.                |
| Perf paths         | No change. `crates/*/src/perf/**` is untouched.                                                      |

### 2.7 Boundary for the Back-Compat Lane

The back-compat lane is a read-side-only predicate, encapsulated in:

- `JournalEvent::legacy_envelope_bindings() const fn`
- The `EnforceKindParity for JournalEvent` override
- The `validate_journal_event_record_kind` parity branch

It does NOT introduce a new boundary (no new I/O path, no new thread, no
new network surface). It does NOT introduce a configuration flag (no
`bool`, no `enum mode { Strict, Compat }`). The dual-bind is part of the
canonical contract.

## 3. Crate / Module Topography

| Crate               | Module / path                                       | Surface         | Owner State |
| ------------------- | --------------------------------------------------- | --------------- | ----------- |
| `vb_storage`        | `src/records.rs:139`                                | `RecordKind` enum + new `StepSucceeded = 33` arm. | Primary     |
| `vb_storage`        | `src/records.rs:207-242`                            | `RecordKind::id()` arm.                          | Primary     |
| `vb_storage`        | `src/events.rs:406`                                 | `JournalEvent::record_kind()` split.             | Primary     |
| `vb_storage`        | `src/codec/validation.rs:23-25`                     | `is_known_record_kind` accepts 33.               | Primary     |
| `vb_storage`        | `src/codec/validation.rs:42-60`                     | `validate_kind_family(Je, _)` admits 33.         | Primary     |
| `vb_storage`        | `src/codec/kind_parity.rs:50-64`                    | `EnforceKindParity for JournalEvent` legacy bind. | Primary     |
| `vb_storage`        | `src/codec/mod.rs:97-111`                           | `validate_journal_event_record_kind` legacy bind.| Primary     |
| `vb_storage`        | `src/codec/flux_validation.rs:14,33`                | Flux refinements (DISABLED — vb-b8i8f; update literals anyway). | Secondary |
| `vb_storage`        | `src/kani_record_kind.rs:265-289,273`               | Kani exhaustive harness predicate extended to 33. | Primary     |
| `vb_storage`        | `src/kani_record_kind.rs:252-255`                   | Golden known-kinds array includes 33.            | Primary     |
| `vb_storage`        | `src/proptests.rs:62,148`                           | id→RecordKind generator includes 33.            | Secondary   |
| `vb_storage`        | `src/proptest_storage.rs:115,126`                   | id→RecordKind generator and selection include 33.| Secondary   |
| `vb_runtime`        | `src/durability_matrix.rs` (rows for set/do/choose/for_each/parallel/collect/aggregate/repeat/wait/ask) | Replace `SlotWritten` with `StepSucceeded` in `journal_events` for step-closing rows. | Primary |
| `vb_runtime`        | `src/durability_matrix/tests.rs:50-51,63,73,84,94`   | Update assertions for `set/do/wait/ask/finish`.  | Test        |
| `vb_runtime`        | `src/journal/chunk_002.rs:81-83`                    | `RuntimeJournalEvent::StepSucceeded → JournalEvent::StepSucceeded`. Round-trip MUST be verified. | Review only |
| `vb_storage`        | `src/codec/tests.rs:1617-1630`                      | Invert `step_succeeded_event_maps_to_slot_written_kind`. | Test |
| `vb_storage`        | `src/tests.rs:3294-3370,3885-3915,2076-2097`        | record_kind() / id() golden tables; invert StepSucceeded→SlotWritten. | Test |
| `vb_storage`        | `src/record_tests.rs:48,71`                         | RecordKind id assertions.                        | Test        |
| `workspace_tests`   | `tests/postcard_envelope_wire_tests.rs:89,366,471-490` | id→RecordKind decode + golden for envelope.    | Test        |
| `workspace_tests`   | `tests/vb_eepg_bdd_tests.rs:509-531`                | Full RecordKind enumeration list.                | Test        |
| `workspace_tests`   | `tests/restate_postcard_newtype_compat_tests.rs`    | Verify postcard enum wire bytes for RecordKind did not shift. | Review |

## 4. Forbidden Crossings

The contract forbids:

- Reaching into the Fjall keyspace directly from the codec layer (the
  codec returns a typed `Result`; the storage boundary owns the
  append).
- Looping over u16 id values with arithmetic `kind + 1` style iteration
  in the codec; iteration is via `match` or via `is_known_record_kind`.
- Constructing a `RecordEnvelope` from raw bytes outside `decode_record`;
  the parse-once-at-the-boundary rule applies.
- Calling `JournalEvent::record_kind()` from non-codec paths when the
  intent is "what variant is this?" — callers should `match` on the
  variant directly.
- Mixing the legacy envelope id into writer code paths. The writer
  exclusively emits id 33 for `StepSucceeded`.

## 5. Boundary Map Summary

The boundary split is straightforward: parser/codec on one side,
storage/Fjall on the other, pure logic in the middle. The change in this
bead adds NO new boundaries — it deepens the existing parity predicate
to admit a typed legacy-tolerance relationship. The back-compat decision
is encoded in the type system, not in a runtime config flag.
