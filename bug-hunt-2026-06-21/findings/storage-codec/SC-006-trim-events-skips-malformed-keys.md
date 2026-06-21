# SC-006: `trim_events_for_run` silently skips malformed keys

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/trimming/logic.rs:73-90`
- **Confidence**: confirmed

## Description

When scanning event keys for trimming, the loop uses `if key.len() < 17 { continue; }` to skip keys that are too short. Any keyspace corruption (truncated key, partial write, cross-keyspace bleed) is silently ignored and the events beneath such keys become permanently un-trimmable, while the operator sees no diagnostic.

## Evidence

```rust
// crates/vb_storage/src/trimming/logic.rs:73-90
for item in self.events.prefix(prefix_key) {
    let key = item.key().map_err(TrimError::from)?;
    if key.len() < 17 {
        continue;                                       // <-- silent skip
    }
    let slice = key.get(9..17).ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
    ...
}
```

The same pattern recurs in `count_trimmable_events` (`crates/vb_storage/src/trimming/logic.rs:213-229`) and the journal scan inside `has_terminal_event` is permissive in a similar spirit. By contrast, `latest_durable_snapshot_seq` returns `Err(IncompleteTrim)` on the same condition (line 23-25), so the two scans disagree about whether short keys are fatal.

## Adversarial Check

A short key under the run-event prefix should be impossible per the encoding contract (`run_event_key` always emits 17 bytes). If one exists, it is by definition corruption. Silently skipping it during trim leaves the corrupt row in place forever; the next `latest_durable_snapshot_seq` over the same keyspace family may also see it (depending on prefix) and return Err, producing inconsistent behavior between adjacent trim operations. The inconsistency itself is a defect: the codebase should pick one policy (skip-with-counter or fail-closed) and apply it everywhere.

## Suggested Fix

Either fail closed with `TrimError::IncompleteTrim` (matching `latest_durable_snapshot_seq`), or increment a `skipped_malformed: u64` counter returned in the `TrimmedRunResult` so operators can see corruption without aborting the trim.
