# RE-009: `WaitResolved` journal event maps to `RetryScheduledEvent`, mis-attributing waits as retries

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/journal/chunk_002.rs:209-216`
- **Confidence**: likely

## Description

`StorageRuntimeJournal::boundary_storage_event` maps `RuntimeJournalEvent::WaitResolved { run, step }` to `JournalEvent::RetryScheduledEvent { run, seq, step, attempt: 1 }`. There is no `WaitResolvedEvent` variant in `vb_storage::JournalEvent`, but `RetryScheduledEvent` is a different semantic concept (a retry was scheduled, not a wait was resolved). The mismatch corrupts downstream analytics and lifecycle state derivation, both of which have separate counters for waits versus retries.

## Evidence

`crates/vb_runtime/src/journal/chunk_002.rs:209-216`:

```rust
RuntimeJournalEvent::WaitResolved { run, step } => {
    Ok(Some(JournalEvent::RetryScheduledEvent {
        run,
        seq,
        step,
        attempt: 1,
    }))
}
```

Downstream consumers in `vb_storage`:

- `crates/vb_storage/src/journal/incident/lifecycle.rs:57-60` classifies events into lifecycle states: `WaitScheduledEvent → Waiting`, `AskAnsweredEvent → Waiting`, `RetryScheduledEvent → Active`. So a resolved wait incorrectly marks the run as `Active` (which it should — wait is over), but with the wrong cause label.

- `crates/vb_storage/src/journal/model/analysis.rs:78-81` increments separate counters:
  ```
  WaitScheduledEvent → self.waits_scheduled
  AskAnsweredEvent   → self.asks_answered
  RetryScheduledEvent → self.retries_scheduled
  ```
  Every wait resolution is counted as a retry. Dashboards that report retry rate will over-report; dashboards that report wait-resolution latency will see zero.

- `crates/vb_storage/src/journal/model/checkpoint.rs:24-27` treats `RetryScheduledEvent` as a checkpoint-eligible event (it returns `Some(step.get())`). Wait resolutions also become checkpoint-able, which may or may not be intended — but it is intended via a misnomer, not via the right variant.

## Adversarial Check

1. *"There is no `WaitResolvedEvent` in `JournalEvent`, so reusing `RetryScheduledEvent` is the least-bad option."* — Then `JournalEvent` is missing a variant and should grow one. The current mapping silently corrupts analytics for any operator who relies on `retries_scheduled` as a metric. This is exactly the kind of "fastest path was a misnomer" trap that should be rejected at review.
2. *"Both states are `Active` after the event, so the lifecycle is fine."* — The lifecycle *state* is fine, but the *cause* is wrong, and analytics counters are wrong. Replays that differentiate waits from retries cannot reconstruct the original intent.
3. *"WaitResolved is rare."* — It fires on every timer resolution for every `WaitUntil` and `WaitEvent` primitive in every run. For event-driven workflows this is one of the most common events in the journal.

Severity Medium: the journal is the system of record; mis-attributing event kinds in the durable log is a real data-quality bug that no downstream consumer can defend against.

## Suggested Fix

Either:

(a) Add `JournalEvent::WaitResolvedEvent { run, seq, step, attempt }` to `vb_storage::events::variant` and update lifecycle/analysis/checkpoint to handle it correctly. Then map `WaitResolved → WaitResolvedEvent` in `boundary_storage_event`.

(b) If the storage layer must stay wire-compatible, document the misattribution in the variant's doc-comment and add a sibling `wait_resolutions` counter to `analysis.rs` so dashboards can subtract them out: `actual_retries = retries_scheduled - wait_resolutions`.

Option (a) is correct. Option (b) is a workaround if (a) is deferred.
