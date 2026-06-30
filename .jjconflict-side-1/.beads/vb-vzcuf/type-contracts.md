# Type Contracts: vb-vzcuf

## Required Types

These are domain/type contracts, not production Rust implementations.

### `JournalBatchByteLimit`

- Representation: newtype over `u32`, `u64`, or `usize`; proof lanes should prefer a single explicit representation and bridge conversions.
- Constructor: rejects zero.
- Upper bound: if represented as `u32`, max is `u32::MAX`; if represented as `usize`, must reject values not representable by proof/refinement bounds.
- Semantic default: align to `vb_core::ResourceContract::max_journal_batch_bytes` default (`1_048_576`) unless an explicit storage default is approved.
- Illegal states eliminated: absent limit, zero limit, unbounded batch.

### `StagedJournalEventBytes`

- Representation: accumulator newtype with zero constructor and checked-add transition.
- Public observation: `batch_journal_event_bytes()` or equivalent should expose the current total for tests, proofs, and operators.
- Transition: `try_accept(encoded_len, limit) -> Accepted(new_total) | Rejected(error)`.
- Illegal states eliminated: negative total, total above limit after successful admission, unchecked overflow.

### `EncodedJournalEventBytes`

- Constructed only from `value.len()` returned by successful `encode_record` for journal events.
- Must not be confused with postcard payload length from `payload_len_u32`.
- Conversion from `usize` must be checked if the accumulator/error fields are narrower.

### `JournalBatchLimits`

- Groups `JournalBatchByteLimit` and any future batch admission limits.
- Default constructor must be safe and bounded.
- Boundary parser converts core `ResourceContract::max_journal_batch_bytes` into the storage value object.

### `JournalError` accumulated-budget variant

Required semantic shape:

```text
JournalError::JournalBatchBytesExceeded {
    attempted: <bounded integer>,
    limit: <same or bridgeable bounded integer>,
}
```

Acceptable field aliases: `actual` for attempted total, `max` for limit. The variant must be distinct from `QueueFull` and `PayloadTooLarge`.

Overflow handling must produce either the same variant with `attempted` saturated/omitted under a documented `Overflow` source, or a more explicit `JournalBatchByteAccountingOverflow { staged, event_len, limit }`. It must not use panic/unwrap/expect.

## API Contract

### Construction

- `JournalWriteBatch::new(journal)` remains bounded by a default `JournalBatchByteLimit`.
- A new or existing storage-visible API must allow injecting a validated byte limit from core/runtime policy.
- The batch must never enter `Open` without a byte limit.

### Append Event

Guard order contract:

1. Build journal event key; key construction errors return normally and do not mutate.
2. Durable duplicate check; duplicate returns `DuplicateEvent` and preserves existing abort behavior.
3. Count capacity check; full returns `QueueFull`, does not abort, does not mutate byte total.
4. Encode record; per-record oversized payload returns `PayloadTooLarge`, does not mutate byte total.
5. Convert encoded length to `EncodedJournalEventBytes`; conversion failure is accumulated accounting error.
6. Checked accumulated admission against `JournalBatchByteLimit`.
7. Insert encoded value and update `StagedJournalEventBytes` atomically in program order.

### Observability

- Provide a public or crate-visible accessor for current staged journal event bytes.
- Accessor semantics on aborted batch must be specified. Recommended: return zero if mirroring `len()`, or return `Option/StagedBatchState` if preserving diagnostics. Do not leave it ambiguous.

## Illegal States That Must Become Unrepresentable

- Open batch with no byte limit.
- Open batch with zero byte limit.
- Successful append with `staged_journal_event_bytes > limit`.
- Successful append where `staged_journal_event_bytes` does not equal accepted encoded event bytes.
- Accumulated budget error represented as `QueueFull` or `PayloadTooLarge`.
- Overflow modeled as success, wraparound, panic, or Fjall failure.

## Rust Reliability Constraints

- No `unsafe`.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or unchecked arithmetic.
- No unchecked casts from `usize` to `u32`/`u64`.
- No behavior flags such as `enforce_bytes: bool`; use typed limits/policies.
- No `Option<limit>` in the core open-batch state; parse optional external policy at the boundary.
