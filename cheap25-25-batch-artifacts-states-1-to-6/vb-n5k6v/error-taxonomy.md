# Error Taxonomy — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Decision (one-paragraph rationale)

This bead is a **build-system repair**: it does not introduce, modify,
or remove any error variants. The error taxonomy below documents the
**error surface that the 26 surfaced tests assert against** — not
errors introduced by the bead itself. The contract's relationship to
errors is purely **observational**: each test pins one or more
`JournalError` variants as post-conditions, and the wire surfaces those
pins into the cargo test run.

The contract explicitly **does not**:

- Add a new `JournalError` variant.
- Add a new diagnostic code.
- Modify any error variant's shape, fields, or `From` impls.
- Touch `error/mod.rs`, `error/codes.rs`, or any other error source.

Any error variant the tests assert against is **pre-existing** and
already exercised by the 16 wired sibling modules (e.g.,
`error_tests.rs`, `error_code_tests.rs`).

---

## Error Variant Tree (relevant subset, pre-existing, NOT modified)

```
JournalError                                              (error/mod.rs)
├── StrictDurabilityFailed                                (no payload)           <-- pinned by tests #1, #2
├── PayloadTooLarge { max: usize, actual: usize }          (struct)               <-- pinned by test #19
├── RecordKindFamilyMismatch { magic: u32, kind: RecordKind }                     <-- pinned by test #16
├── DuplicateEvent { run: RunId, seq: EventSeq }          (struct)               <-- pinned by test #21
├── QueueShutdown                                         (no payload)           <-- pinned by test #26
└── (many unrelated variants, not exercised by edge_case_tests.rs)
```

The 5 `JournalError` variants exercised by `edge_case_tests.rs` are:

| Variant | Pinned by test # | Test name | Production return site |
|---------|------------------|-----------|------------------------|
| `JournalError::StrictDurabilityFailed` | #1, #2 | `persist_strict_handles_simulated_failure`, `persist_strict_recovers_after_simulated_failure` | `journal/append.rs:84` |
| `JournalError::PayloadTooLarge { .. }` | #19 | `decode_rejects_zero_max_payload_with_nonzero_payload` | `codec/mod.rs` (returns from `decode_record`) |
| `JournalError::RecordKindFamilyMismatch { .. }` | #16 | `encode_rejects_unknown_magic` | `error/mod.rs:80` (returned from `encode_record`) |
| `JournalError::DuplicateEvent { .. }` | #21 | `batch_commit_then_second_batch_with_same_run_seq_rejected` | `batch/append_event.rs:63` (returned from `BatchBuilder::append_event`) |
| `JournalError::QueueShutdown` | #26 | `queue_rejects_all_writes_after_shutdown` | `error/mod.rs:45` (returned at `queue/writer.rs:82`) |

The remaining 21 tests assert **success** (`Ok(_)`) post-conditions or
non-error invariants (`assert_eq!(events.len(), N)`,
`assert!(result.is_ok())`).

---

## Per-Variant Contracts (pre-existing, NOT modified)

### `JournalError::StrictDurabilityFailed`

```rust
// error/mod.rs (pre-existing)
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ...
    #[error("strict durability persist failed")]
    StrictDurabilityFailed,
    // ...
}
```

| Field | Domain meaning | Range |
|-------|----------------|-------|
| (none) | The strict-persist path (fsync) failed. | n/a (unit variant) |

| Pinning test | Assertion shape |
|--------------|-----------------|
| `persist_strict_handles_simulated_failure` | `matches!(result, Err(JournalError::StrictDurabilityFailed))` after `fail_next_persist_for_test()` then `persist_strict()` |
| `persist_strict_recovers_after_simulated_failure` | First `append_strict` returns `Err(StrictDurabilityFailed)`; second `append_strict` returns `Ok` |

**Why pinned here**: validates the **strict-durability failure path**
and the **recovery path**. Both are part of the project's P1 wave-3
dormant-test sweep; surfacing them restores lost CI coverage.

**Stability**: the variant shape and name are stable; pinned by both
`edge_case_tests.rs:1-2` and the sibling `error_tests.rs` module.

### `JournalError::PayloadTooLarge { max: usize, actual: usize }`

```rust
// error/mod.rs (pre-existing)
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ...
    #[error("payload too large: max={max}, actual={actual}")]
    PayloadTooLarge { max: usize, actual: usize },
    // ...
}
```

| Field | Domain meaning | Range |
|-------|----------------|-------|
| `max: usize` | The maximum allowed payload length passed to `decode_record`. | `0..=usize::MAX` |
| `actual: usize` | The actual length observed in the encoded record. | `0..=usize::MAX` |

| Pinning test | Assertion shape |
|--------------|-----------------|
| `decode_rejects_zero_max_payload_with_nonzero_payload` | Encodes a `RunAccepted` event, then calls `decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 0)` (zero `max_payload_len`); asserts `matches!(result, Err(JournalError::PayloadTooLarge { .. }))` |

**Why pinned here**: validates the **decoder's max-payload-length
boundary** when the caller passes `max = 0` against a non-zero encoded
payload. The `..` in the pattern means the test does not pin specific
field values, only the variant.

**Stability**: the field names and types are stable; pinned by
`edge_case_tests.rs:19` and the sibling `codec/tests.rs` module.

### `JournalError::RecordKindFamilyMismatch { magic: u32, kind: RecordKind }`

```rust
// error/mod.rs:80 (pre-existing)
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ...
    #[error("record kind {kind:?} does not match magic 0x{magic:08x}")]
    RecordKindFamilyMismatch { magic: u32, kind: RecordKind },
    // ...
}
```

| Field | Domain meaning | Range |
|-------|----------------|-------|
| `magic: u32` | The 4-byte magic prefix of the encoded record. | `0x0000_0000..=0xFFFF_FFFF` |
| `kind: RecordKind` | The `RecordKind` enum variant passed to `encode_record`. | enum (5+ variants) |

| Pinning test | Assertion shape |
|--------------|-----------------|
| `encode_rejects_unknown_magic` | Calls `encode_record(0xFFFF_0000, RecordKind::WorkflowSource, 0, &record, 128)`; asserts `matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. }))` |

**Why pinned here**: validates the **encoder's magic/kind family
table** — `WorkflowSource` (kind 0) maps to a specific magic prefix;
passing a wrong magic (0xFFFF0000) must yield this typed error rather
than silently accepting.

**Stability**: the field names and types are stable; pinned by
`edge_case_tests.rs:16` and the sibling `record_tests.rs` and
`kani_record_magic.rs` harnesses.

### `JournalError::DuplicateEvent { run: RunId, seq: EventSeq }`

```rust
// error/mod.rs (pre-existing); returned at batch/append_event.rs:63
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ...
    #[error("duplicate event for run {run:?} seq {seq:?}")]
    DuplicateEvent { run: RunId, seq: EventSeq },
    // ...
}
```

| Field | Domain meaning | Range |
|-------|----------------|-------|
| `run: RunId` | The run ID that already has an event at this seq. | newtype wrapping `u64` |
| `seq: EventSeq` | The duplicate sequence number. | newtype wrapping `u64` |

| Pinning test | Assertion shape |
|--------------|-----------------|
| `batch_commit_then_second_batch_with_same_run_seq_rejected` | Commits batch1 with event `(run=1, seq=0)`; creates batch2, calls `append_event(&same_event)`; asserts `matches!(result, Err(JournalError::DuplicateEvent { .. }))` |

**Why pinned here**: validates the **batch cross-commit duplicate
detection** — two separate batches cannot both contain the same
`(run, seq)` pair. This is a **cross-batch invariant** (different from
intra-batch duplicate detection which would be caught earlier).

**Stability**: the field names and types are stable; pinned by
`edge_case_tests.rs:21` and the sibling `batch/tests.rs` module.

### `JournalError::QueueShutdown`

```rust
// error/mod.rs:45 (pre-existing); returned at queue/writer.rs:82
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    // ...
    #[error("queue is shut down")]
    QueueShutdown,
    // ...
}
```

| Field | Domain meaning | Range |
|-------|----------------|-------|
| (none) | The `JournalWriterQueue` has been shut down; further enqueues are rejected. | n/a (unit variant) |

| Pinning test | Assertion shape |
|--------------|-----------------|
| `queue_rejects_all_writes_after_shutdown` | Creates queue, enqueues one event, calls `shutdown(&journal)`, then enqueues a second event via both `enqueue_journaled` and `enqueue_strict`; both return `Err(QueueShutdown)` |

**Why pinned here**: validates the **queue terminal-state invariant**
— once `shutdown` is called, no further writes are accepted. The test
exercises both `enqueue_journaled` and `enqueue_strict` paths.

**Stability**: the variant shape and name are stable; pinned by
`edge_case_tests.rs:26` and the sibling `queue/tests.rs` module.

---

## Error Variants NOT Touched by This Contract

For completeness, the following `JournalError` variants exist in
production but are **not pinned** by `edge_case_tests.rs`:

| Variant | Pinned by |
|---------|-----------|
| `Fjall(fjall::Error)` | `journal/tests.rs`, `recovery/tests.rs` |
| `MalformedKeyspaceRow { .. }` | `trimming/tests.rs` (via `TrimError` wrapping) |
| `NoDurableSnapshot { run: RunId }` | `recovery/tests.rs` |
| `RetentionPolicyBlocks { run: RunId }` | `trimming/tests.rs` |
| `StrictnessRequired` | `journal/tests.rs` |
| `EventOrderingViolation` | `journal/tests.rs` |
| `BatchAlreadyCommitted` | `batch/tests.rs` |
| `CapacityExceeded` | `queue/tests.rs` |
| `Timeout` | `journal/tests.rs` |
| (any others) | (sibling modules) |

The wire does **not** alter these; they continue to be pinned by their
existing sibling modules.

---

## Diagnostic Code Mapping (pre-existing, NOT modified)

Each `JournalError` variant has a `diagnostic_code()` method that
returns a `u16`. The contract does not modify any of these mappings.
The relevant subset for `edge_case_tests.rs`:

| Variant | Diagnostic code | Source |
|---------|-----------------|--------|
| `StrictDurabilityFailed` | (code registered at `error/codes.rs`) | unchanged |
| `PayloadTooLarge` | (code registered) | unchanged |
| `RecordKindFamilyMismatch` | (code registered) | unchanged |
| `DuplicateEvent` | (code registered) | unchanged |
| `QueueShutdown` | (code registered) | unchanged |

The existing `error_code_tests.rs` module (wired at `lib.rs:127`)
already exercises all diagnostic-code propagations. The wire does not
introduce new code dependencies.

---

## Error-Handling Philosophy (preserved)

The project uses **typed errors** (`thiserror::Error`) with semantic
variant names. There are **no boolean error flags**, **no string
error messages as enum discriminants**, and **no `anyhow::Error`
boundaries** in the storage crate.

The contract preserves this philosophy: every assertion in
`edge_case_tests.rs` uses `matches!(result, Err(JournalError::Variant {
.. }))` (structural matching) rather than asserting on `Display`
output or stringly-typed error codes.

---

## Forbidden Error-Handling Patterns

| Pattern | Why forbidden |
|---------|---------------|
| Introducing a new `JournalError::EdgeCaseTestFailure` variant | The 26 tests are tests, not error sources. They assert against existing variants only. |
| Wrapping `JournalError` in `anyhow::Error` for the test path | Would break the typed-error contract and make the structural `matches!` assertions fail to compile. |
| Replacing `matches!` with `assert!(result.is_err())` | Would lose the variant-pinning property and reduce test rigor. |
| Adding a `From<EdgeCaseError> for JournalError` impl | The wire does not introduce a new error type; this would be a no-op API churn. |

---

END OF ERROR TAXONOMY.