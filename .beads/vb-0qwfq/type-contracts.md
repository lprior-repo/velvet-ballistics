# Type Contracts: Artifact Acceptance Sequence

## EventSeq (existing, reused)

```rust
pub struct EventSeq(u64);
impl EventSeq {
    pub const fn new(value: u64) -> Self;
    pub const fn get(self) -> u64;
    pub const ZERO: Self;
    pub const MAX: Self;
}
```

- Wraps u64 for both run event sequences and artifact acceptance sequences.
- Ord for freshness comparisons (`accepted_at_seq < required_at_least`).
- Serialize/Deserialize for artifact envelope persistence.
- No change required for this bead.

## AcceptedArtifact (existing, field semantics change)

```rust
pub struct AcceptedArtifact {
    pub digest: WorkflowDigest,
    pub source_digest: WorkflowDigest,
    pub policy_digest: WorkflowDigest,
    pub ir: Vec<u8>,
    pub verification: VerificationProof,
    pub accepted_at_seq: EventSeq,  // new: nonzero journal-allocated value
    pub required_capabilities: Box<[Capability]>,
    pub action_contracts: Box<[ActionContract]>,
}
```

- `accepted_at_seq` was previously `EventSeq::new(0)` sentinel; now carries the journal-allocated sequence.
- Part of the envelope digest computed by `accepted_artifact_digest()`.
- Smart constructor: `prepare_compiled_artifact()` now receives the allocated sequence.

## New Journal API: accept_seq_allocate

```rust
impl FjallJournal {
    pub fn accept_seq_allocate(&self) -> Result<EventSeq, JournalError>;
}
```

- Atomically reads counter, increments, writes back, returns new value.
- Uses journal's `write_lock` for mutual exclusion.
- Errors: `JournalError::WriteLockPoisoned`, `JournalError::SequenceOverflow`, Fjall IO errors.
- Durable before returning; counter is written to the `system` keyspace.

## Keyspace: system (new)

```rust
pub const KEYSPACE_SYSTEM: &str = "system";
pub const PREFIX_SYSTEM: u8 = 0x40;
pub const SYSTEM_ACCEPT_SEQ_KEY: &[u8] = &[PREFIX_SYSTEM, 0x01];
```

- Hot keyspace profile (bloom filter, small read/write path).
- Contains exactly one key: the acceptance sequence counter as big-endian u64.
- Value semantics: the next value to hand out (counter, not last-handout).

## JournalError (existing, new variant)

```rust
pub enum JournalError {
    // ... existing variants
    SequenceOverflow,  // new: counter reached EventSeq::MAX
}
```

## Admission API (unchanged signature, changed behavior)

```rust
pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError>;
```

- Internally calls `journal.accept_seq_allocate()` before preparing artifact.
- All policies (Relaxed/Journaled/Strict) now receive nonzero `accepted_at_seq`.
- Relaxed still skips SyncAll; sequence allocation is durable regardless.
