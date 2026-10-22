# Contract: Real Acceptance Sequence for Accepted Artifacts

## Problem Statement

`AcceptedArtifact.accepted_at_seq` is currently always `EventSeq::new(0)` (sentinel). This makes:
- Nonzero freshness floors in `admit_artifact_run_with_certificate_floor()` reject all artifacts.
- Zero freshness floors meaningless (every artifact passes regardless of age).
- No way to distinguish when artifacts were accepted relative to each other.

## Solution Design

Introduce a durable, journal-wide acceptance sequence counter owned by `FjallJournal`. Each artifact submission atomically allocates the next sequence value and binds it to the artifact envelope before digest computation and persistence.

## Authority Model

**One bounded authority**: `FjallJournal` instance.
- The journal owns the counter and the allocation API.
- Callers invoke `submit_artifact()` which internally allocates.
- Callers cannot fabricate or pass arbitrary sequences to the default submission path.
- The counter persists in the journal's `system` keyspace across reopen.

**Distinction from run event sequence**:
- Run event sequence: Per-run, assigned at event append time (RunAccepted at seq 0, etc.).
- Acceptance sequence: Journal-wide, assigned at artifact submission time, independent of any run.
- An artifact accepted at sequence n can be executed in any number of runs at any run event sequence.

## API Contract

### New Journal Method

```rust
impl FjallJournal {
    pub fn accept_seq_allocate(&self) -> Result<EventSeq, JournalError>;
}
```

- Atomically increments the durable counter.
- Returns the new counter value (first call returns 1).
- Errors: `SequenceOverflow`, `WriteLockPoisoned`, Fjall IO errors.
- Durable before returning (batch commit to `system` keyspace).

### Modified Submission Flow

`submit_artifact()` and `submit_artifact_with_contracts()` now:
1. Call `journal.accept_seq_allocate()` → EventSeq(n).
2. Pass n to `prepare_compiled_artifact()` → artifact with `accepted_at_seq=n`.
3. Compute artifact envelope digest (includes n).
4. Persist artifact.
5. Return artifact.

### Unchanged Public API Signatures

- `submit_artifact(journal, workflow, policy)` — same signature, seq now nonzero.
- `submit_artifact_with_contracts(journal, workflow, policy, contracts)` — same.
- No new public API for callers to pass their own sequence.

## Keyspace Contract

New keyspace: `system`
- Profile: Hot (bloom filter, small read/write path).
- Key: `[0x40, 0x01]` (SYSTEM_ACCEPT_SEQ_KEY).
- Value: big-endian u64 — the next counter value to hand out.
- Initial state: Key absent (counter=0, not yet allocated).
- Declared in `FjallJournal::declared_keyspaces()` (now 12 entries).

## Digest Stability

The acceptance sequence is included in the artifact envelope digest:
- `accepted_artifact_digest()` computes BLAKE3 over the full envelope including `accepted_at_seq`.
- `verify_accepted_artifact()` recomputes and compares; forged seq values fail verification.
- The seq is assigned before `bind_artifact_digest()` is called.

## Policy Compatibility

All three policies receive nonzero acceptance sequences:
- **Relaxed**: No gate validation, no SyncAll, but still gets sequence.
- **Journaled**: Gate validation, no SyncAll, gets sequence.
- **Strict**: Gate validation + SyncAll + readback, gets sequence.

Sequence allocation is a journal property, not a durability policy property.

## Freshness Check Semantics

`admit_artifact_run_with_certificate_floor(store, policy, run_id, digest, caps, floor)`:
- Compares `artifact.accepted_at_seq < required_at_least`.
- Now functional: artifacts accepted before floor are rejected.
- Backward compatibility: Old artifacts with `accepted_at_seq=0` pass only with floor=0.
- No-op/rejected prototype behavior: The old `submit_artifact_at_seq` variants (caller-passed seq) are NOT part of this contract; the prototype's dual API was rejected as unnecessary complexity.

## Concurrency and Atomicity

- `accept_seq_allocate()` uses the journal's existing `write_lock` (Mutex<()>).
- All journal writes (events, artifacts, snapshots, counter) are serialized.
- No two callers can observe the same counter value.
- No lockback on failure; sequence numbers may be skipped (acceptable).

## Overflow Handling

- Counter uses `checked_add(1)`; overflow returns `Err(JournalError::SequenceOverflow)`.
- No wrapping, no silent reuse.
- New `JournalError` variant: `SequenceOverflow`.

## Implementation Footprint

Modified files (bounded):
1. `crates/vb_storage/src/constants.rs` — add `KEYSPACE_SYSTEM`, `PREFIX_SYSTEM`, `SYSTEM_ACCEPT_SEQ_KEY`, `MAGIC_SYSTEM_RECORD`.
2. `crates/vb_storage/src/journal/core.rs` — add `system` keyspace field, `accept_seq_allocate()` method, update `declared_keyspaces()`, update `probe_health()`.
3. `crates/vb_storage/src/admission.rs` — update `submit_artifact()` and `submit_artifact_with_contracts()` to allocate and bind sequence.
4. `crates/vb_storage/src/lib.rs` — no change (submission API already public).

New files: None.
Modified tests: Many existing tests expect `accepted_at_seq=0`; they must be updated to expect nonzero values or use policy-appropriate assertions.

## Existing Behavior Separated

- Zero-seq artifacts stored before this change remain readable with seq=0.
- No migration of existing artifacts required.
- `declared_keyspaces()` count changes from 11 to 12.
- `probe_health()` now probes the `system` keyspace.
