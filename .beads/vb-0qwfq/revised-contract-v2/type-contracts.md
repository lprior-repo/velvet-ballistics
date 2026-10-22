# Type Contracts — Run Sequence Number Allocation

## EventSeq

```rust
// crates/vb_storage/src/types.rs:69-96
pub struct EventSeq(u64);

impl EventSeq {
    pub const fn new(value: u64) -> Self;      // no bounds check
    pub const fn get(self) -> u64;
    pub const ZERO: Self = Self(0);
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(u64::MAX);
}
```

**Contract**:
- `new(v)` accepts any u64; no runtime validation.
- Caller must ensure `v < u64::MAX` when incrementing; otherwise use `checked_add` and report `JournalError::SequenceOverflow` on overflow.
- `ZERO` is the initial sequence for the first event in a new run.

## JournalWriteBatch<'j>

```rust
// crates/vb_storage/src/batch/types.rs
pub struct JournalWriteBatch<'j> {
    inner: fjall::OwnedWriteBatch,
    journal: &'j FjallJournal,
    staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>,
    aborted: bool,
    staged_bytes: u64,
    byte_limit: Option<u64>,
    _not_send_or_sync: PhantomData<*mut FjallJournal>,
}
```

**Contract**:
- `!Send + !Sync`; batch lifetime bound to journal reference.
- `strict()` sets durability to `PersistMode::SyncAll`.
- `commit()` fails with `BatchAborted` if any prior stage step set `aborted = true`.
- Duplicate `(run, seq)` detection within same batch via `staged_event_keys`.
- Cross-batch duplicate detection via Fjall key uniqueness.

## RunSubmission

```rust
// crates/vb_storage/src/inputs/submission.rs
pub struct RunSubmission {
    source: WorkflowSourceRecord,
    artifact: AcceptedArtifact,
    inputs: RunInputsRecord,
    header: RunHeaderRecord,
}
```

**Contract**:
- Encapsulates complete prepared envelope for a new run.
- `new()` validates identity binding: source → artifact → inputs → header must reference the same run and compatible digests.
- `commit()` writes all components atomically in one batch.
- `RunAlreadyExists` error if run header, inputs, or events already present.

## AcceptedArtifact

```rust
// vb_core/src/artifact.rs
pub struct AcceptedArtifact {
    digest: WorkflowDigest,
    source_digest: WorkflowDigest,
    ir: Vec<u8>,
    verification: ArtifactVerification,
    accepted_at_seq: EventSeq,
    required_capabilities: Box<[ActionContract]>,
    action_contracts: Box<[ActionContract]>,
    // ... other fields
}
```

**Contract**:
- `accepted_at_seq` is the sequence at which the artifact was accepted into the store.
- Freshness floor checks compare `artifact.accepted_at_seq < required_at_least`.
- Digest must match `verification.digest` and `source_digest` (transitive binding).