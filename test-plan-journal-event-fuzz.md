# Test Plan: LETHAL-7 — `journal_event` Fuzz Target

## Summary

- **Bead**: LETHAL-7
- **Problem**: `fuzz/fuzz_targets/journal_event.rs` does not exist. `DRIFT-2` cannot be closed.
- **Dependencies**: Requires `vb_storage::journal::parse_event()` and `JournalEvent::is_valid()` implementations
- **Behaviors identified**: 4 core behaviors, 8+ error variants
- **Trophy allocation**: 0 unit / 2 integration (existing decode_record proptests) / 1 fuzz / 0 e2e / 0 static
- **Fuzz targets**: 1 (journal_event)
- **Kani harnesses**: 0 (decode_record already has Kani coverage in `kani_record_*.rs`)

---

## 1. Behavior Inventory

### `parse_event` (new function in `vb_storage::journal`)

| # | Behavior |
|---|----------|
| B1 | `parse_event(data)` returns `Result<JournalEvent, JournalError>` and must never panic |
| B2 | `parse_event` accepts valid `MAGIC_JOURNAL_EVENT` record bytes and returns the decoded event |
| B3 | `parse_event` rejects wrong magic bytes with `JournalError::BadMagic` |
| B4 | `parse_event` rejects truncated records with `JournalError::UnexpectedEof` |
| B5 | `parse_event` rejects corrupt payload with `JournalError::PayloadDigestMismatch` |
| B6 | `parse_event` rejects unknown `RecordKind` with `JournalError::UnknownRecordKind` |

### `JournalEvent::is_valid()` (new method on `JournalEvent`)

| # | Behavior |
|---|----------|
| B7 | `JournalEvent::is_valid()` returns `true` for events with valid run_id, seq, and variant-specific fields |
| B8 | `JournalEvent::is_valid()` returns `false` for events with zero run_id |
| B9 | `JournalEvent::is_valid()` returns `false` for `SlotWrittenEvent` with `seq == EventSeq::MAX` |
| B10 | `JournalEvent::is_valid()` returns `false` for events with attempt == 0 where attempt is required |

### Fuzz Target Invariant

| # | Behavior |
|---|----------|
| B11 | For any `data: &[u8]`, `parse_event(data)` must not panic |
| B12 | If `parse_event(data)` returns `Ok(event)`, then `event.is_valid()` must be `true` |

---

## 2. Trophy Allocation

| Layer | Allocation | Rationale |
|-------|-----------|-----------|
| **Fuzz** | 1 target | Primary coverage for deserialization boundary — all 18 `JournalEvent` variants + corrupt/truncated inputs |
| **Integration** | 2 | Existing proptests in `vb_2bok_durability_gate_tests.rs` and `proptests.rs` cover round-trip encode/decode |
| **Static** | 0 | Kani harnesses already cover header decoding (`kani_record_*.rs`) |

No unit tests needed — all behavior is exercised via the fuzz target and existing integration tests.

---

## 3. BDD Scenarios

### Scenario: parse_event accepts valid SlotWrittenEvent

```gherkin
Given: A valid JournalEvent record encoded with MAGIC_JOURNAL_EVENT
  And: RecordKind = SlotWritten
  And: Payload is a postcard-encoded SlotWrittenEvent with run, seq, slot, attempt, value
When: parse_event(bytes) is called
Then: Returns Ok(SlotWrittenEvent)
 And: The returned event.is_valid() is true
```

### Scenario: parse_event rejects wrong magic

```gherkin
Given: Record bytes with magic != MAGIC_JOURNAL_EVENT (e.g., MAGIC_BLOB, MAGIC_SNAPSHOT, 0xFFFF_FFFF, 0x0000_0000)
When: parse_event(bytes) is called
Then: Returns Err(JournalError::BadMagic { expected: 0x5642_4A45, actual: <given> })
```

### Scenario: parse_event rejects truncated header

```gherkin
Given: Bytes shorter than RECORD_HEADER_BYTES (60 bytes)
When: parse_event(bytes) is called
Then: Returns Err(JournalError::UnexpectedEof { expected: 60, actual: <len> })
```

### Scenario: parse_event rejects truncated payload

```gherkin
Given: A valid 60-byte header declaring payload_len = N
  And: Fewer than N payload bytes follow
When: parse_event(bytes) is called
Then: Returns Err(JournalError::UnexpectedEof { .. })
```

### Scenario: parse_event rejects corrupt payload

```gherkin
Given: A valid record with correct header but payload bytes mutated
When: parse_event(bytes) is called
Then: Returns Err(JournalError::PayloadDigestMismatch)
```

### Scenario: parse_event rejects future schema version

```gherkin
Given: A record with schema_version = CURRENT_SCHEMA_VERSION + 1
When: parse_event(bytes) is called
Then: Returns Err(JournalError::SchemaVersionTooNew { .. })
```

### Scenario: parse_event rejects all-zero bytes

```gherkin
Given: All-zero bytes of length >= RECORD_HEADER_BYTES
When: parse_event(bytes) is called
Then: Returns Err (not panic)
 And: Error is one of: BadMagic, UnknownRecordKind, UnexpectedEof, PayloadDigestMismatch
```

### Scenario: All 18 JournalEvent variants parse correctly

```gherkin
Given: Valid postcard-encoded bytes for each JournalEvent variant:
  - RunAccepted, RunAdmission, StepStarted, StepSucceeded
  - ActionScheduled, ActionCompletedEvent, ActionFailedEvent
  - SlotWrittenEvent, WaitScheduledEvent, AskScheduledEvent, AskAnsweredEvent
  - RetryScheduledEvent, RunCancelled, RunFinished, RunFailedEvent
  - RunResumed, RunRetried, RunAnswered
When: Each is passed to parse_event
Then: Each returns Ok(event) where event.is_valid() == true
```

---

## 4. Proptest Invariants

No new proptest invariants required. Existing proptests in `proptests.rs` already cover:
- `encode_decode_record_roundtrip_for_all_record_kinds` — verifies decode(encode(v)) == v for all RecordKind variants
- `roundtrips_through_postcard_without_panic` — verifies postcard roundtrip for JournalEvent variants

---

## 5. Fuzz Targets

### Fuzz Target: `journal_event`

**File**: `fuzz/fuzz_targets/journal_event.rs`

**Input type**: `&[u8]`

**Risk class**:
- Panic on malformed header parsing
- Panic on invalid record kind enum deserialization
- Panic on postcard decode failure
- OOM on oversized payload declaration
- Logic error: valid event returning `is_valid() == false`

**Corpus seeds required** (directory: `fuzz/corpus/journal_event/`):

| Seed File | Content | Purpose |
|-----------|---------|---------|
| `valid_slot_written.event` | Valid `SlotWrittenEvent` record (postcard + envelope) | Happy path for slot write |
| `valid_run_accepted.event` | Valid `RunAccepted` record | Happy path for run start |
| `valid_run_finished.event` | Valid `RunFinished` record | Terminal event |
| `valid_run_cancelled.event` | Valid `RunCancelled` record with reason | Cancellation event |
| `valid_step_started.event` | Valid `StepStarted` record | Mid-run event |
| `valid_action_completed.event` | Valid `ActionCompletedEvent` record | Action completion |
| `truncated_header_10b.bin` | 10 bytes (below 60-byte header) | Truncated header |
| `truncated_header_50b.bin` | 50 bytes (partial header) | Partial header |
| `truncated_payload.bin` | Full header + 1 byte of declared payload | Truncated payload |
| `wrong_magic_blob.bin` | Header with MAGIC_BLOB (0x5642_424C) | Wrong record type |
| `wrong_magic_all_zero.bin` | All zeros | Invalid magic |
| `wrong_magic_ffff.bin` | Header with magic 0xFFFF_FFFF | Garbage magic |
| `corrupt_payload.bin` | Valid header + mutated payload bytes | Digest mismatch |
| `future_schema_v2.bin` | Header with schema_version = 2 | Schema version future |
| `unknown_record_kind.bin` | Header with record_kind = 0 or 255 | Unknown kind |

**Fuzz target body**:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Journal event deserialization must not panic
    let result = vb_storage::journal::parse_event(data);

    // If parsing succeeded, the event must be valid
    if let Ok(event) = result {
        assert!(
            event.is_valid(),
            "parse_event succeeded but is_valid() returned false for {:?}",
            event
        );
    }
});
```

**Implementation requirements**:

1. **`vb_storage::journal::parse_event(data: &[u8]) -> Result<JournalEvent, JournalError>`**:
   - Wrapper around `decode_record::<JournalEvent>(data, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)`
   - Must be added to `vb_storage/src/journal/mod.rs` or a new `journal/parse.rs`
   - Returns the `JournalEvent` directly (not the `RecordEnvelope`)

2. **`JournalEvent::is_valid() -> bool`**:
   - Returns `true` if:
     - `run_id != RunId::ZERO`
     - For events with `seq`: `seq != EventSeq::MAX`
     - For events with `attempt`: `attempt >= 1`
   - Returns `false` otherwise

---

## 6. Kani Harnesses

No new Kani harnesses required. Existing harnesses cover the critical header decoding paths:
- `kani_record_magic.rs` — verifies `BadMagic` for wrong magic
- `kani_record_kind.rs` — verifies `UnknownRecordKind` for unknown kinds
- `kani_record_payload_len.rs` — verifies `PayloadTooLarge` bounds
- `kani_record_crc.rs` — verifies `HeaderChecksumMismatch` detection
- `kani_record_schema.rs` — verifies `MigrationRequired` for old schema

---

## 7. Mutation Checkpoints

**Threshold**: 85% mutation kill rate (decode_record is already well-tested)

| Mutation | Must be caught by |
|----------|-------------------|
| Remove header magic validation | `parse_event` returns `Err(BadMagic)` not panic |
| Remove payload digest verification | `parse_event` returns `Err(PayloadDigestMismatch)` not panic |
| Remove schema version check | `parse_event` returns `Err(SchemaVersionTooNew)` not panic |
| Remove RecordKind family check | `parse_event` returns `Err(UnknownRecordKind)` not panic |
| Remove payload size bound check | `parse_event` returns `Err(PayloadTooLarge)` not panic |

---

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: SlotWrittenEvent | Valid record bytes | `Ok(event)` + `is_valid()` | fuzz |
| Happy: RunAccepted | Valid record bytes | `Ok(event)` + `is_valid()` | fuzz |
| Happy: RunFinished | Valid record bytes | `Ok(event)` + `is_valid()` | fuzz |
| Error: Truncated header (<60B) | 0–59 bytes | `Err(UnexpectedEof)` | fuzz |
| Error: Truncated payload | Header + partial payload | `Err(UnexpectedEof)` | fuzz |
| Error: Wrong magic | MAGIC_BLOB, 0xFFFF, 0x0000 | `Err(BadMagic)` | fuzz |
| Error: Corrupt payload | Valid header + mutated payload | `Err(PayloadDigestMismatch)` | fuzz |
| Error: Future schema | schema_version = 2 | `Err(SchemaVersionTooNew)` | fuzz |
| Error: Unknown RecordKind | kind = 0, 255 | `Err(UnknownRecordKind)` | fuzz |
| All 18 variants roundtrip | Each variant encoded then parsed | Original == Parsed | integration |

---

## Open Questions

1. **Parse event wrapper location**: `vb_storage::journal::parse_event` — should this live in `journal/mod.rs`, `journal/core.rs`, or a new `journal/parse.rs` module? Recommend `journal/parse.rs` to keep concerns separated.

2. **`is_valid()` semantic scope**: Does `is_valid()` need to verify the `WorkflowDigest` in `RunAccepted`/`RunAdmission`? If so, it would need to accept a reference to the known-good digest. Recommend: `is_valid()` only checks structural validity (non-zero run, valid seq, valid attempt), not cryptographic digests.

3. **Existing `fuzz_lib::fuzz_journal_event`**: The library function at `fuzz/src/lib.rs:255` already exists and exercises `decode_record`. Should this be refactored to call the new `parse_event` wrapper, or kept separate? Recommend: refactor to use `parse_event` to avoid code duplication.

4. **Corpus seed format**: Should corpus seeds be raw envelope bytes (postcard + storage header) or just postcard-encoded `JournalEvent` bytes? The fuzz target parses full records, so seeds should be full envelope bytes.
