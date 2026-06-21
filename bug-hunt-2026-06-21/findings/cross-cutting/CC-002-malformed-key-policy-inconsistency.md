# CC-002: Cross-module inconsistency on how to handle malformed keyspace rows (skip vs fail-closed)

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/trimming/logic.rs:75-77, 215-217` (skip silently); `crates/vb_storage/src/trimming/logic.rs:23-25` (fail-closed); `crates/vb_storage/src/preview.rs:65-68` (skip silently); `crates/vb_storage/src/headers.rs:53-62`, `artifacts.rs:19-26` (no length check at all)
- **Confidence**: confirmed

## Description

The storage layer has four distinct policies for encountering a malformed keyspace row, with no central definition of which applies where:

1. `trim_events_for_run` and `count_trimmable_events` `continue` silently on `key.len() < 17`.
2. `latest_durable_snapshot_seq` returns `Err(TrimError::IncompleteTrim)` on `key.len() < 17`.
3. `preview_keyspace` `continue`s silently on any `decode_storage_key` error.
4. `run_headers` and `list_artifacts` perform no length or shape check on keys at all — they assume well-formedness and `decode_record` only on the value.

The same keyspace can produce different operator-visible behavior depending on which API is called.

## Evidence

```rust
// crates/vb_storage/src/trimming/logic.rs:73-77 -- skip silently
for item in self.events.prefix(prefix_key) {
    let key = item.key().map_err(TrimError::from)?;
    if key.len() < 17 { continue; }
    ...
}

// crates/vb_storage/src/trimming/logic.rs:21-25 -- fail closed
for item in self.run_snapshot.prefix(prefix_key) {
    let (key, value) = item.into_inner().map_err(TrimError::from)?;
    if key.len() < 17 {
        return Err(TrimError::IncompleteTrim { deleted_count: 0 });
    }
    ...
}

// crates/vb_storage/src/preview.rs:62-68 -- skip silently on any decode error
for (key_bytes, value_bytes) in entries {
    let key = match decode_storage_key(key_bytes) {
        Ok(k) => k,
        Err(_) => continue,
    };
    ...
}

// crates/vb_storage/src/headers.rs:49-63 -- no key check at all
pub fn run_headers(&self) -> Result<Vec<RunHeaderRecord>, JournalError> {
    let mut headers = Vec::new();
    let prefix = [PREFIX_RUN_HEADER];
    for item in self.run_header.prefix(prefix) {
        let value = item.value()?;
        let (_, header) = decode_record(value.as_ref(), MAGIC_INDEX_RECORD, ...)?;
        headers.push(header);
    }
    Ok(headers)
}
```

## Adversarial Check

One could argue each caller has different requirements: trim should be self-healing, snapshot scans must be strict, preview is best-effort, listing is "everything that decodes". But the absence of a documented contract means a maintainer cannot predict what will happen when corruption appears under any specific API. The trim-vs-snapshot inconsistency within the same file is particularly jarring: both scan a keyspace and decode values, but one tolerates short keys and the other does not. Operator dashboards built on these APIs will give contradictory reports about the same underlying corruption.

## Suggested Fix

Pick one policy and apply it uniformly. Recommended: define a `KeyspaceScanPolicy` enum (`{SkipMalformed, FailClosed}`) and thread it through scan APIs, with a default of `FailClosed` for production paths and `SkipMalformed` for diagnostic/doctor paths. Alternatively, document the per-function policy in the module-level doc of each file and add a cross-reference.
