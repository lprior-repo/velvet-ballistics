# Strict Journal Seam Analysis

## Current Code Path (registered_executor.rs)

`Runtime::execute_registered_action` (line 26-40):
```rust
let (ticket, context) = self.registered_context(run, wire_step, grant)?;
if context.mode != RegisteredExecutionMode::Live { ... }
let receipt = adapter.execute(grant, ticket, context.value)?;  // effect happens FIRST
self.enqueue_registered_completion(ticket, context.output, receipt.clone())?;
Ok(receipt)
```
No journal write before `adapter.execute`. Effect happens before any durable record.

## Runtime Journal Architecture

`crates/vb_runtime/src/runtime.rs:189-194` — Runtime owns `journal: SharedRuntimeJournal`:
```rust
pub type SharedRuntimeJournal = Arc<dyn RuntimeJournal>;  // chunk_001.rs:353
```
Runtime does NOT own the FjallJournal handle directly. It has the trait object.

## Sequence Number Ownership

`crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:271-277`:
```rust
pub(crate) fn journal_sequence_for(&self, run: RunId) -> EventSeq {
    self.journal_sequences.get(&run).copied().unwrap_or(EventSeq::ZERO)
}
```
Shard manages per-run sequence in `journal_sequences: HashMap`. Runtime methods
like `execute_registered_action` operate on `&mut self` (the Runtime), not on a
shard. They cannot call `journal_sequence_for` because that's a Shard method.

## Journal Append Methods

StorageRuntimeJournal (chunk_002.rs:434-456):
- `append()` — unsupported, returns `UnsupportedOperation`
- `append_sequenced(event, seq)` — converts to storage event, calls `append_storage_event`
- `append_sequenced_batch(events, start_seq)` — converts all, calls `append_runtime_event_batch`
- `convert_to_storage_event(event, seq)` — exposed conversion method

`append_storage_event` (chunk_002.rs:39-46):
```rust
fn append_storage_event(&self, event: &JournalEvent) -> RuntimeResult<()> {
    let result = if self.profile == DurabilityProfile::Strict {
        self.journal.append_strict(event)   // SyncAll
    } else {
        self.journal.append_journaled(event) // no sync
    };
    result.map_err(RuntimeError::from)
}
```

`append_runtime_event_batch` (chunk_002.rs:48-68):
```rust
if profile == DurabilityProfile::Strict {
    return batch.strict().commit();  // SyncAll
}
batch.commit()  // no sync
```

QueuedStorageRuntimeJournal (chunk_003.rs:1-56):
- With Strict profile: `append_sequenced` and `append_sequenced_batch` return `UnsupportedAsyncStrictAck`
- With Journaled profile: enqueues only, no sync, no wait for durable persistence
- Cannot provide strict durability — fundamentally async

## Fjall Storage Layer

`crates/vb_storage/src/journal/append.rs`:
- `append_strict(event)` — validates, pre-checks duplicate, stages in batch, `batch.strict().commit()` = SyncAll. Atomic: never visible without durable.
- `append_journaled(event)` — calls `append_unfsynced`, no sync barrier
- `append_strict_batch(events)` — single batch, single SyncAll

## DecisionRecord Type

`crates/vb_core/src/policy.rs:165`:
```rust
pub struct DecisionRecord {
    pub ticket: ActionTicket,
    pub admission: PinnedAdmissionPolicy,
    pub current_live: CurrentPolicyIdentity,
    pub input_digest: PolicyDigest,
    pub taint: Taint,
    pub outcome: DecisionCode,
    pub layer: PolicyLayer,
    pub emergency_influence: InfluenceState,
    pub compat_influence: InfluenceState,
}
```

LivePolicy exists in `crates/vb_runtime/src/policy.rs:27` with `evaluate()` method
that produces `DecisionRecord`. Currently NOT wired into shard action execution
path — policy evaluation exists as infrastructure but is not called before
`adapter.execute`.

## Storage Event for Decision

`crates/vb_storage/src/events/event.rs:309-318`:
```rust
PolicyDecisionEvent {
    run: RunId,
    seq: EventSeq,
    decision: vb_core::policy::DecisionRecord,
}
```
Storage layer already has this variant. `RuntimeJournalEvent` has no corresponding
variant — would need to add `PolicyDecision { decision: DecisionRecord }` to the
runtime journal event enum and wire conversion in `storage_event()`.

## Errors

`RuntimeError::StorageJournalAppend { source: Arc<vb_storage::JournalError> }` —
for journal append failures. Propagated through `map_err(RuntimeError::from)`.

`RuntimeError::UnsupportedAsyncStrictAck` — returned by QueuedStorageRuntimeJournal
when Strict profile is selected (impossible combination).

## Fault Injection Seam

`crates/vb_storage/src/journal/internal.rs` — `consume_persist_failure_for_test()`
is `#[cfg(test)]` and `pub(crate)`. Not accessible from vb_runtime crate.

Tests in `crates/vb_runtime/src/shard/lifecycle_tests/` use fake journal
implementations that override `append_sequenced`/`append_sequenced_batch` to
return errors, rather than injecting failures into real Fjall storage.

## Missing Seam

No method exists on `RuntimeJournal` trait for "convert to storage event and
return without appending". `StorageRuntimeJournal` has `convert_to_storage_event`
but it's not in the trait interface, so code that holds `SharedRuntimeJournal`
cannot call it.

## Summary of Findings

1. Runtime owns trait object, not Fjall handle
2. StorageRuntimeJournal::Strict provides SyncAll before returning
3. QueuedStorageRuntimeJournal cannot provide strict durability
4. Shard manages sequence numbers; Runtime methods cannot access them
5. LivePolicy::evaluate exists but is not called before action execution
6. Storage layer has PolicyDecisionEvent; runtime layer lacks corresponding event
7. No seam to convert event to storage form without appending
8. Tests use fake journal implementations, not Fjall fault injection
