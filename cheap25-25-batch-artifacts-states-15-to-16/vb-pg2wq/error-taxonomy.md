# Error Taxonomy — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair

## Surface in Scope

The test-fix only touches assertions on `JournalError::DuplicateEvent { run, seq }`. The full `JournalError` enum is enumerated below for completeness so the test can correctly **distinguish** the target variant from siblings, but no production change to `JournalError` is in scope.

## Canonical Error: `JournalError::DuplicateEvent`

**File:** `crates/vb_storage/src/error/mod.rs:30-31`

```rust
#[error("duplicate journal event for run {run:?} seq {seq:?}")]
DuplicateEvent { run: RunId, seq: EventSeq },
```

- **Triggered by:** `JournalWriteBatch::append_event` when the durable keyspace `journal.events.contains_key(key)` is `true` for the `(run, seq)` of the incoming event (lines 61-67 of `crates/vb_storage/src/batch/append_event.rs`).
- **Payload contract:** `run: RunId, seq: EventSeq` — the exact `event.run_id()` and `event.seq()` of the rejected event (NOT a copy with shifted fields, NOT a synthesized `RunId::ZERO`).
- **Side effect:** `self.aborted = true` — the batch is poisoned; subsequent `commit()` returns `BatchAborted` (PS_004 verifies this).
- **Diagnostic code:** `DUPLICATE_EVENT_CODE` mapped at `crates/vb_storage/src/error/codes.rs:104` (string `"DUPLICATE_EVENT"` at `codes.rs:197`). Out of scope for this bead but available for future enhancement.
- **Test contract:** Every weak assertion in scope MUST be rewritten to pin both `run` and `seq` to the proptest inputs.

## Sibling Variants (FORBIDDEN result variants in cross-batch scenario)

These variants MUST NOT be returned by `b2.append_event(&event)` in the cross-batch scenario. The strong assertion catches regressions that return them.

### `JournalError::DuplicateStagedKey { run: RunId, seq: EventSeq }`

**File:** `crates/vb_storage/src/error/mod.rs:32-33`

```rust
#[error("duplicate journal event staged in the same batch for run {run:?} seq {seq:?}")]
DuplicateStagedKey { run: RunId, seq: EventSeq },
```

- **Triggered by:** `JournalWriteBatch::append_event` when `self.staged_event_keys.contains(&key)` is `true` (same-batch duplicate, lines 55-60 of `append_event.rs`).
- **Payload:** IDENTICAL fields to `DuplicateEvent` (`run: RunId, seq: EventSeq`).
- **Test contract:** Cross-batch scenario MUST NOT produce this. The strong assertion `let Err(JournalError::DuplicateEvent { .. }) = ... else { panic!() }` panics if this variant is returned.

### `JournalError::BatchAborted`

**File:** `crates/vb_storage/src/error/mod.rs:42-43`

```rust
#[error("journal write batch was aborted; commit is a no-op")]
BatchAborted,
```

- **Triggered by:** `JournalWriteBatch::commit` after `self.aborted == true` (PS_004 verifies this in the second assertion).
- **Payload:** unit.
- **Test contract:** NOT in `result` (which is `b2.append_event(&event)`); it appears in the SECOND `commit_result` in PS_004. Out of scope for this bead's first assertion; preserved as-is.

### `JournalError::QueueFull`

**File:** `crates/vb_storage/src/error/mod.rs:38-39`

```rust
#[error("journal writer queue is full")]
QueueFull,
```

- **Triggered by:** `JournalWriteBatch::append_event` when `self.inner.len() >= MAX_BATCH_COUNT` (line 68 of `append_event.rs`).
- **Payload:** unit.
- **Test contract:** MUST NOT be returned in the cross-batch duplicate scenario. The strong assertion catches this.

### `JournalError::KeyCapacity`

**File:** `crates/vb_storage/src/error/mod.rs:28-29`

```rust
#[error("journal key capacity exceeded")]
KeyCapacity,
```

- **Test contract:** MUST NOT be returned by `append_event` in this scenario.

### `JournalError::InvalidEvent`

**File:** `crates/vb_storage/src/error/mod.rs` (variant declaration out of view at line 80+)

- **Triggered by:** `JournalWriteBatch::append_event` when `!event.is_valid()` (line 44-46 of `append_event.rs`).
- **Test contract:** MUST NOT be triggered by `make_event(run, seq)` which constructs a valid `RunAccepted`. The strong assertion catches this if a regression somehow invalidates the event.

### `JournalError::SequenceOverflow`, `JournalError::Encode`, `JournalError::Fjall`, etc.

- Lower-priority siblings. The strong assertion catches ALL non-`DuplicateEvent` variants via the `let-else` exhaustive panic or the `matches!` guard returning `false`.

## Forbidden Result Variants in the Duplicate Scenario

The strong assertion (field-bound `matches!` guard or `let-else` exhaustive pattern) MUST reject every variant below. Each is enumerated to make the regression-resistance explicit.

| Variant | File:line | Why forbidden here |
|---------|-----------|--------------------|
| `Ok(())` | n/a | Cross-batch duplicate MUST error, not silently overwrite. |
| `Err(JournalError::DuplicateStagedKey { .. })` | `error/mod.rs:32` | Same-batch duplicate; not the cross-batch scenario. |
| `Err(JournalError::QueueFull)` | `error/mod.rs:38` | Capacity exhaustion; proptest uses 1 event. |
| `Err(JournalError::BatchAborted)` | `error/mod.rs:42` | Only valid on `commit`, not `append_event`. |
| `Err(JournalError::KeyCapacity)` | `error/mod.rs:28` | Journal config-level; not triggered here. |
| `Err(JournalError::InvalidEvent)` | `error/mod.rs` | `make_event` is valid. |
| `Err(JournalError::Encode(_))` | `error/mod.rs:24` | Encoding failure; not triggered here. |
| `Err(JournalError::Fjall(_))` | `error/mod.rs:22` | Storage failure; not triggered here. |
| `Err(JournalError::JournalBatchBytesExceeded { .. })` | `error/mod.rs:40` | Byte budget exceeded; 1 event is fine. |
| `Err(JournalError::PayloadTooLarge { .. })` | `error/mod.rs` (in view after line 80) | Payload too large; not triggered here. |
| `Err(JournalError::WriteLockPoisoned)` | `error/mod.rs:34` | Lock poisoned; not triggered here. |
| `Err(JournalError::QueueShutdown)` | `error/mod.rs:44` | Queue shut down; not triggered here. |
| `Err(JournalError::QueueCapacity)` | `error/mod.rs:36` | Queue config; not triggered here. |
| `Err(JournalError::SequenceOverflow)` | `error/mod.rs:69` | Sequence overflow; not triggered here. |

## Required Result Variant

Exactly one variant is allowed in the duplicate scenario:

```rust
Err(JournalError::DuplicateEvent {
    run: RunId::new(run),  // PROPTEST INPUT
    seq: EventSeq::new(seq), // PROPTEST INPUT
})
```

The test assertion MUST bind `run` and `seq` to the proptest inputs (after re-bagging via smart constructors) and assert equality.

## Error Variant Confusion Risk (Test-Side)

The `DuplicateEvent` and `DuplicateStagedKey` variants carry IDENTICAL payload fields (`run: RunId, seq: EventSeq`). A test that uses `matches!(result, Err(JournalError::DuplicateStagedKey { .. }))` would also pass for `DuplicateEvent`. The strong assertion binds the **variant name** in the pattern; a regression that swaps the production branch to return `DuplicateStagedKey` instead of `DuplicateEvent` fails because the variant does not match.

This is a real risk because the production code has BOTH branches (`append_event.rs:55-67`) and a regression could move the early-return boundary.

## Test-Side Panic Surface

The `let-else` exhaustive pattern panics if `result` is anything other than `Err(DuplicateEvent { .. })`:

```rust
let Err(JournalError::DuplicateEvent { run, seq }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
```

This `panic!` is allowed in test code per the master contract. The format string includes `{:?}` of `result` for diagnostic clarity; this is `Debug` formatting of the `Result<(), JournalError>` type, which derives `Debug`.

For proptest functions, the equivalent does NOT use `panic!`; it uses `prop_assert!(matches!(... if guard))` so proptest can shrink the counterexample. The `prop_assert!` macro returns a `Result<(), TestCaseError>` internally; the `if` guard binds the field check inside the macro.

## Forbidden Test Patterns (Reaffirmed)

- `assert!(result.is_err())` — accepts any error variant.
- `assert_ne!(result, Ok(()))` — accepts any error variant.
- `matches!(result, Err(JournalError::DuplicateEvent { .. }))` — accepts any tuple.
- `matches!(result, Err(_))` — accepts any error.
- `assert!(format!("{result:?}").contains("DuplicateEvent"))` — stringly; misses `DuplicateStagedKey`.

## Required Test Patterns (Reaffirmed)

For proptest lanes (PS_001, PS_003, PS_004, PS_008, PS_009):

```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

For non-proptest reference pattern (canonical, at `vb_storage/src/tests.rs:1344-1367`, not modified by this bead):

```rust
let Err(JournalError::DuplicateEvent { run, seq }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(run, RunId::new(EXPECTED));
assert_eq!(seq, EventSeq::new(EXPECTED));
```

Both patterns reject ALL forbidden variants and bind ALL required fields.